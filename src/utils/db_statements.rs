// This file contains all SQL statements issued by TMS.
#![forbid(unsafe_code)]

pub const PLACEHOLDER: &str = "${PLACEHOLDER}";


// ========================= identity_providers table =========================

pub const INSERT_IDP: &str = concat!(
"INSERT INTO identity_providers ",
  "(id, name, client_id, client_secret, identity_redirect_url, oauth2_token_url, provider_type,",
  " supports_login, supports_resources, created, updated) ",
  "VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
);

// TODO/TBD Will tms_server ever need this?
// pub const GET_IDP: &str = concat!(
// "SELECT id, name, client_id, secret, enabled, created, updated ",
// "FROM clients WHERE client_id = $1",
// );


pub const SEL_IDP_EXISTS: &str = concat!(
"SELECT EXISTS(SELECT 1 FROM identity_providers WHERE id = $1)"
);

// ========================= tms_identities table ========================
// NOTE: We just need one in this table for referencing, so OK if it already exists.
pub const INSERT_TMS_IDENTITY: &str = concat!(
"INSERT INTO tms_identities (tms_identity) VALUES ($1) ON CONFLICT DO NOTHING"
);


// ========================= clients table =========================
pub const INSERT_CLIENT: &str = concat!(
    "INSERT INTO clients (name, client_id, secret, enabled, created, updated) ",
    "VALUES ($1, $2, $3, $4, $5, $6)",
);

pub const GET_CLIENT: &str = concat!(
    "SELECT id, name, client_id, secret, enabled, created, updated ",
    "FROM clients WHERE client_id = $1",
);

pub const SEL_CLIENT_EXISTS: &str = concat!(
"SELECT EXISTS(SELECT 1 FROM clients WHERE client_id = $1)"
);

// Secret elided.
pub const LIST_CLIENTS_TEMPLATE: &str = concat!(
    "SELECT id, name, client_id, enabled, created, updated ",
    "FROM clients ${PLACEHOLDER} ORDER BY client_id",
);

// Conforms to the signature required for secret retrieval queries as defined by 
// get_authz_secret() in authz.rs.
pub const GET_CLIENT_SECRET: &str = concat!(
    "SELECT secret FROM clients WHERE client_id = $1",
);

pub const UPDATE_CLIENT_ENABLED: &str = concat!(
    "UPDATE clients SET enabled = $1, updated = $2 WHERE client_id = $3"
);

pub const UPDATE_CLIENT_SECRET: &str = concat!(
    "UPDATE clients SET secret = $1, updated = $2 WHERE client_id = $3"
);

pub const DELETE_CLIENT: &str = concat!(
    "DELETE FROM clients WHERE client_id = $1"
);

// ========================= resource_provider_logins table ========================
pub const INSERT_RP_LOGIN: &str = concat!(
    "INSERT INTO resource_provider_logins ",
    "(tms_identity, rp_id, rp_account, enabled, created, updated, last_login) ",
    "VALUES ($1, $2, $3, $4, $5, $6, $7)",
);

pub const INSERT_RP_LOGIN_NOT_STRICT: &str = concat!(
    "INSERT INTO resource_provider_logins (tms_identity, rp_id, rp_account, enabled, created, updated, last_login) ",
    "VALUES ($1, $2, $3, $4, $5, $7) ON CONFLICT DO NOTHING",
);

pub const GET_RP_LOGIN: &str = concat!(
    "SELECT id, tms_identity, rp_id, rp_account, enabled, created, updated ",
    "FROM resource_provider_logins WHERE tms_identity = $1 and rp_id = $2 and rp_account = $3"
);

pub const GET_RP_LOGIN_ACTIVE: &str = concat!(
    "SELECT enabled ",
    "FROM resource_provider_logins WHERE tms_identity = $1 AND rp_id = $2 AND rp_account = $3"
);

pub const GET_RP_LOGIN_EXISTS: &str = concat!(
    "SELECT 1 FROM resource_provider_logins WHERE tms_identity = $1 AND rp_id = $2 AND rp_account = $3"
);

