-- Initial TMS database schema

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
CREATE TABLE IF NOT EXISTS user_hosts
(
    id                INTEGER PRIMARY KEY NOT NULL,
    user_name         TEXT NOT NULL,
    host              TEXT NOT NULL,
    user_name_on_host TEXT NOT NULL,
    created           TEXT NOT NULL,
    updated           TEXT NOT NULL
) STRICT;

CREATE UNIQUE INDEX IF NOT EXISTS uh_user_name_idx ON user_hosts (user_name);
CREATE INDEX IF NOT EXISTS uhost_host_idx ON user_hosts (host);
CREATE INDEX IF NOT EXISTS uhost_user_on_host_idx ON user_hosts (user_name_on_host);
CREATE INDEX IF NOT EXISTS uhost_created_idx ON user_hosts (created);

-- ---------------------------------------
-- delegations table
-- ---------------------------------------
CREATE TABLE IF NOT EXISTS delegations
(
    id                INTEGER PRIMARY KEY NOT NULL,
    user_name         TEXT NOT NULL,
    client_id         TEXT NOT NULL,
    created           TEXT NOT NULL,
    updated           TEXT NOT NULL
) STRICT;

CREATE UNIQUE INDEX IF NOT EXISTS delg_user_client_idx ON delegations (user_name, client_id);
CREATE INDEX IF NOT EXISTS delg_created_idx ON delegations (created);

-- ---------------------------------------
-- pubkeys table
-- ---------------------------------------
CREATE TABLE IF NOT EXISTS pubkeys
(
    id                INTEGER PRIMARY KEY NOT NULL,
    key_name          TEXT NOT NULL,
    user_name         TEXT NOT NULL,
    host              TEXT NOT NULL,
    public_key        TEXT NOT NULL,
    ttl               TEXT NOT NULL,
    max_uses          INT  NOT NULL,
    available_uses    INT  NOT NULL,
    created           TEXT NOT NULL,
    updated           TEXT NOT NULL
) STRICT;

CREATE UNIQUE INDEX IF NOT EXISTS pubk_key_name_idx ON pubkeys (key_name);
CREATE UNIQUE INDEX IF NOT EXISTS pubk_user_host_idx ON pubkeys (user_name, host);
CREATE INDEX IF NOT EXISTS pubk_created_idx ON pubkeys (created);

