// This file contains all SQL statements issued by TMS.
#![forbid(unsafe_code)]

pub const PLACEHOLDER: &str = "${PLACEHOLDER}";

// ========================= clients table =========================
pub const INSERT_CLIENT: &str = concat!(
    "INSERT INTO clients (app_name, client_id, client_secret, enabled, created, updated) ",
    "VALUES ($1, $2, $3, $4, $5, $6)",
);

pub const GET_CLIENT: &str = concat!(
    "SELECT id, app_name, client_id, client_secret, enabled, created, updated ",
    "FROM clients WHERE client_id = $1",
);

pub const SEL_CLIENT_EXISTS: &str = concat!(
"SELECT EXISTS(SELECT 1 FROM clients WHERE client_id = $1)"
);

// Secret elided.
pub const LIST_CLIENTS_TEMPLATE: &str = concat!(
    "SELECT id, app_name, client_id, enabled, created, updated ",
    "FROM clients ${PLACEHOLDER} ORDER BY client_id",
);

// Conforms to the signature required for secret retrieval queries as defined by 
// get_authz_secret() in authz.rs.
pub const GET_CLIENT_SECRET: &str = concat!(
    "SELECT client_secret FROM clients WHERE client_id = $1",
);

pub const UPDATE_CLIENT_ENABLED: &str = concat!(
    "UPDATE clients SET enabled = $1, updated = $2 WHERE client_id = $3"
);

pub const UPDATE_CLIENT_SECRET: &str = concat!(
    "UPDATE clients SET client_secret = $1, updated = $2 WHERE client_id = $3"
);

pub const DELETE_CLIENT: &str = concat!(
    "DELETE FROM clients WHERE client_id = $1"
);

// ========================= user_mfa table ========================
pub const INSERT_USER_MFA: &str = concat!(
    "INSERT INTO user_mfa (tms_user_id, expires_at, enabled, created, updated) ",
    "VALUES ($1, $2, $3, $4, $5)",
);

pub const INSERT_USER_MFA_NOT_STRICT: &str = concat!(
    "INSERT INTO user_mfa (tms_user_id, expires_at, enabled, created, updated) ",
    "VALUES ($1, $2, $3, $4, $5) ON CONFLICT DO NOTHING",
);

pub const GET_USER_MFA: &str = concat!(
    "SELECT id, tms_user_id, expires_at, enabled, created, updated ",
    "FROM user_mfa WHERE tms_user_id = $1"
);

pub const GET_USER_MFA_ACTIVE: &str = concat!(
    "SELECT expires_at, enabled ",
    "FROM user_mfa WHERE tms_user_id = $1"
);

pub const GET_USER_MFA_EXISTS: &str = concat!(
    "SELECT 1 FROM user_mfa WHERE tms_user_id = $1"
);

pub const UPDATE_USER_MFA_ENABLED: &str = concat!(
    "UPDATE user_mfa SET enabled = $1, updated = $2 WHERE tms_user_id = $3"
);

pub const DELETE_USER_MFA: &str = concat!(
    "DELETE FROM user_mfa WHERE tms_user_id = $1"
);

// Secret elided.
pub const LIST_USER_MFA: &str = concat!(
    "SELECT id, tms_user_id, expires_at, enabled, created, updated ",
    "FROM user_mfa ORDER BY tms_user_id",
);

// ========================= user_hosts table =======================
pub const INSERT_USER_HOSTS: &str = concat!(
    "INSERT INTO user_hosts (tms_user_id, host, host_account, expires_at, created, updated) ",
    "VALUES ($1, $2, $3, $4, $5, $6)",
);

pub const INSERT_USER_HOSTS_NOT_STRICT: &str = concat!(
    "INSERT INTO user_hosts (tms_user_id, host, host_account, expires_at, created, updated) ",
    "VALUES ($1, $2, $3, $4, $5, $6) ON CONFLICT DO NOTHING",
);

pub const GET_USER_HOST: &str = concat!(
    "SELECT id, tms_user_id, host, host_account, expires_at, created, updated ",
    "FROM user_hosts WHERE id = $1"
);

pub const GET_USER_HOST_ACTIVE: &str = concat!(
    "SELECT expires_at ",
    "FROM user_hosts WHERE tms_user_id = $1 AND host = $2 AND host_account = $3"
);

pub const GET_USER_HOST_EXISTS: &str = concat!(
    "SELECT 1 FROM user_hosts WHERE tms_user_id = $1 AND host = $2 AND host_account = $3"
);

pub const DELETE_USER_HOST: &str = concat!(
    "DELETE FROM user_hosts WHERE tms_user_id = $1 AND host = $2 AND host_account = $3"
);

pub const LIST_USER_HOSTS: &str = concat!(
    "SELECT id, tms_user_id, host, host_account, expires_at, created, updated ",
    "FROM user_hosts ORDER BY tms_user_id, host, host_account",
);

pub const UPDATE_USER_HOST_EXPIRY: &str = concat!(
    "UPDATE user_hosts SET expires_at = $1, updated = $2 ",
    "WHERE tms_user_id = $3 AND host = $4 AND host_account = $5",
);

