#![forbid(unsafe_code)]

use std::collections::HashMap;

use anyhow::{Result, anyhow};
use log::{info, error};
use serde::Deserialize;

use crate::RUNTIME_CTX;

// ***************************************************************************
//                                Constants
// ***************************************************************************
// Constants.
const DEFAULT_KEYGEN_PATH  : &str = "/usr/bin/ssh-keygen";
const DEFAULT_KEY_OUT_PATH : &str = "/tmp/tms/keygen";
const DEFAULT_WIPE_PATH    : &str = "/usr/bin/shred";

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
    pub wipe_path: String,
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
            wipe_path: DEFAULT_WIPE_PATH.to_string(),
            key_len_map: get_key_len_map(),
        }
    }
}

// ---------------------------------------------------------------------------
// GeneratedKeyObj:
// ---------------------------------------------------------------------------
pub struct GeneratedKeyObj {
    private_key: String,
    public_key: String,
    public_key_fingerprint: String,
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

    // Get the bit length for this key type.

    // Get a unique file name for this key.

    // Build the ssh-keygen command.

    // Issue the command.




    // Temp
    Ok(GeneratedKeyObj::new("PRIVATE".to_string(), 
                            "PUBLIC".to_string(), 
                            "FINGERPRINT".to_string()))
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