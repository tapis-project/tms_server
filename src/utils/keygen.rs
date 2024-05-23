#![forbid(unsafe_code)]

use core::panic;
use std::collections::HashMap;
use std::os::unix::fs::MetadataExt;
use std::process::Command;
use std::path::Path;
use std::{fs, fmt};

use anyhow::{Result, anyhow};
use log::{info, warn, error};
use serde::Deserialize;
use uuid::Uuid;

use crate::utils::tms_utils::run_command;

use crate::RUNTIME_CTX;

// ***************************************************************************
//                                Constants
// ***************************************************************************
// Constants.
const DEFAULT_KEYGEN_PATH   : &str = "/usr/bin/ssh-keygen";
const DEFAULT_SHREDDER_PATH : &str = "/usr/bin/shred";

// ***************************************************************************
//                                Enums
// ***************************************************************************
#[derive(Debug, Eq, PartialEq, Hash, Deserialize)]
pub enum KeyType { 
    Dsa,
    Ecdsa,
    EcdsaSk,
    Ed25519,
    Ed25519Sk,
    Rsa,
}

// Convert enum to it's string representation.
impl fmt::Display for KeyType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            KeyType::Dsa       => write!(f, "dsa"),
            KeyType::Ecdsa     => write!(f, "ecdsa"),
            KeyType::EcdsaSk   => write!(f, "ecdsa-sk"),
            KeyType::Ed25519   => write!(f, "ed25519"),
            KeyType::Ed25519Sk => write!(f, "ed25519-sk"),
            KeyType::Rsa       => write!(f, "rsa"),
        }
    }
}

// ***************************************************************************
//                            KeygenConfig Structs
// ***************************************************************************
// ---------------------------------------------------------------------------
// KeygenConfig:
// ---------------------------------------------------------------------------
#[derive(Debug, Deserialize)]
pub struct KeygenConfig {
    pub keygen_path: String,
    pub shredder_path: String,
    pub key_len_map: HashMap<KeyType, i32>,
}

impl KeygenConfig {
    #[allow(dead_code)]
    pub fn new() -> Self {
        KeygenConfig::default()
    }
}

impl Default for KeygenConfig {
    fn default() -> Self {
        Self {
            keygen_path: DEFAULT_KEYGEN_PATH.to_string(),
            shredder_path: DEFAULT_SHREDDER_PATH.to_string(),
            key_len_map: get_key_len_map(),
        }
    }
}

// ---------------------------------------------------------------------------
// GeneratedKeyObj:
// ---------------------------------------------------------------------------
#[derive(Debug, Deserialize)]
pub struct GeneratedKeyObj {
    pub private_key: String,
    pub public_key: String,
    pub public_key_fingerprint: String,
    pub key_type: String,
    pub key_bits: i32,
}

impl GeneratedKeyObj {
    pub fn new(private_key: String, public_key: String, public_key_fingerprint: String, 
               key_type: String, key_bits: i32) -> Self {
        GeneratedKeyObj { private_key, public_key, public_key_fingerprint, key_type, key_bits}
    }
}

