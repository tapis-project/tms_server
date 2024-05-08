// This file contains all SQL statements issued by TMS.
#![forbid(unsafe_code)]

pub const INSERT_PUBKEYS: &str = concat!(
    "INSERT INTO PUBKEYS (tenant, user_name, host, public_key_fingerprint, public_key, key_type, ",
    "key_bits, max_uses, remaining_uses, initial_ttl_minutes, expires_at, created, updated) ", 
    "VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
);

