// This file contains the TMS database structs and related definitions.
#![forbid(unsafe_code)]

use serde::Deserialize;

// ---------------------------------------------------------------------------
// pubkeys:
// ---------------------------------------------------------------------------
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct Pubkey {
    pub id: i64,
    pub tenant: String,
    pub client_id: String,
    pub client_user_id: String,
    pub host: String,
    pub host_account: String,
    pub public_key_fingerprint: String, 
    pub public_key: String,
    pub key_type: String,
    pub key_bits: i64,
    pub max_uses: i64,
    pub remaining_uses: i64,
    pub initial_ttl_minutes: i64,
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
    pub key_bits: i64,
    pub max_uses: i64,
    pub remaining_uses: i64,
    pub initial_ttl_minutes: i64,
    pub expires_at: String,
    pub created: String,
    pub updated: String,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub struct PubkeyRetrieval {
    pub public_key: String,
    pub remaining_uses: i64,
    pub expires_at: String,
}

impl Pubkey {
    #[allow(dead_code, clippy::too_many_arguments)]
    pub fn new(
        id: i64,
        tenant: String,
        client_id: String,
        client_user_id: String,
        host: String,
        host_account: String,
        public_key_fingerprint: String,
        public_key: String,
        key_type: String,
        key_bits: i64,
        max_uses: i64,
        remaining_uses: i64,
        initial_ttl_minutes: i64,
        expires_at: String,
        created: String,
        updated: String,
    ) 
    -> Pubkey {
        Pubkey {
            id, tenant, client_id, client_user_id, host, host_account, public_key_fingerprint, 
            public_key, key_type, key_bits, max_uses, remaining_uses, initial_ttl_minutes, 
            expires_at, created, updated
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
        key_bits: i64,
        max_uses: i64,
        remaining_uses: i64,
        initial_ttl_minutes: i64,
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
        remaining_uses: i64,
        expires_at: String,
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
    pub id: i64,
    pub tenant: String,
    pub app_name: String,
    pub app_version: String,
    pub client_id: String,
    pub client_secret: String,
    pub enabled: i64,
    pub created: String,
    pub updated: String,
}

#[derive(Debug, Deserialize)]
pub struct ClientInput {
    pub tenant: String,
    pub app_name: String,
    pub app_version: String,
    pub client_id: String,
    pub client_secret: String,
    pub enabled: i64,
    pub created: String,
    pub updated: String,
}

impl Client {
    pub fn new(
        id: i64,
        tenant: String,
        app_name: String,
        app_version: String,
        client_id: String,
        client_secret: String,
        enabled: i64,
        created: String,
        updated: String,
    ) 
    -> Client {
        Client {
            id, tenant, app_name, app_version, client_id, client_secret, enabled, created, updated
        }
    }
}

impl ClientInput {
        #[allow(dead_code, clippy::too_many_arguments)]
        pub fn new(
            tenant: String,
            app_name: String,
            app_version: String,
            client_id: String,
            client_secret: String,
            enabled: i64,
            created: String,
            updated: String,
        ) 
        -> ClientInput {
            ClientInput {
                tenant, app_name, app_version, client_id, client_secret, enabled, created, updated
            }
        }
}

// ---------------------------------------------------------------------------
// user_mfa:
// ---------------------------------------------------------------------------
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct UserMfa {
    pub id: i64,
    pub tenant: String,
    pub tms_user_id: String,
    pub expires_at: String,
    pub enabled: i64,
    pub created: String,
    pub updated: String,
}

#[derive(Debug, Deserialize)]
pub struct UserMfaInput {
    pub tenant: String,
    pub tms_user_id: String,
    pub expires_at: String,
    pub enabled: i64,
    pub created: String,
    pub updated: String,
}

impl UserMfa {
    #[allow(dead_code, clippy::too_many_arguments)]
    pub fn new(
        id: i64,
        tenant: String,
        tms_user_id: String,
        expires_at: String,
        enabled: i64,
        created: String,
        updated: String,
    ) 
    -> UserMfa {
        UserMfa {
            id, tenant, tms_user_id, expires_at, enabled, created, updated
        }
    }
}

impl UserMfaInput {
    #[allow(dead_code, clippy::too_many_arguments)]
    pub fn new(
        tenant: String,
        tms_user_id: String,
        expires_at: String,
        enabled: i64,
        created: String,
        updated: String,
    ) 
    -> UserMfaInput {
        UserMfaInput {
            tenant, tms_user_id, expires_at, enabled, created, updated
        }
    }
}

// ---------------------------------------------------------------------------
// user_host:
// ---------------------------------------------------------------------------
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct UserHost {
    pub id: i64,
    pub tenant: String,
    pub tms_user_id: String,
    pub host: String,
    pub host_account: String,
    pub expires_at: String,
    pub created: String,
    pub updated: String,
}

#[derive(Debug, Deserialize)]
pub struct UserHostInput {
    pub tenant: String,
    pub tms_user_id: String,
    pub host: String,
    pub host_account: String,
    pub expires_at: String,
    pub created: String,
    pub updated: String,
}

impl UserHost {
    #[allow(dead_code, clippy::too_many_arguments)]
    pub fn new(
        id: i64,
        tenant: String,
        tms_user_id: String,
        host: String,
        host_account: String,
        expires_at: String,
        created: String,
        updated: String,
    ) 
    -> UserHost {
        UserHost {
            id, tenant, tms_user_id, host, host_account, expires_at, created, updated
        }
    }
}

impl UserHostInput {
    #[allow(dead_code, clippy::too_many_arguments)]
    pub fn new(
        tenant: String,
        tms_user_id: String,
        host: String,
        host_account: String,
        expires_at: String,
        created: String,
        updated: String,
    ) 
    -> UserHostInput {
        UserHostInput {
            tenant, tms_user_id, host, host_account, expires_at, created, updated
        }
    }
}

// ---------------------------------------------------------------------------
// delegation:
// ---------------------------------------------------------------------------
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct Delegation {
    pub id: i64,
    pub tenant: String,
    pub client_id: String,
    pub client_user_id: String,
    pub expires_at: String,
    pub created: String,
    pub updated: String,
}

#[derive(Debug, Deserialize)]
pub struct DelegationInput {
    pub tenant: String,
    pub client_id: String,
    pub client_user_id: String,
    pub expires_at: String,
    pub created: String,
    pub updated: String,
}

impl Delegation {
    #[allow(dead_code, clippy::too_many_arguments)]
    pub fn new(
        id: i64,
        tenant: String,
        client_id: String,
        client_user_id: String,
        expires_at: String,
        created: String,
        updated: String,
    ) 
    -> Delegation {
        Delegation {
            id, tenant, client_id, client_user_id, expires_at, created, updated
        }
    }
}

impl DelegationInput {
    #[allow(dead_code, clippy::too_many_arguments)]
    pub fn new(
        tenant: String,
        client_id: String,
        client_user_id: String,
        expires_at: String,
        created: String,
        updated: String,
    ) 
    -> DelegationInput {
        DelegationInput {
            tenant, client_id, client_user_id, expires_at, created, updated
        }
    }
}

// ---------------------------------------------------------------------------
// tenants:
// ---------------------------------------------------------------------------
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct Tenant {
    pub id: i64,
    pub tenant: String,
    pub enabled: i64,
    pub created: String,
    pub updated: String,
}

#[derive(Debug, Deserialize)]
pub struct TenantInput {
    pub tenant: String,
    pub enabled: i64,
    pub key_hash: String,
    pub created: String,
    pub updated: String,
}

impl Tenant {
    #[allow(dead_code, clippy::too_many_arguments)]
    pub fn new(
        id: i64,
        tenant: String,
        enabled: i64,
        created: String,
        updated: String,
    ) 
    -> Tenant {
        Tenant {
            id, tenant, enabled, created, updated
        }
    }
}

impl TenantInput {
    #[allow(dead_code, clippy::too_many_arguments)]
    pub fn new(
        tenant: String,
        enabled: i64,
        key_hash: String,
        created: String,
        updated: String,
    ) 
    -> TenantInput {
        TenantInput {
            tenant, enabled, key_hash, created, updated
        }
    }
}

// ---------------------------------------------------------------------------
// hosts:
// ---------------------------------------------------------------------------
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct Host {
    pub id: i64,
    pub tenant: String,
    pub host: String,
    pub addr: String,
    pub created: String,
    pub updated: String,
}

#[derive(Debug, Deserialize)]
pub struct HostInput {
    pub tenant: String,
    pub host: String,
    pub addr: String,
    pub created: String,
    pub updated: String,
}

impl Host {
    #[allow(dead_code, clippy::too_many_arguments)]
    pub fn new(
        id: i64,
        tenant: String,
        host: String,
        addr: String,
        created: String,
        updated: String,
    ) 
    -> Host {
        Host {
            id, tenant, host, addr, created, updated
        }
    }
}

impl HostInput {
    #[allow(dead_code, clippy::too_many_arguments)]
    pub fn new(
        tenant: String,
        host: String,
        addr: String,
        created: String,
        updated: String,
    ) 
    -> HostInput {
        HostInput {
            tenant, host, addr, created, updated
        }
    }
}

// ---------------------------------------------------------------------------
// Reservations:
// ---------------------------------------------------------------------------
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct Reservation {
    pub id: i64,
    pub resid: String,
    pub parent_resid: String,
    pub tenant: String,
    pub client_id: String,
    pub client_user_id: String,
    pub host: String,
    pub public_key_fingerprint: String, 
    pub expires_at: String,
    pub created: String,
    pub updated: String,
}

#[derive(Debug, Deserialize)]
pub struct ReservationInput {
    pub resid: String,
    pub parent_resid: String,
    pub tenant: String,
    pub client_id: String,
    pub client_user_id: String,
    pub host: String,
    pub public_key_fingerprint: String, 
    pub expires_at: String,
    pub created: String,
    pub updated: String,
}

impl Reservation {
    #[allow(dead_code, clippy::too_many_arguments)]
    pub fn new(
        id: i64,
        resid: String,
        parent_resid: String,
        tenant: String,
        client_id: String,
        client_user_id: String,
        host: String,
        public_key_fingerprint: String, 
        expires_at: String,
        created: String,
        updated: String,
    ) 
    -> Reservation {
        Reservation {
            id, resid, parent_resid, tenant, client_id, client_user_id, host, 
            public_key_fingerprint, expires_at, created, updated
        }
    }
}

impl ReservationInput {
    #[allow(dead_code, clippy::too_many_arguments)]
    pub fn new(
        resid: String,
        parent_resid: String,
        tenant: String,
        client_id: String,
        client_user_id: String,
        host: String,
        public_key_fingerprint: String, 
        expires_at: String,
        created: String,
        updated: String,
    ) 
    -> ReservationInput {
        ReservationInput {
            resid, parent_resid, tenant, client_id, client_user_id, host,  
            public_key_fingerprint, expires_at, created, updated
        }
    }
}