// ========================= user_delegations table =================
pub const INSERT_DELEGATIONS: &str = concat!(
    "INSERT INTO delegations (client_id, client_user_id, expires_at, created, updated) ",
    "VALUES ($1, $2, $3, $4, $5)",
);

pub const INSERT_DELEGATIONS_NOT_STRICT: &str = concat!(
    "INSERT INTO delegations (client_id, client_user_id, expires_at, created, updated) ",
    "VALUES ($1, $2, $3, $4, $5) ON CONFLICT DO NOTHING",
);

pub const GET_DELEGATION: &str = concat!(
    "SELECT id, client_id, client_user_id, expires_at, created, updated ",
    "FROM delegations WHERE id = $1"
);

pub const GET_DELEGATION_ACTIVE: &str = concat!(
    "SELECT expires_at ",
    "FROM delegations WHERE client_id = $1 AND client_user_id = $2"
);

pub const GET_DELEGATION_EXISTS: &str = concat!(
    "SELECT 1 FROM delegations WHERE client_id = $1 AND client_user_id = $2"
);

pub const SEL_DELEGATION_EXISTS: &str = concat!(
    "SELECT EXISTS(SELECT 1 FROM delegations WHERE client_id = $1 AND client_user_id = $2)"
);

pub const LIST_DELEGATIONS: &str = concat!(
    "SELECT id, client_id, client_user_id, expires_at, created, updated ",
    "FROM delegations ORDER BY client_id, client_user_id",
);

pub const DELETE_DELEGATION: &str = concat!(
    "DELETE FROM delegations WHERE client_id = $1 AND client_user_id = $2"
);

pub const UPDATE_DELEGATION_EXPIRY: &str = concat!(
    "UPDATE delegations SET expires_at = $1, updated = $2 ",
    "WHERE client_id = $3 AND client_user_id = $4",
);

// ========================= pubkeys table =========================
pub const INSERT_PUBKEYS: &str = concat!(
    "INSERT INTO pubkeys (client_id, client_user_id, host, host_account, public_key_fingerprint, public_key, ",
    "key_type, key_bits, max_uses, remaining_uses, initial_ttl_minutes, expires_at, created, updated) ", 
    "VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)",
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

pub const SELECT_PUBKEY_HOST_ACCOUNT: &str = concat!(
    "SELECT host_account FROM pubkeys ",
    "WHERE client_id = $1 AND host = $2 AND public_key_fingerprint = $3",
);

pub const SELECT_PUBKEY_RESERVATION_INFO: &str = concat!(
    "SELECT remaining_uses, expires_at, host_account FROM pubkeys ",
    "WHERE client_id = $1 AND host = $2 AND public_key_fingerprint = $3",
);

pub const GET_PUBKEY_TEMPLATE: &str = concat!(
    "SELECT id, client_id, client_user_id, host, host_account, public_key_fingerprint, public_key, ",
    "key_type, key_bits, max_uses, remaining_uses, initial_ttl_minutes, expires_at, created, updated ",
    "FROM pubkeys WHERE id = $1 ${PLACEHOLDER}",
);

pub const LIST_PUBKEYS_TEMPLATE: &str = concat!(
    "SELECT id, client_id, client_user_id, host, host_account, public_key_fingerprint, public_key, ",
    "key_type, key_bits, max_uses, remaining_uses, initial_ttl_minutes, expires_at, created, updated ",
    "FROM pubkeys ${PLACEHOLDER} ORDER BY client_user_id, host, host_account",
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
pub const INSERT_HOSTS: &str = concat!(
    "INSERT INTO hosts (host, addr, created, updated) ",
    "VALUES ($1, $2, $3, $4)",
);

pub const GET_HOST: &str = concat!(
    "SELECT id, host, addr, created, updated ",
    "FROM hosts WHERE id = $1"
);

pub const DELETE_HOST: &str = concat!(
    "DELETE FROM hosts WHERE host = $1 AND addr = $2"
);

pub const LIST_HOSTS: &str = concat!(
    "SELECT id, host, addr, created, updated ",
    "FROM hosts ORDER BY host, addr",
);

// ==================== reservations table =========================
pub const INSERT_RESERVATIONS: &str = concat!(
    "INSERT INTO reservations (resid, parent_resid, client_id, client_user_id, ",
    "host, public_key_fingerprint, expires_at, created, updated) ",
    "VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
);

pub const GET_RESERVATION: &str = concat!(
    "SELECT id, resid, parent_resid, client_id, client_user_id, host, ",
    "public_key_fingerprint, expires_at, created, updated ",
    "FROM reservations WHERE resid = $1",
);

pub const GET_RESERVATION_FOR_EXTEND: &str = concat!(
    "SELECT parent_resid, expires_at FROM reservations ", 
    "WHERE resid = $1 AND client_id = $2",
);

pub const DELETE_RESERVATION: &str = concat!(
    "DELETE FROM reservations WHERE resid = $1 AND client_id = $2"
);

pub const DELETE_RELATED_RESERVATIONS: &str = concat!(
    "DELETE FROM reservations WHERE (resid = $1 OR parent_resid = $2) AND client_id = $3"
);
