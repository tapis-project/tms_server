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
CREATE INDEX IF NOT EXISTS clts_created_idx ON clients (created);

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
CREATE INDEX IF NOT EXISTS uhost_created_idx ON user_hosts (created);

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
CREATE INDEX IF NOT EXISTS delg_created_idx ON delegations (created);

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
    key_name               TEXT NOT NULL,
    host                   TEXT NOT NULL,
    public_key_fingerprint TEXT NOT NULL,
    public_key             TEXT NOT NULL,
    max_uses               INT  NOT NULL,
    remaining_uses         INT  NOT NULL,
    expires_at             TEXT NOT NULL,
    created                TEXT NOT NULL,
    updated                TEXT NOT NULL
) STRICT;

CREATE UNIQUE INDEX IF NOT EXISTS pubk_key_name_idx ON pubkeys (tenant, user_name, key_name);
CREATE UNIQUE INDEX IF NOT EXISTS pubk_fprint_idx ON pubkeys (public_key_fingerprint, host);
CREATE INDEX IF NOT EXISTS pubk_expires_idx ON pubkeys (expires_at);
CREATE INDEX IF NOT EXISTS pubk_created_idx ON pubkeys (created);

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
CREATE INDEX IF NOT EXISTS adm_created_idx ON admin (created);

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
CREATE INDEX IF NOT EXISTS host_created_idx ON hosts (created);