// ***************************************************************************
//                               Functions
// ***************************************************************************
// ---------------------------------------------------------------------------
// generate_key:
// ---------------------------------------------------------------------------
pub fn generate_key(key_type: KeyType) -> Result<GeneratedKeyObj> {

    // -------------------------- Generate New Keys --------------------------
    // -----------------------------------------------------------------------
    // Convenience.
    let kconfig = &RUNTIME_CTX.parms.config.keygen_config;

    // Get the bit length for this key type. This should never fail.
    let bitlen = *kconfig.key_len_map.get(&key_type)
        .unwrap_or_else(|| panic!("Unable to determine bit length for key type {}.", key_type));

    // Get a unique file name for this key.
    let key_name = Uuid::new_v4().as_hyphenated().to_string();

    // Construct the private key file name.
    let mut key_output_path = RUNTIME_CTX.tms_dirs.keygen_dir.clone();
    if !key_output_path.ends_with('/') {
        key_output_path += "/";
    }
    key_output_path += key_name.as_str();

    // Build the ssh-keygen command.
    let mut keyscmd = Command::new(&kconfig.keygen_path);
    keyscmd.arg("-t").arg(key_type.to_string());
    if bitlen > 0 {
        keyscmd.arg("-b").arg(bitlen.to_string());
    }
    keyscmd.arg("-f").arg(&key_output_path).arg("-q").arg("-N").arg("");

    // Issue the keygen command which create the key pair files.
    // We return from here on error, no clean up necessary.
    run_command(keyscmd, "keygen-createkeys")?;

    // -------------------------- Generate Fingerprint -----------------------
    // -----------------------------------------------------------------------
    // Create fingerprint.
    let mut fpcmd = Command::new(&kconfig.keygen_path);
    let pub_key_output_path = key_output_path.clone() + ".pub";
    fpcmd.arg("-l").arg("-f").arg(&pub_key_output_path);
    let fpcmdout = match run_command(fpcmd, "keygen-fingerprint") {
        Ok(o) => o,
        Err(e)=> {
            let msg = "Unable to generate fingerprint: ".to_string() + &e.to_string(); 
            error!("{}", msg);
            shred_keys(&key_output_path, &pub_key_output_path);
            return Result::Err(anyhow!(msg));
        },
    };

    // Convert fingerprint to a string.
    let fputf8 = match String::from_utf8(fpcmdout.stdout) {
        Ok(s) => s,
        Err(e) => {
            let msg = "Unable to parse fingerprint: ".to_string() + &e.to_string(); 
            error!("{}", msg);
            shred_keys(&key_output_path, &pub_key_output_path);
            return Result::Err(anyhow!(msg));
        },
    };

    // -------------------------- Parse Fingerprint --------------------------
    // -----------------------------------------------------------------------
    // Extract the fingerprint hash from the string with this general format:
    //
    //   4096 SHA256:zIrsbkO7lZ/35472qdoQUFXoir2tcH2D09efHikBZxA bud@host (RSA)
    //
    // We are intested only in the second word.
    let mut iter = fputf8.split_whitespace();
    iter.next(); // Skip length value
    let fingerprint = match iter.next() {
        Some(s) => s,
        None => {
            let msg = "Fingerprint hash value not found."; 
            error!("{}", msg);
            shred_keys(&key_output_path, &pub_key_output_path);
            return Result::Err(anyhow!(msg));
        },
    };
    info!("Generated public key with fingerprint: {}.", fingerprint);

    // -------------------------- Read Key Files -----------------------------
    // -----------------------------------------------------------------------
    // Read the private key file into a string.
    let prv_key = match fs::read_to_string(&key_output_path) {
        Ok(s) => s,
        Err(e) => {
            let msg = "Unable read private key file ".to_string() + 
                                key_output_path.as_str() + ": " + &e.to_string(); 
            error!("{}", msg);
            shred_keys(&key_output_path, &pub_key_output_path);
            return Result::Err(anyhow!(msg));
        },
    };

    // Read the private key file into a string.
    let pub_key = match fs::read_to_string(&pub_key_output_path) {
        Ok(s) => s,
        Err(e) => {
            let msg = "Unable read public key file ".to_string() + 
                                pub_key_output_path.as_str() + ": " + &e.to_string(); 
            error!("{}", msg);
            shred_keys(&key_output_path, &pub_key_output_path);
            return Result::Err(anyhow!(msg));
        },
    };

    // -------------------------- Shred Key Files ----------------------------
    // -----------------------------------------------------------------------
    // Delete the key files in a reasonably secure way.
    if !shred_keys(&key_output_path, &pub_key_output_path) {
        return Result::Err(anyhow!("**** Key file shred error ****"))
    }

    // -------------------------- Package Results ----------------------------
    // -----------------------------------------------------------------------
    // Substitute the a value for fixed length keys that we generate.
    let mut key_bits = bitlen;
    if key_bits == 0 && key_type == KeyType::Ed25519 {
        key_bits = 256;
    }

    // Return a newly populated key object.
    Ok(GeneratedKeyObj::new(prv_key, 
                            pub_key, 
                            fingerprint.to_string(), 
                            key_type.to_string(),
                            key_bits,
                        ))
}

// ---------------------------------------------------------------------------
// init_runtime_context:
// ---------------------------------------------------------------------------
/** One time initialization routine. This function panics if it cannot complete 
 * successfully. 
 */
