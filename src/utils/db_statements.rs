// This file contains all SQL statements issued by TMS.
#![forbid(unsafe_code)]

// ========================= tenants table =========================
pub const INSERT_STD_TENANTS: &str = concat!(
    "INSERT OR IGNORE INTO tenants (tenant, enabled, created, updated) ",
    "VALUES (?, ?, ?, ?)",
);

// ========================= clients table =========================
pub const INSERT_CLIENTS: &str = concat!(
    "INSERT INTO clients (tenant, app_name, app_version, client_id, client_secret, enabled, created, updated) ",
    "VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
);

pub const GET_CLIENT: &str = concat!(
    "SELECT id, tenant, app_name, app_version, client_id, client_secret, enabled, created, updated ",
    "FROM clients WHERE client_id = ? AND tenant = ?",
);

// Secret elided.
pub const LIST_CLIENTS: &str = concat!(
    "SELECT id, tenant, app_name, app_version, client_id, enabled, created, updated ",
    "FROM clients WHERE tenant = ? ORDER BY client_id",
);

// Conforms to the signature required for secret retrieval queries as defined by 
// get_authz_secret() in authz.rs.
pub const GET_CLIENT_SECRET: &str = concat!(
    "SELECT client_secret FROM clients WHERE client_id = ? AND tenant = ?",
);

pub const UPDATE_CLIENT_APP_VERSION: &str = concat!(
    "UPDATE clients SET app_version = ?, updated = ? WHERE client_id = ? AND tenant = ?"
);

pub const UPDATE_CLIENT_ENABLED: &str = concat!(
    "UPDATE clients SET enabled = ?, updated = ? WHERE client_id = ? AND tenant = ?"
);

pub const UPDATE_CLIENT_SECRET: &str = concat!(
    "UPDATE clients SET client_secret = ?, updated = ? WHERE client_id = ? AND tenant = ?"
);

pub const DELETE_CLIENT: &str = concat!(
    "DELETE FROM clients WHERE client_id = ? AND tenant = ?"
);

// ========================= user_mfa table ========================
pub const INSERT_USER_MFA: &str = concat!(
    "INSERT INTO user_mfa (tenant, tms_user_id, expires_at, enabled, created, updated) ",
    "VALUES (?, ?, ?, ?, ?, ?)",
);

pub const GET_USER_MFA: &str = concat!(
    "SELECT id, tenant, tms_user_id, expires_at, enabled, created, updated ",
    "FROM user_mfa WHERE tms_user_id = ? AND tenant = ?"
);

pub const UPDATE_USER_MFA_ENABLED: &str = concat!(
    "UPDATE user_mfa SET enabled = ?, updated = ? WHERE tms_user_id = ? AND tenant = ?"
);

pub const DELETE_USER_MFA: &str = concat!(
    "DELETE FROM user_mfa WHERE tms_user_id = ? AND tenant = ?"
);

// Secret elided.
pub const LIST_USER_MFA: &str = concat!(
    "SELECT id, tenant, tms_user_id, expires_at, enabled, created, updated ",
    "FROM user_mfa WHERE tenant = ? ORDER BY tms_user_id",
);

// ========================= user_hosts table =======================
pub const INSERT_USER_HOSTS: &str = concat!(
    "INSERT INTO user_hosts (tenant, tms_user_id, host, host_account, expires_at, created, updated) ",
    "VALUES (?, ?, ?, ?, ?, ?, ?)",
);

// ========================= user_delegations table =================
pub const INSERT_DELEGATIONS: &str = concat!(
    "INSERT INTO delegations (tenant, client_id, client_user_id, expires_at, created, updated) ",
    "VALUES (?, ?, ?, ?, ?, ?)",
);

// ========================= pubkeys table =========================
pub const INSERT_PUBKEYS: &str = concat!(
    "INSERT INTO pubkeys (tenant, client_id, client_user_id, host, host_account, public_key_fingerprint, public_key, ",
    "key_type, key_bits, max_uses, remaining_uses, initial_ttl_minutes, expires_at, created, updated) ", 
    "VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
);

pub const SELECT_PUBKEY: &str = concat!(
    "SELECT public_key, remaining_uses, expires_at FROM pubkeys ",
    "WHERE host_account = ? AND host = ? AND public_key_fingerprint = ?",
);

// ========================= admin table ===========================
pub const INSERT_ADMIN: &str = concat!(
    "INSERT INTO admin (tenant, admin_user, admin_secret, privilege, created, updated) ",
    "VALUES (?, ?, ?, ?, ?, ?)",
);

// Conforms to the signature required for secret retrieval queries as defined by 
// get_authz_secret() in authz.rs.
pub const GET_ADMIN_SECRET: &str = concat!(
    "SELECT admin_secret FROM admin WHERE admin_user = ? AND tenant = ?",
);

