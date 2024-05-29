#![forbid(unsafe_code)]

use core::panic;
use std::collections::HashMap;
use std::fmt;

use anyhow::{Result, anyhow};
use log::error;
use serde::Deserialize;
use lazy_static::lazy_static;

use ssh_key::{Algorithm, HashAlg, EcdsaCurve, PrivateKey};

// ***************************************************************************
//                                Constants
// ***************************************************************************
// ***************************************************************************
//                             Static Variables
// ***************************************************************************
// The number of bit used to create each key are hardcoded in TMS.
lazy_static! {
    static ref KEY_LEN_MAP: HashMap<KeyType, i32> = get_key_len_map();
}

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
// init_runtime_context:
// ---------------------------------------------------------------------------
/** One time initialization routine. This function panics if it cannot complete 
 * successfully. 
 */
#[allow(unused_variables)]
pub fn init_keygen() {
    // Get the bit length any key type to force map initialization. 
    // This should never fail, but if it does we abort execution.
    let key_type = KeyType::Ed25519;
    let bitlen = *KEY_LEN_MAP.get(&key_type)
        .unwrap_or_else(|| panic!("Unable to determine bit length for key type {}.", key_type));
}

// ---------------------------------------------------------------------------
// generate_key:
// ---------------------------------------------------------------------------
pub fn generate_key(key_type: KeyType) -> Result<GeneratedKeyObj> {

    // --------------------- Generate Key ---------------------
    // --------------------------------------------------------
    // Generate the private key based on the key type.
    let gen_result = match key_type {
        KeyType::Ed25519 => gen_private_key(Algorithm::Ed25519),
        KeyType::Ecdsa   => {
            let curve = EcdsaCurve::NistP521;
            gen_private_key(Algorithm::Ecdsa {curve})
        },
        KeyType::Rsa     => {
            let hash = Some(HashAlg::Sha256);
            gen_private_key(Algorithm::Rsa {hash})
        },
        _ => {Err(anyhow!("Algorithm not supported: {}", key_type.to_string()))},
    };

    // Get the generated private key.
    let prvkey: PrivateKey;
    if let Err(e) = gen_result {
        error!("Key generation failed: {}", e.to_string());
        return Err(e);
    } else {
        // This should never fail.
        prvkey = gen_result.expect("Unexpected failure unwrapping generated private key");
    }

    // ------------------ Create Key Artifacts ----------------
    // --------------------------------------------------------
    // Get the ssh formatted private key.
    let ssh_prvkey = match prvkey.to_openssh(ssh_key::LineEnding::LF) {
        Ok(k) => k.to_string(),
        Err(e) => {
            let msg = format!("Failure to convert private key to SSH format: {}", e);
            error!("{}", msg);
            return Err(anyhow!(msg));
        }
    };

    // Get the ssh formatted public key.
    let ssh_pubkey = match prvkey.public_key().to_openssh() {
        Ok(k) => k,
        Err(e) => {
            let msg = format!("Failure to convert public key to SSH format: {}", e);
            error!("{}", msg);
            return Err(anyhow!(msg));
        }        
    };

    // Get the public key fingerprint.
    let ssh_fp = prvkey.fingerprint(HashAlg::Sha256).to_string(); 

    // // -------------------------- Package Results ----------------------------
    // // -----------------------------------------------------------------------
    // Get the bit length for this key type. This should never fail, 
    // but if it we just return 0.
    let bitlen = *KEY_LEN_MAP.get(&key_type)
        .unwrap_or(&(0_i32));

    // Substitute the a value for fixed length keys that we generate.
    let mut key_bits = bitlen;
    if key_bits == 0 && key_type == KeyType::Ed25519 {
        key_bits = 256;
    }

    // Return a newly populated key object.
    Ok(GeneratedKeyObj::new(ssh_prvkey, 
                            ssh_pubkey, 
                            ssh_fp, 
                            key_type.to_string(),
                            key_bits,
                        ))
}

// ---------------------------------------------------------------------------
// gen_private_key:
// ---------------------------------------------------------------------------
fn gen_private_key(algorithm: Algorithm) -> Result<PrivateKey> {
    
    // Use operating system's random number generator.
    let mut rng = rand::rngs::OsRng;
 
    // Generate the private key.
    let alg = algorithm.clone();
    match PrivateKey::random(&mut rng, alg) {
        Ok(k) => Ok(k),
        Err(e) => {
            let msg = format!("Unable to generate {} key: {}", algorithm.clone(), e);
            Err(anyhow!(msg))
        }
    }
}

// ***************************************************************************
//                            Private Functions
// ***************************************************************************
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