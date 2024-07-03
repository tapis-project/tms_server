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

// ========================= user_mfa table ========================
pub const INSERT_USER_MFA: &str = concat!(
    "INSERT INTO user_mfa (tenant, tms_user_id, expires_at, enabled, created, updated) ",
    "VALUES (?, ?, ?, ?, ?, ?)",
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