pub const UPDATE_RP_LOGIN_ENABLED: &str = concat!(
    "UPDATE resource_provider_logins SET enabled = $1, updated = $2 WHERE tms_identity = $3 AND rp_id = $4 AND rp_account = $5"
);

// TODO
pub const DELETE_RP_LOGIN: &str = concat!(
    "DELETE FROM resource_provider_logins WHERE tms_identity = $1 AND rp_id = $2 AND rp_account = $3"
);

// Secret elided.
// TODO
pub const LIST_RP_LOGIN: &str = concat!(
    "SELECT id, tms_identity, enabled, created, updated ",
    "FROM resource_provider_logins ORDER BY tms_identity",
);

// ========================= user_delegations table =================
pub const INSERT_DELEGATIONS: &str = concat!(
    "INSERT INTO delegations (client_id, tms_identity, rp_id, rp_account, expires_at, created, updated) ",
    "VALUES ($1, $2, $3, $4, $5, $6, $7)",
);

pub const INSERT_DELEGATIONS_NOT_STRICT: &str = concat!(
    "INSERT INTO delegations (client_id, tms_identity, rp_id, rp_account, expires_at, created, updated) ",
    "VALUES ($1, $2, $3, $4, $5, $6) ON CONFLICT DO NOTHING",
);

pub const GET_DELEGATION: &str = concat!(
    "SELECT id, client_id, tms_identity, rp_id, rp_account, expires_at, created, updated ",
    "FROM delegations WHERE id = $1"
);

pub const GET_DELEGATION_ACTIVE: &str = concat!(
    "SELECT expires_at ",
    "FROM delegations WHERE client_id = $1 AND tms_identity = $2 AND rp_id = $3 AND rp_account = $4"
);

// TODO
pub const GET_DELEGATION_EXISTS: &str = concat!(
    "SELECT 1 FROM delegations WHERE client_id = $1 AND tms_identity = $2 AND rp_id = $3 AND rp_account = $4"
);

pub const SEL_DELEGATION_EXISTS: &str = concat!(
    "SELECT EXISTS(SELECT 1 FROM delegations WHERE client_id = $1 AND tms_identity = $2 AND rp_id = $3 AND rp_account = $4)"
);

//TODO
pub const LIST_DELEGATIONS: &str = concat!(
    "SELECT id, client_id, tms_identity, rp_id, rp_account, expires_at, created, updated ",
    "FROM delegations ORDER BY client_id, rp_id, rp_account",
);

// TODO
pub const DELETE_DELEGATION: &str = concat!(
    "DELETE FROM delegations WHERE client_id = $1 AND rp_account = $2"
);

// TODO
pub const UPDATE_DELEGATION_EXPIRY: &str = concat!(
    "UPDATE delegations SET expires_at = $1, updated = $2 ",
    "WHERE client_id = $3 AND rp_account = $4",
);

// ========================= pubkeys table =========================
pub const INSERT_PUBKEYS: &str = concat!(
    "INSERT INTO pubkeys (client_id, tms_identity, rp_id, rp_account, host, host_account, ",
      "public_key_fingerprint, public_key, key_type, key_bits, max_uses, remaining_uses, ",
      "initial_ttl_minutes, expires_at, created, updated) ",
    "VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)",
);

pub const SELECT_PUBKEY: &str = concat!(
    "SELECT public_key, remaining_uses, expires_at FROM pubkeys ",
    "WHERE host_account = $1 AND host = $2 AND public_key_fingerprint = $3",
);

pub const SEL_PUBKEY_EXISTS: &str = concat!(
"SELECT EXISTS(SELECT 1 FROM pubkeys WHERE host_account = $1 AND host = $2)"
);

pub const SELECT_PUBKEY_FOR_UPDATE: &str = concat!(
    "SELECT max_uses, remaining_uses FROM pubkeys ",
    "WHERE client_id = $1 AND host = $2 AND public_key_fingerprint = $3",
);

// TODO
pub const SELECT_PUBKEY_HOST_ACCOUNT: &str = concat!(
    "SELECT host_account FROM pubkeys ",
    "WHERE client_id = $1 AND host = $2 AND public_key_fingerprint = $3",
);

