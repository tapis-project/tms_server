// This file contains the TMS database structs and related definitions.
#![forbid(unsafe_code)]

use chrono::{DateTime, Utc};
use serde::Deserialize;

// ---------------------------------------------------------------------------
// pubkeys:
// ---------------------------------------------------------------------------
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct Pubkey {
    pub id: i32,
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
    pub expires_at: DateTime<Utc>,
    pub created: DateTime<Utc>,
    pub updated: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct PubkeyInput {
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
    pub expires_at: DateTime<Utc>,
    pub created: DateTime<Utc>,
    pub updated: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub struct PubkeyRetrieval {
    pub public_key: String,
    pub remaining_uses: i32,
    pub expires_at: DateTime<Utc>,
}

impl Pubkey {
    #[allow(dead_code, clippy::too_many_arguments)]
    pub fn new(
        id: i32,
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
        expires_at: DateTime<Utc>,
        created: DateTime<Utc>,
        updated: DateTime<Utc>,
    ) 
    -> Pubkey {
        Pubkey {
            id, client_id, client_user_id, host, host_account, public_key_fingerprint,
            public_key, key_type, key_bits, max_uses, remaining_uses, initial_ttl_minutes, 
            expires_at, created, updated
        }
    }
}

impl PubkeyInput {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
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
        expires_at: DateTime<Utc>,
        created: DateTime<Utc>,
        updated: DateTime<Utc>,
    ) 
    -> PubkeyInput {
        PubkeyInput {
            client_id, client_user_id, host, host_account, public_key_fingerprint, public_key,
            key_type, key_bits, max_uses, remaining_uses, initial_ttl_minutes, expires_at, created, updated
        }
    }
}

impl PubkeyRetrieval {
    pub fn new(
        public_key: String,
        remaining_uses: i32,
        expires_at: DateTime<Utc>,
    )
    -> PubkeyRetrieval {
        PubkeyRetrieval {
            public_key, remaining_uses, expires_at,
        }
    }
}

// ---------------------------------------------------------------------------
// clients:
// ---------------------------------------------------------------------------
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct Client {
    pub id: i32,
    pub app_name: String,
    pub app_version: String,
    pub client_id: String,
    pub client_secret: String,
    pub enabled: bool,
    pub created: DateTime<Utc>,
    pub updated: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct ClientInput {
    pub app_name: String,
    pub app_version: String,
    pub client_id: String,
    pub client_secret: String,
    pub enabled: bool,
    pub created: DateTime<Utc>,
    pub updated: DateTime<Utc>,
}

impl Client {
    #[allow(dead_code, clippy::too_many_arguments)]
    pub fn new(
        id: i32,
        app_name: String,
        app_version: String,
        client_id: String,
        client_secret: String,
        enabled: bool,
        created: DateTime<Utc>,
        updated: DateTime<Utc>,
    ) 
    -> Client {
        Client {
            id, app_name, app_version, client_id, client_secret, enabled, created, updated
        }
    }
}

impl ClientInput {
        #[allow(dead_code, clippy::too_many_arguments)]
        pub fn new(
            app_name: String,
            app_version: String,
            client_id: String,
            client_secret: String,
            enabled: bool,
            created: DateTime<Utc>,
            updated: DateTime<Utc>,
        ) 
        -> ClientInput {
            ClientInput {
                app_name, app_version, client_id, client_secret, enabled, created, updated
            }
        }
}

// ---------------------------------------------------------------------------
// user_mfa:
// ---------------------------------------------------------------------------
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct UserMfa {
    pub id: i32,
    pub tms_user_id: String,
    pub expires_at: DateTime<Utc>,
    pub enabled: bool,
    pub created: DateTime<Utc>,
    pub updated: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct UserMfaInput {
    pub tms_user_id: String,
    pub expires_at: DateTime<Utc>,
    pub enabled: bool,
    pub created: DateTime<Utc>,
    pub updated: DateTime<Utc>,
}

impl UserMfa {
    #[allow(dead_code, clippy::too_many_arguments)]
    pub fn new(
        id: i32,
        tms_user_id: String,
        expires_at: DateTime<Utc>,
        enabled: bool,
        created: DateTime<Utc>,
        updated: DateTime<Utc>,
    ) 
    -> UserMfa {
        UserMfa {
            id, tms_user_id, expires_at, enabled, created, updated
        }
    }
}

impl UserMfaInput {
    #[allow(dead_code, clippy::too_many_arguments)]
    pub fn new(
        tms_user_id: String,
        expires_at: DateTime<Utc>,
        enabled: bool,
        created: DateTime<Utc>,
        updated: DateTime<Utc>,
    ) 
    -> UserMfaInput {
        UserMfaInput {
            tms_user_id, expires_at, enabled, created, updated
        }
    }
}

// ---------------------------------------------------------------------------
// user_host:
// ---------------------------------------------------------------------------
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct UserHost {
    pub id: i32,
    pub tms_user_id: String,
    pub host: String,
    pub host_account: String,
    pub expires_at: DateTime<Utc>,
    pub created: DateTime<Utc>,
    pub updated: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct UserHostInput {
    pub tms_user_id: String,
    pub host: String,
    pub host_account: String,
    pub expires_at: DateTime<Utc>,
    pub created: DateTime<Utc>,
    pub updated: DateTime<Utc>,
}

impl UserHost {
    #[allow(dead_code, clippy::too_many_arguments)]
    pub fn new(
        id: i32,
        tms_user_id: String,
        host: String,
        host_account: String,
        expires_at: DateTime<Utc>,
        created: DateTime<Utc>,
        updated: DateTime<Utc>,
    ) 
    -> UserHost {
        UserHost {
            id, tms_user_id, host, host_account, expires_at, created, updated
        }
    }
}

impl UserHostInput {
    #[allow(dead_code, clippy::too_many_arguments)]
    pub fn new(
        tms_user_id: String,
        host: String,
        host_account: String,
        expires_at: DateTime<Utc>,
        created: DateTime<Utc>,
        updated: DateTime<Utc>,
    ) 
    -> UserHostInput {
        UserHostInput {
            tms_user_id, host, host_account, expires_at, created, updated
        }
    }
}

// ---------------------------------------------------------------------------
// delegation:
// ---------------------------------------------------------------------------
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct Delegation {
    pub id: i32,
    pub client_id: String,
    pub client_user_id: String,
    pub expires_at: DateTime<Utc>,
    pub created: DateTime<Utc>,
    pub updated: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct DelegationInput {
    pub client_id: String,
    pub client_user_id: String,
    pub expires_at: DateTime<Utc>,
    pub created: DateTime<Utc>,
    pub updated: DateTime<Utc>,
}

impl Delegation {
    #[allow(dead_code, clippy::too_many_arguments)]
    pub fn new(
        id: i32,
        client_id: String,
        client_user_id: String,
        expires_at: DateTime<Utc>,
        created: DateTime<Utc>,
        updated: DateTime<Utc>,
    ) 
    -> Delegation {
        Delegation {
            id, client_id, client_user_id, expires_at, created, updated
        }
    }
}

impl DelegationInput {
    #[allow(dead_code, clippy::too_many_arguments)]
    pub fn new(
        client_id: String,
        client_user_id: String,
        expires_at: DateTime<Utc>,
        created: DateTime<Utc>,
        updated: DateTime<Utc>,
    ) 
    -> DelegationInput {
        DelegationInput {
            client_id, client_user_id, expires_at, created, updated
        }
    }
}

// ---------------------------------------------------------------------------
// hosts:
// ---------------------------------------------------------------------------
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct Host {
    pub id: i32,
    pub host: String,
    pub addr: String,
    pub created: DateTime<Utc>,
    pub updated: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct HostInput {
    pub host: String,
    pub addr: String,
    pub created: DateTime<Utc>,
    pub updated: DateTime<Utc>,
}

impl Host {
    #[allow(dead_code, clippy::too_many_arguments)]
    pub fn new(
        id: i32,
        host: String,
        addr: String,
        created: DateTime<Utc>,
        updated: DateTime<Utc>,
    ) 
    -> Host {
        Host {
            id, host, addr, created, updated
        }
    }
}

impl HostInput {
    #[allow(dead_code, clippy::too_many_arguments)]
    pub fn new(
        host: String,
        addr: String,
        created: DateTime<Utc>,
        updated: DateTime<Utc>,
    ) 
    -> HostInput {
        HostInput {
            host, addr, created, updated
        }
    }
}

// ---------------------------------------------------------------------------
// Reservations:
// ---------------------------------------------------------------------------
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct Reservation {
    pub id: i32,
    pub resid: String,
    pub parent_resid: String,
    pub client_id: String,
    pub client_user_id: String,
    pub host: String,
    pub public_key_fingerprint: String, 
    pub expires_at: DateTime<Utc>,
    pub created: DateTime<Utc>,
    pub updated: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct ReservationInput {
    pub resid: String,
    pub parent_resid: String,
    pub client_id: String,
    pub client_user_id: String,
    pub host: String,
    pub public_key_fingerprint: String, 
    pub expires_at: DateTime<Utc>,
    pub created: DateTime<Utc>,
    pub updated: DateTime<Utc>,
}

impl Reservation {
    #[allow(dead_code, clippy::too_many_arguments)]
    pub fn new(
        id: i32,
        resid: String,
        parent_resid: String,
        client_id: String,
        client_user_id: String,
        host: String,
        public_key_fingerprint: String, 
        expires_at: DateTime<Utc>,
        created: DateTime<Utc>,
        updated: DateTime<Utc>,
    ) 
    -> Reservation {
        Reservation {
            id, resid, parent_resid, client_id, client_user_id, host,
            public_key_fingerprint, expires_at, created, updated
        }
    }
}

impl ReservationInput {
    #[allow(dead_code, clippy::too_many_arguments)]
    pub fn new(
        resid: String,
        parent_resid: String,
        client_id: String,
        client_user_id: String,
        host: String,
        public_key_fingerprint: String, 
        expires_at: DateTime<Utc>,
        created: DateTime<Utc>,
        updated: DateTime<Utc>,
    ) 
    -> ReservationInput {
        ReservationInput {
            resid, parent_resid, client_id, client_user_id, host,
            public_key_fingerprint, expires_at, created, updated
        }
    }
}

