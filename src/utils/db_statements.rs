// This file contains all SQL statements issued by TMS.
#![forbid(unsafe_code)]

pub const INSERT_PUBKEYS: &str = concat!(
    "INSERT INTO pubkeys (tenant, client_user_id, host, public_key_fingerprint, public_key, key_type, ",
    "key_bits, max_uses, remaining_uses, initial_ttl_minutes, expires_at, created, updated) ", 
    "VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
);

pub const SELECT_PUBKEY: &str = concat!(
    "SELECT public_key, remaining_uses, expires_at FROM pubkeys ",
    "WHERE client_user_id = ? AND host = ? AND public_key_fingerprint = ?",
);