// TODO
pub const SELECT_PUBKEY_RESERVATION_INFO: &str = concat!(
    "SELECT remaining_uses, expires_at, host_account FROM pubkeys ",
    "WHERE client_id = $1 AND host = $2 AND public_key_fingerprint = $3",
);

pub const GET_PUBKEY_TEMPLATE: &str = concat!(
    "SELECT id, client_id, tms_identity, rp_id, rp_account, host, host_account, ",
      "public_key_fingerprint, public_key, key_type, key_bits, max_uses, remaining_uses, ",
      "initial_ttl_minutes, expires_at, created, updated ",
    " FROM pubkeys WHERE id = $1 ${PLACEHOLDER}",
);

// TODO
pub const LIST_PUBKEYS_TEMPLATE: &str = concat!(
    "SELECT id, client_id, rp_id, rp_account, host, host_account, public_key_fingerprint, public_key, ",
    "key_type, key_bits, max_uses, remaining_uses, initial_ttl_minutes, expires_at, created, updated ",
    "FROM pubkeys ${PLACEHOLDER} ORDER BY rp_id, rp_account, host, host_account",
);

pub const UPDATE_MAX_USES: &str = concat!(
    "UPDATE pubkeys SET max_uses = $1, remaining_uses = $2, updated = $3 ",
    "WHERE client_id = $4 AND host = $5 AND public_key_fingerprint = $6",
);

pub const UPDATE_EXPIRES_AT: &str = concat!(
    "UPDATE pubkeys SET expires_at = $1, updated = $2 ",
    "WHERE client_id = $3 AND host = $4 AND public_key_fingerprint = $5",
);

pub const DELETE_PUBKEY: &str = concat!(
    "DELETE FROM pubkeys WHERE client_id = $1 AND host = $2 AND public_key_fingerprint = $3"
);

// ========================= admin table ===========================
pub const INSERT_ADMIN: &str = concat!(
    "INSERT INTO admin (admin_user, admin_secret, privilege, created, updated) ",
    "VALUES ($1, $2, $3, $4, $5)",
);

// Conforms to the signature required for secret retrieval queries as defined by 
// get_authz_secret() in authz.rs.
pub const GET_ADMIN_SECRET: &str = concat!(
    "SELECT admin_secret FROM admin WHERE admin_user = $1",
);

// ========================= hosts table ===========================
// TODO
pub const INSERT_HOSTS: &str = concat!(
    "INSERT INTO hosts (host, addr, created, updated) ",
    "VALUES ($1, $2, $3, $4)",
);

// TODO
pub const GET_HOST: &str = concat!(
    "SELECT id, host, addr, created, updated ",
    "FROM hosts WHERE id = $1"
);

// TODO
pub const DELETE_HOST: &str = concat!(
    "DELETE FROM hosts WHERE host = $1 AND addr = $2"
);

pub const LIST_HOSTS: &str = concat!(
    "SELECT id, host, addr, created, updated ",
    "FROM hosts ORDER BY host, addr",
);

// ==================== reservations table =========================
// TODO
pub const INSERT_RESERVATIONS: &str = concat!(
    "INSERT INTO reservations (resid, parent_resid, client_id, rp_id, rp_account, ",
    "host, public_key_fingerprint, expires_at, created, updated) ",
    "VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
);

// TODO
pub const GET_RESERVATION: &str = concat!(
    "SELECT id, resid, parent_resid, client_id, rp_id, rp_account, host, ",
    "public_key_fingerprint, expires_at, created, updated ",
    "FROM reservations WHERE resid = $1",
);

// TODO
pub const GET_RESERVATION_FOR_EXTEND: &str = concat!(
    "SELECT parent_resid, expires_at FROM reservations ", 
    "WHERE resid = $1 AND client_id = $2",
);

// TODO
pub const DELETE_RESERVATION: &str = concat!(
    "DELETE FROM reservations WHERE resid = $1 AND client_id = $2"
);

// TODO
pub const DELETE_RELATED_RESERVATIONS: &str = concat!(
    "DELETE FROM reservations WHERE (resid = $1 OR parent_resid = $2) AND client_id = $3"
);