pub fn init_keygen() {

    // Check that the keygen path is absolute and represents an executable file.
    // Note: The absolute check does not actually touch the file system.
    let kconfig = &RUNTIME_CTX.parms.config.keygen_config;
    let keygen_path_obj = Path::new(&kconfig.keygen_path);
    if !keygen_path_obj.is_absolute() {
        panic!("The keygen program path must be absolute: {}", &kconfig.keygen_path);
    }
    if !keygen_path_obj.is_file() {
        panic!("The keygen program path must be an executable file: {}", &kconfig.keygen_path);
    }
    if !is_executable(keygen_path_obj) {
        panic!("The keygen program file must be executable: {}", &kconfig.keygen_path);
    }

    // Check the shredder path is absolute and represents an executable file. 
    let shredder_path_obj = Path::new(&kconfig.shredder_path);
    if !shredder_path_obj.is_absolute() {
        panic!("The shredder program path must be absolute: {}", &kconfig.shredder_path);
    }
    if !shredder_path_obj.is_file() {
        panic!("The shredder program path must be an executable file: {}", &kconfig.shredder_path);
    }
    if !is_executable(shredder_path_obj) {
        panic!("The shredder program file must be executable: {}", &kconfig.shredder_path);
    }

    // The output directory temperarily stored generated keys files.
    let key_output_path_obj = Path::new(&RUNTIME_CTX.tms_dirs.keygen_dir);
    shred_files_in_dir(key_output_path_obj);

}

// ***************************************************************************
//                            Private Functions
// ***************************************************************************
// ---------------------------------------------------------------------------
// shred_keys:
// ---------------------------------------------------------------------------
fn shred_keys(key_output_path: &String, pub_key_output_path: &String) -> bool {
    let mut shredded = shred(key_output_path);
    shredded &= shred(pub_key_output_path);
    shredded
}

// ---------------------------------------------------------------------------
// shred:
// ---------------------------------------------------------------------------
/** This function shreds the key files when they are no longer needed.  It 
 * returns true if the shredding succeeded, false otherwise.  See shred
 * man page for limitations of shredding on different file systems. 
 * 
 * Errors are logged here.
 */
fn shred(filepath : &String) -> bool {
    // Create the shred command with a number of iterations, 
    // a final zero out iteration and file removal.
    let mut shredcmd = Command::new(&RUNTIME_CTX.parms.config.keygen_config.shredder_path);
    shredcmd.arg("-n").arg("10").arg("-z").arg("-u").arg(filepath);

    // Shred then delete the file.
    let shredded = match run_command(shredcmd, "shred-keys") {
        Ok(_) => true,
        Err(e) => {
            let msg = "Unable unable to shred file ".to_string() + 
                filepath.as_str() + ": " + &e.to_string(); 
            error!("{}", msg);
            false    
        },
    };

    // File shredded.
    shredded
}

// ---------------------------------------------------------------------------
// shred_files_in_dir:
// ---------------------------------------------------------------------------
/** Shred all the files in the key output directory.  Called only on initialization.
 * Done on a best effort basis, try not to abort server if shredding isn't possible. 
 */
fn shred_files_in_dir(key_output_path_obj: &Path) {
    // Get all the files in the directory.
    let mut files_shredded = 0;
    if let Ok(entries) = fs::read_dir(key_output_path_obj) {
        for entry in entries.flatten() {
            let entry_path = entry.path();
                if entry_path.is_file() {
                    // Delete the file.
                    let p = &*entry_path.to_string_lossy();
                    shred(&p.to_string());
                    files_shredded += 1;
                }
        } 
    } else {
        // Unable to read entries in directory.
        let msg = format!("Unable to clean up key output directory ({:?}): ",
                                    key_output_path_obj); 
        warn!("{}", msg);
    }

    // Log activity.
    info!("{} leftover files shredded in directory {:?}.", files_shredded, key_output_path_obj);
}

// ---------------------------------------------------------------------------
// is_executable:
// ---------------------------------------------------------------------------
// Determine whether a path--typically a file--is executable.
fn is_executable(path: &Path) -> bool {
    let meta = path.metadata()
        .unwrap_or_else(|_| panic!("Unable to retrieve metadata for {:?}", path));
    meta.mode() & 0o111 != 0
}

// ---------------------------------------------------------------------------
// get_key_len_map:
// ---------------------------------------------------------------------------
/** One-time initialization routine that defines the bit lengths used 
 * for the various key types. 
 */
fn get_key_len_map() -> HashMap<KeyType, i32> {
    // Create the key type bit length mappings.
    let mut key_len_map = HashMap::new();
    key_len_map.insert(KeyType::Dsa, 1024);    // required length
    key_len_map.insert(KeyType::Ecdsa, 521);   // max allowed
    key_len_map.insert(KeyType::EcdsaSk, 0);   // ssh-keygen ignored
    key_len_map.insert(KeyType::Ed25519, 0);   // ssh-keygen ignored
    key_len_map.insert(KeyType::Ed25519Sk, 0); // ssh-keygen ignored
    key_len_map.insert(KeyType::Rsa, 4096);    // ssh-keygen default is 3072
    key_len_map
}