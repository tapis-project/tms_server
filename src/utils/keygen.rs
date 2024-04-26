#![forbid(unsafe_code)]

use std::collections::HashMap;
use std::process::Command;
use std::{fs, fmt};

use anyhow::{Result, anyhow};
use log::{info, error};
use serde::Deserialize;
use uuid::Uuid;

use crate::utils::tms_utils::run_command;

use crate::RUNTIME_CTX;

// ***************************************************************************
//                                Constants
// ***************************************************************************
// Constants.
const DEFAULT_KEYGEN_PATH   : &str = "/usr/bin/ssh-keygen";
const DEFAULT_KEY_OUT_PATH  : &str = "/tmp/tms/keygen/";
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
    pub key_output_path: String,
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
            key_output_path: DEFAULT_KEY_OUT_PATH.to_string(),
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
}

impl GeneratedKeyObj {
    pub fn new(private_key: String, public_key: String, public_key_fingerprint: String) -> Self {
        GeneratedKeyObj { private_key, public_key, public_key_fingerprint }
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
    // Get the bit length for this key type.
    let bitlen = *RUNTIME_CTX.parms.config.keygen_config.key_len_map.get(&key_type)
        .expect("Unable to determine bit length for key type");

    // Get a unique file name for this key.
    let key_name = Uuid::new_v4().as_hyphenated().to_string();

    // Construct the private key file name.
    let mut key_output_path = RUNTIME_CTX.parms.config.keygen_config.key_output_path.clone();
    if !key_output_path.ends_with("/") {
        key_output_path += "/";
    }
    key_output_path += key_name.as_str();

    // Build the ssh-keygen command.
    let mut keyscmd = Command::new(&RUNTIME_CTX.parms.config.keygen_config.keygen_path);
    keyscmd.arg("-t").arg(key_type.to_string());
    if bitlen > 0 {
        keyscmd.arg("-b").arg(bitlen.to_string());
    }
    keyscmd.arg("-f").arg(&key_output_path).arg("-q").arg("-N").arg("");

    // Issue the keygen command which create the key pair files.
    // We return from here on error, no clean up necessary.
    run_command(keyscmd, "keygen-createkeys")?;

    // -------------------------- Generate New Keys --------------------------
    // Create fingerprint.
    let mut fpcmd = Command::new(&RUNTIME_CTX.parms.config.keygen_config.keygen_path);
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

    // -------------------------- Generate Fingerprint -----------------------
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
    // Delete the key files in a reasonably secure way.
    if !shred_keys(&key_output_path, &pub_key_output_path) {
        return Result::Err(anyhow!("**** Key file shred error ****"))
    }

    // Return a newly populated key object.
    Ok(GeneratedKeyObj::new(prv_key, pub_key, fingerprint.to_string()))
}

// ---------------------------------------------------------------------------
// init_runtime_context:
// ---------------------------------------------------------------------------
/** This function panics if it cannot complete successfully.
 * 
 */
pub fn init_keygen() {
    // Check that all keygen paths start are absolute.
    let keygen_path = &RUNTIME_CTX.parms.config.keygen_config.keygen_path;

    // Test that we can execute the keygen program.

    // Test that we can execute the wipe progam.

    // Create key output path if it doesn't exist.

    // Check that the key output path has 700 permissions.

    // Wipe any files in the key output directory that may 
    // have been left over from a previous run.


}

// ***************************************************************************
//                            Private Functions
// ***************************************************************************
// ---------------------------------------------------------------------------
// shred_keys:
// ---------------------------------------------------------------------------
fn shred_keys(key_output_path: &String, pub_key_output_path: &String) -> bool {
    let mut shredded = shred(&key_output_path);
    shredded &= shred(&pub_key_output_path);
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