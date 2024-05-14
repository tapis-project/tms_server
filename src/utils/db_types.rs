// This file contains the TMS database structs and related definitions.
#![forbid(unsafe_code)]

use serde::Deserialize;

// ---------------------------------------------------------------------------
// pubkeys:
// ---------------------------------------------------------------------------
#[derive(Debug, Deserialize)]
pub struct Pubkey {
    pub id: i32,
    pub tenant: String,
    pub client_user_id: String,
    pub host: String,
    pub host_account: String,
    pub public_key_fingerprint: String, 
    pub public_key: String,
    pub key_type: String,
    pub key_bits: i32,
    pub max_uses: i32,
    pub remaining_uses: i32,
    pub initial_ttl_minutes: i32,
    pub expires_at: String,
    pub created: String,
    pub updated: String,
}

#[derive(Debug, Deserialize)]
pub struct PubkeyInput {
    pub tenant: String,
    pub client_id: String,
    pub client_user_id: String,
    pub host: String,
    pub host_account: String,
    pub public_key_fingerprint: String, 
    pub public_key: String,
    pub key_type: String,
    pub key_bits: i32,
    pub max_uses: i32,
    pub remaining_uses: i32,
    pub initial_ttl_minutes: i32,
    pub expires_at: String,
    pub created: String,
    pub updated: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct PubkeyRetrieval {
    pub public_key: String,
    pub remaining_uses: i32,
    pub expires_at: String,
}

impl Pubkey {
    #[allow(dead_code, clippy::too_many_arguments)]
    pub fn new(
        id: i32,
        tenant: String,
        client_user_id: String,
        host: String,
        host_account: String,
        public_key_fingerprint: String,
        public_key: String,
        key_type: String,
        key_bits: i32,
        max_uses: i32,
        remaining_uses: i32,
        initial_ttl_minutes: i32,
        expires_at: String,
        created: String,
        updated: String,
    ) 
    -> Pubkey {
        Pubkey {
            id, tenant, client_user_id, host, host_account, public_key_fingerprint, public_key, key_type, key_bits, max_uses, 
            remaining_uses, initial_ttl_minutes, expires_at, created, updated
        }
    }
}

impl PubkeyInput {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        tenant: String,
        client_id: String,
        client_user_id: String,
        host: String,
        host_account: String,
        public_key_fingerprint: String,
        public_key: String,
        key_type: String,
        key_bits: i32,
        max_uses: i32,
        remaining_uses: i32,
        initial_ttl_minutes: i32,
        expires_at: String,
        created: String,
        updated: String,
    ) 
    -> PubkeyInput {
        PubkeyInput {
            tenant, client_id, client_user_id, host, host_account, public_key_fingerprint, public_key, 
            key_type, key_bits, max_uses, remaining_uses, initial_ttl_minutes, expires_at, created, updated
        }
    }
}

impl PubkeyRetrieval {
    pub fn new(
        public_key: String,
        remaining_uses: i32,
        expires_at: String,
    )
    -> PubkeyRetrieval {
        PubkeyRetrieval {
            public_key, remaining_uses, expires_at,
        }
    }
}

