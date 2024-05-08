-- Initial TMS database schema.
-- Does not include foreign key definitions.

-- ---------------------------------------
-- clients table
-- ---------------------------------------
CREATE TABLE IF NOT EXISTS clients
(
    id            INTEGER PRIMARY KEY NOT NULL,
    app_name      TEXT NOT NULL,
    client_id     TEXT NOT NULL,
    client_secret TEXT NOT NULL,
    created       TEXT NOT NULL,
    updated       TEXT NOT NULL
) STRICT;

CREATE UNIQUE INDEX IF NOT EXISTS clts_app_name_idx ON clients (app_name);
CREATE UNIQUE INDEX IF NOT EXISTS clts_client_id_idx ON clients (client_id);
CREATE INDEX IF NOT EXISTS clts_updated_idx ON clients (updated);

-- ---------------------------------------
-- user_mfa table
-- ---------------------------------------
-- This table records when a user's MFA validation will expire.
CREATE TABLE IF NOT EXISTS user_mfa
(
    id                     INTEGER PRIMARY KEY NOT NULL,
    tenant                 TEXT NOT NULL,
    user_name              TEXT NOT NULL,
    expires_at             TEXT NOT NULL,
    created                TEXT NOT NULL,
    updated                TEXT NOT NULL
) STRICT; 

CREATE UNIQUE INDEX IF NOT EXISTS umfa_name_idx ON user_mfa (tenant, user_name);
CREATE INDEX IF NOT EXISTS umfa_expires_idx ON user_mfa (expires_at);
CREATE INDEX IF NOT EXISTS umfa_updated_idx ON user_mfa (updated);

-- ---------------------------------------
-- user_hosts table
-- ---------------------------------------
-- If both user_name and user_name_on_host are set to "*",
-- then all users have their identity linked on host.
CREATE TABLE IF NOT EXISTS user_hosts
(
    id                INTEGER PRIMARY KEY NOT NULL,
    tenant            TEXT NOT NULL,
    user_name         TEXT NOT NULL,
    host              TEXT NOT NULL,
    user_name_on_host TEXT NOT NULL,
    created           TEXT NOT NULL,
    updated           TEXT NOT NULL
) STRICT;

CREATE UNIQUE INDEX IF NOT EXISTS uh_user_name_idx ON user_hosts (tenant, user_name, host);
CREATE INDEX IF NOT EXISTS uhost_host_idx ON user_hosts (host);
CREATE INDEX IF NOT EXISTS uhost_user_on_host_idx ON user_hosts (user_name_on_host);
CREATE INDEX IF NOT EXISTS uhost_updated_idx ON user_hosts (updated);

-- ---------------------------------------
-- delegations table
-- ---------------------------------------
-- If user_name "*", then the delegation applies to all users.
CREATE TABLE IF NOT EXISTS delegations
(
    id                INTEGER PRIMARY KEY NOT NULL,
    tenant            TEXT NOT NULL,
    user_name         TEXT NOT NULL,
    client_id         TEXT NOT NULL,
    created           TEXT NOT NULL,
    updated           TEXT NOT NULL
) STRICT;

CREATE UNIQUE INDEX IF NOT EXISTS delg_user_client_idx ON delegations (tenant, user_name, client_id);
CREATE INDEX IF NOT EXISTS delg_updated_idx ON delegations (updated);

-- ---------------------------------------
-- pubkeys table
-- ---------------------------------------
-- Public keys tie users and hosts together. The same public key can be 
-- used on multiple hosts, though that practice is discouraged.  A user
-- can have multiple keys defined for a host, but application code must
-- ensure that a key can only be used by a single user in a single tenant.
CREATE TABLE IF NOT EXISTS pubkeys
(
    id                     INTEGER PRIMARY KEY NOT NULL,
    tenant                 TEXT NOT NULL,
    user_name              TEXT NOT NULL,
    host                   TEXT NOT NULL,
    public_key_fingerprint TEXT NOT NULL,
    public_key             TEXT NOT NULL,
    key_type               TEXT NOT NULL,
    key_bits               INT  NOT NULL,
    max_uses               INT  NOT NULL,
    remaining_uses         INT  NOT NULL,
    initial_ttl_minutes    INT  NOT NULL,
    expires_at             TEXT NOT NULL,
    created                TEXT NOT NULL,
    updated                TEXT NOT NULL
) STRICT;

CREATE UNIQUE INDEX IF NOT EXISTS pubk_fprint_idx ON pubkeys (public_key_fingerprint, host);
CREATE INDEX IF NOT EXISTS pubk_tenant_user_idx ON pubkeys (tenant, user_name);
CREATE INDEX IF NOT EXISTS pubk_expires_idx ON pubkeys (expires_at);
CREATE INDEX IF NOT EXISTS pubk_updated_idx ON pubkeys (updated);

-- ---------------------------------------
-- reservations table
-- ---------------------------------------
-- Record a record for each tenant/user_name/host/fingerprint/client combination
-- in a single reservation (i.e., with the same resid).
CREATE TABLE IF NOT EXISTS reservations
(
    id                     INTEGER PRIMARY KEY NOT NULL,
    resid                  TEXT NOT NULL,
    tenant                 TEXT NOT NULL,
    user_name              TEXT NOT NULL,
    host                   TEXT NOT NULL,
    public_key_fingerprint TEXT NOT NULL,
    client_id              TEXT NOT NULL,
    expires_at             TEXT NOT NULL,
    created                TEXT NOT NULL,
    updated                TEXT NOT NULL
) STRICT;

CREATE UNIQUE INDEX IF NOT EXISTS res_unique_tuple_idx ON reservations (resid, tenant, user_name, host, client_id, public_key_fingerprint);
CREATE INDEX IF NOT EXISTS res_resid_idx ON reservations (resid);
CREATE INDEX IF NOT EXISTS res_tenant_user_idx ON reservations (tenant, user_name); 
CREATE INDEX IF NOT EXISTS res_client_idx ON reservations (client_id);
CREATE INDEX IF NOT EXISTS res_expires_idx ON reservations (expires_at);
CREATE INDEX IF NOT EXISTS res_updated_idx ON reservations (updated);

-- ---------------------------------------
-- admin table
-- ---------------------------------------
-- ENUMS not supported and column checks cannot be altered,
-- so valid privileges enforced in application code 
CREATE TABLE IF NOT EXISTS admin
(
    id                INTEGER PRIMARY KEY NOT NULL,
    tenant            TEXT NOT NULL,
    user_name         TEXT NOT NULL,
    privilege         TEXT NOT NULL,
    created           TEXT NOT NULL,
    updated           TEXT NOT NULL
) STRICT;

CREATE UNIQUE INDEX IF NOT EXISTS adm_user_priv_idx ON admin (tenant, user_name, privilege);
CREATE INDEX IF NOT EXISTS adm_updated_idx ON admin (updated);

-- ---------------------------------------
-- hosts table
-- ---------------------------------------
-- addr takes 3 forms:
--  IPv4 addr 
--  IPv4 addr with at least 2 segments, the last can be an asterisk (*)
--  IPv4 range [addr1, addr2] where both addresses are full IPv4 addresses 
CREATE TABLE IF NOT EXISTS hosts
(
    id                INTEGER PRIMARY KEY NOT NULL,
    host              TEXT NOT NULL,
    addr              TEXT NOT NULL,
    created           TEXT NOT NULL,
    updated           TEXT NOT NULL
) STRICT;

CREATE UNIQUE INDEX IF NOT EXISTS host_host_addr_idx ON hosts (host, addr);
CREATE INDEX IF NOT EXISTS host_updated_idx ON hosts (updated);
