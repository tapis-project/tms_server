// This file contains all SQL statements issued by TMS.
#![forbid(unsafe_code)]

// ========================= tenants table =========================
pub const INSERT_STD_TENANTS: &str = concat!(
    "INSERT OR IGNORE INTO tenants (tenant, created, updated) ",
    "VALUES (?, ?, ?)",
);

// ========================= pubkeys table =========================
pub const INSERT_PUBKEYS: &str = concat!(
    "INSERT INTO pubkeys (tenant, client_user_id, host, host_account, public_key_fingerprint, public_key, key_type, ",
    "key_bits, max_uses, remaining_uses, initial_ttl_minutes, expires_at, created, updated) ", 
    "VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
);

pub const SELECT_PUBKEY: &str = concat!(
    "SELECT public_key, remaining_uses, expires_at FROM pubkeys ",
    "WHERE host_account = ? AND host = ? AND public_key_fingerprint = ?",
);

