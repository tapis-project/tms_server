-- Define audit tables and triggers
--
-- All tables follow the same format with the same columns.
-- These columns have the following meanings:
--
--  id - Unique record id
--  refid - Id of record in associated table
--  refname - Name of record in associated table
--  refcol - Name of column that changed
--  change - Type of change: I = Insert, U = Update, D = Delete
--  oldvalue - String representation of original value if one existed
--  newvalue - String representation of new value if there is one
--  changed - UTC time that associated table was changed
--
-- There a triggers for inserts, updates and deletions on each TMS
-- table.  The insert and delete triggers write records that contain
-- a whole row in json format.  Update triggers are defined for
-- each colunm of each TMS table (except the updated column to avoid
-- unnecessary redundancy).  The audit tables usually only have 
-- a primary key to avoid the extra overhead of index management
-- when writing audit records.

-- -------------------------------------------------------------
-- TABLES
-- -------------------------------------------------------------
CREATE TABLE IF NOT EXISTS tenants_audit
(
    id            INTEGER PRIMARY KEY NOT NULL,
    refid         INTEGER NOT NULL,
    refcol        TEXT NOT NULL,
    change        TEXT CHECK( change IN ('I','U','D') ),
    oldvalue      TEXT,
    newvalue      TEXT,
    changed       TEXT NOT NULL default current_timestamp
) STRICT;   

CREATE TABLE IF NOT EXISTS clients_audit
(
    id            INTEGER PRIMARY KEY NOT NULL,
    refid         INTEGER NOT NULL,
    refcol        TEXT NOT NULL,
    change        TEXT CHECK( change IN ('I','U','D') ),
    oldvalue      TEXT,
    newvalue      TEXT,
    changed       TEXT NOT NULL default current_timestamp
) STRICT; 

CREATE TABLE IF NOT EXISTS user_mfa_audit
(
    id            INTEGER PRIMARY KEY NOT NULL,
    refid         INTEGER NOT NULL,
    refcol        TEXT NOT NULL,
    change        TEXT CHECK( change IN ('I','U','D') ),
    oldvalue      TEXT,
    newvalue      TEXT,
    changed       TEXT NOT NULL default current_timestamp
) STRICT; 

CREATE TABLE IF NOT EXISTS user_hosts_audit
(
    id            INTEGER PRIMARY KEY NOT NULL,
    refid         INTEGER NOT NULL,
    refcol        TEXT NOT NULL,
    change        TEXT CHECK( change IN ('I','U','D') ),
    oldvalue      TEXT,
    newvalue      TEXT,
    changed       TEXT NOT NULL default current_timestamp
) STRICT; 

CREATE TABLE IF NOT EXISTS delegations_audit
(
    id            INTEGER PRIMARY KEY NOT NULL,
    refid         INTEGER NOT NULL,
    refcol        TEXT NOT NULL,
    change        TEXT CHECK( change IN ('I','U','D') ),
    oldvalue      TEXT,
    newvalue      TEXT,
    changed       TEXT NOT NULL default current_timestamp
) STRICT; 

CREATE TABLE IF NOT EXISTS pubkeys_audit
(
    id            INTEGER PRIMARY KEY NOT NULL,
    refid         INTEGER NOT NULL,
    refcol        TEXT NOT NULL,
    change        TEXT CHECK( change IN ('I','U','D') ),
    oldvalue      TEXT,
    newvalue      TEXT,
    changed       TEXT NOT NULL default current_timestamp
) STRICT; 

CREATE TABLE IF NOT EXISTS reservations_audit
(
    id            INTEGER PRIMARY KEY NOT NULL,
    refid         INTEGER NOT NULL,
    refcol        TEXT NOT NULL,
    change        TEXT CHECK( change IN ('I','U','D') ),
    oldvalue      TEXT,
    newvalue      TEXT,
    changed       TEXT NOT NULL default current_timestamp
) STRICT; 

CREATE TABLE IF NOT EXISTS admin_audit
(
    id            INTEGER PRIMARY KEY NOT NULL,
    refid         INTEGER NOT NULL,
    refcol        TEXT NOT NULL,
    change        TEXT CHECK( change IN ('I','U','D') ),
    oldvalue      TEXT,
    newvalue      TEXT,
    changed       TEXT NOT NULL default current_timestamp
) STRICT; 

CREATE TABLE IF NOT EXISTS hosts_audit
(
    id            INTEGER PRIMARY KEY NOT NULL,
    refid         INTEGER NOT NULL,
    refcol        TEXT NOT NULL,
    change        TEXT CHECK( change IN ('I','U','D') ),
    oldvalue      TEXT,
    newvalue      TEXT,
    changed       TEXT NOT NULL default current_timestamp
) STRICT; 


-- -------------------------------------------------------------
-- TRIGGERS - tenants
-- -------------------------------------------------------------
CREATE TRIGGER IF NOT EXISTS
    insert_tenants
AFTER INSERT ON
    tenants
FOR EACH ROW 
BEGIN
    INSERT INTO tenants_audit (refid, refcol, change, newvalue)
        VALUES (NEW.id, 'row', 'I', json_array(NEW.id, NEW.tenant, NEW.enabled, NEW.created, NEW.updated));
END;

CREATE TRIGGER IF NOT EXISTS
    delete_tenants
AFTER DELETE ON
    tenants
FOR EACH ROW 
BEGIN
    INSERT INTO tenants_audit (refid, refcol, change, oldvalue)
        VALUES (OLD.id, 'row', 'D', json_array(OLD.id, OLD.tenant, OLD.enabled, OLD.created, OLD.updated));
END;

CREATE TRIGGER IF NOT EXISTS
    update_tenants_id
AFTER UPDATE ON
    tenants
FOR EACH ROW WHEN
    OLD.id != NEW.id
BEGIN
    INSERT INTO tenants_audit (refid, refcol, change, oldvalue, newvalue)
        VALUES (NEW.id, 'id', 'U', CAST(OLD.id as TEXT), CAST(NEW.id as TEXT));
END;

CREATE TRIGGER IF NOT EXISTS
    update_tenants_tenant
AFTER UPDATE ON
    tenants
FOR EACH ROW WHEN
    OLD.tenant != NEW.tenant
BEGIN
    INSERT INTO tenants_audit (refid, refcol, change, oldvalue, newvalue)
        VALUES (NEW.id, 'tenant', 'U', OLD.tenant, NEW.tenant);
END;

CREATE TRIGGER IF NOT EXISTS
    update_tenants_enabled
AFTER UPDATE ON
    tenants
FOR EACH ROW WHEN
    OLD.enabled != NEW.enabled
BEGIN
    INSERT INTO tenants_audit (refid, refcol, change, oldvalue, newvalue)
        VALUES (NEW.id, 'enabled', 'U', CAST(OLD.enabled as TEXT), CAST(NEW.enabled as TEXT));
END;

CREATE TRIGGER IF NOT EXISTS
    update_tenants_created
AFTER UPDATE ON
    tenants
FOR EACH ROW WHEN
    OLD.created != NEW.created
BEGIN
    INSERT INTO tenants_audit (refid, refcol, change, oldvalue, newvalue)
        VALUES (NEW.id, 'created', 'U', OLD.created, NEW.created);
END;

-- -------------------------------------------------------------
-- TRIGGERS - clients
-- -------------------------------------------------------------
CREATE TRIGGER IF NOT EXISTS
    insert_clients
AFTER INSERT ON
    clients
FOR EACH ROW 
BEGIN
    INSERT INTO clients_audit (refid, refcol, change, newvalue)
        VALUES (NEW.id, 'row', 'I', json_array(NEW.id, NEW.tenant, NEW.app_name, NEW.app_version, 
                NEW.client_id, NEW.enabled, NEW.created, NEW.updated));
END;

CREATE TRIGGER IF NOT EXISTS
    delete_clients
AFTER DELETE ON
    clients
FOR EACH ROW 
BEGIN
    INSERT INTO clients_audit (refid, refcol, change, oldvalue)
        VALUES (OLD.id, 'row', 'D', json_array(OLD.id, OLD.tenant, OLD.app_name, OLD.app_version, 
                OLD.client_id, OLD.enabled, OLD.created, OLD.updated));
END;
    
CREATE TRIGGER IF NOT EXISTS
    update_clients_id
AFTER UPDATE ON
    clients
FOR EACH ROW WHEN
    OLD.id != NEW.id
BEGIN
    INSERT INTO clients_audit (refid, refcol, change, oldvalue, newvalue)
        VALUES (NEW.id, 'id', 'U', CAST(OLD.id as TEXT), CAST(NEW.id as TEXT));
END;

CREATE TRIGGER IF NOT EXISTS
    update_clients_tenant
AFTER UPDATE ON
    clients
FOR EACH ROW WHEN
    OLD.tenant != NEW.tenant
BEGIN
    INSERT INTO clients_audit (refid, refcol, change, oldvalue, newvalue)
        VALUES (NEW.id, 'tenant', 'U', OLD.tenant, NEW.tenant);
END;

CREATE TRIGGER IF NOT EXISTS
    update_clients_app_name
AFTER UPDATE ON
    clients
FOR EACH ROW WHEN
    OLD.app_name != NEW.app_name
BEGIN
    INSERT INTO clients_audit (refid, refcol, change, oldvalue, newvalue)
        VALUES (NEW.id, 'app_name', 'U', OLD.app_name, NEW.app_name);
END;

CREATE TRIGGER IF NOT EXISTS
    update_clients_app_version
AFTER UPDATE ON
    clients
FOR EACH ROW WHEN
    OLD.app_version != NEW.app_version
BEGIN
    INSERT INTO clients_audit (refid, refcol, change, oldvalue, newvalue)
        VALUES (NEW.id, 'app_version', 'U', OLD.app_version, NEW.app_version);
END;

CREATE TRIGGER IF NOT EXISTS
    update_clients_client_id
AFTER UPDATE ON
    clients
FOR EACH ROW WHEN
    OLD.client_id != NEW.client_id
BEGIN
    INSERT INTO clients_audit (refid, refcol, change, oldvalue, newvalue)
        VALUES (NEW.id, 'client_id', 'U', OLD.client_id, NEW.client_id);
END;

CREATE TRIGGER IF NOT EXISTS
    update_clients_enabled
AFTER UPDATE ON
    clients
FOR EACH ROW WHEN
    OLD.enabled != NEW.enabled
BEGIN
    INSERT INTO clients_audit (refid, refcol, change, oldvalue, newvalue)
        VALUES (NEW.id, 'enabled', 'U', CAST(OLD.enabled as TEXT), CAST(NEW.enabled as TEXT));
END;

CREATE TRIGGER IF NOT EXISTS
    update_clients_created
AFTER UPDATE ON
    clients
FOR EACH ROW WHEN
    OLD.created != NEW.created
BEGIN
    INSERT INTO clients_audit (refid, refcol, change, oldvalue, newvalue)
        VALUES (NEW.id, 'created', 'U', OLD.created, NEW.created);
END;

-- CREATE TABLE IF NOT EXISTS clients
-- (
--     id            INTEGER PRIMARY KEY NOT NULL,
--     tenant        TEXT NOT NULL,
--     app_name      TEXT NOT NULL,
--     app_version   TEXT NOT NULL,
--     client_id     TEXT NOT NULL,
--     client_secret TEXT NOT NULL,
--     enabled       INTEGER NOT NULL CHECK (enabled IN (0, 1)),
--     created       TEXT NOT NULL,
--     updated       TEXT NOT NULL, 
--     FOREIGN KEY(tenant) REFERENCES tenants(tenant) ON UPDATE CASCADE ON DELETE RESTRICT
-- ) STRICT;


-- -------------------------------------------------------------
-- TRIGGERS - user_mfa
-- -------------------------------------------------------------
CREATE TRIGGER IF NOT EXISTS
    insert_user_mfa
AFTER INSERT ON
    user_mfa
FOR EACH ROW 
BEGIN
    INSERT INTO user_mfa_audit (refid, refcol, change, newvalue)
        VALUES (NEW.id, 'row', 'I', json_array(NEW.id, NEW.tenant, NEW.tms_user_id, NEW.expires_at,
                NEW.enabled, NEW.created, NEW.updated));
END;

CREATE TRIGGER IF NOT EXISTS
    delete_user_mfa
AFTER DELETE ON
    user_mfa
FOR EACH ROW 
BEGIN
    INSERT INTO user_mfa_audit (refid, refcol, change, oldvalue)
        VALUES (OLD.id, 'row', 'D', json_array(OLD.id, OLD.tenant, OLD.tms_user_id, OLD.expires_at,
                OLD.enabled, OLD.created, OLD.updated));
END;

CREATE TRIGGER IF NOT EXISTS
    update_user_mfa_id
AFTER UPDATE ON
    user_mfa
FOR EACH ROW WHEN
    OLD.id != NEW.id
BEGIN
    INSERT INTO user_mfa_audit (refid, refcol, change, oldvalue, newvalue)
        VALUES (NEW.id, 'id', 'U', CAST(OLD.id as TEXT), CAST(NEW.id as TEXT));
END;

CREATE TRIGGER IF NOT EXISTS
    update_user_mfa_tenant
AFTER UPDATE ON
    user_mfa
FOR EACH ROW WHEN
    OLD.tenant != NEW.tenant
BEGIN
    INSERT INTO user_mfa_audit (refid, refcol, change, oldvalue, newvalue)
        VALUES (NEW.id, 'tenant', 'U', OLD.tenant, NEW.tenant);
END;

CREATE TRIGGER IF NOT EXISTS
    update_user_mfa_tms_user_id
AFTER UPDATE ON
    user_mfa
FOR EACH ROW WHEN
    OLD.tms_user_id != NEW.tms_user_id
BEGIN
    INSERT INTO user_mfa_audit (refid, refcol, change, oldvalue, newvalue)
        VALUES (NEW.id, 'tms_user_id', 'U', OLD.tms_user_id, NEW.tms_user_id);
END;

CREATE TRIGGER IF NOT EXISTS
    update_user_mfa_expires_at
AFTER UPDATE ON
    user_mfa
FOR EACH ROW WHEN
    OLD.expires_at != NEW.expires_at
BEGIN
    INSERT INTO user_mfa_audit (refid, refcol, change, oldvalue, newvalue)
        VALUES (NEW.id, 'expires_at', 'U', OLD.expires_at, NEW.expires_at);
END;

CREATE TRIGGER IF NOT EXISTS
    update_user_mfa_enabled
AFTER UPDATE ON
    user_mfa
FOR EACH ROW WHEN
    OLD.enabled != NEW.enabled
BEGIN
    INSERT INTO user_mfa_audit (refid, refcol, change, oldvalue, newvalue)
        VALUES (NEW.id, 'enabled', 'U', CAST(OLD.enabled as TEXT), CAST(NEW.enabled as TEXT));
END;

CREATE TRIGGER IF NOT EXISTS
    update_user_mfa_created
AFTER UPDATE ON
    user_mfa
FOR EACH ROW WHEN
    OLD.created != NEW.created
BEGIN
    INSERT INTO user_mfa_audit (refid, refcol, change, oldvalue, newvalue)
        VALUES (NEW.id, 'created', 'U', OLD.created, NEW.created);
END;

-- CREATE TABLE IF NOT EXISTS user_mfa
-- (
--     id                     INTEGER PRIMARY KEY NOT NULL,
--     tenant                 TEXT NOT NULL,
--     tms_user_id            TEXT NOT NULL,
--     expires_at             TEXT NOT NULL,
--     enabled                INTEGER NOT NULL CHECK (enabled IN (0, 1)),
--     created                TEXT NOT NULL,
--     updated                TEXT NOT NULL,
--     FOREIGN KEY(tenant) REFERENCES tenants(tenant) ON UPDATE CASCADE ON DELETE RESTRICT
-- ) STRICT; 

-- -------------------------------------------------------------
-- TRIGGERS - user_hosts
-- -------------------------------------------------------------
CREATE TRIGGER IF NOT EXISTS
    insert_user_hosts
AFTER INSERT ON
    user_hosts
FOR EACH ROW 
BEGIN
    INSERT INTO user_hosts_audit (refid, refcol, change, newvalue)
        VALUES (NEW.id, 'row', 'I', json_array(NEW.id, NEW.tenant, NEW.tms_user_id, NEW.host, 
                NEW.host_account, NEW.expires_at, NEW.created, NEW.updated));
END;

CREATE TRIGGER IF NOT EXISTS
    delete_user_hosts
AFTER DELETE ON
    user_hosts
FOR EACH ROW 
BEGIN
    INSERT INTO user_hosts_audit (refid, refcol, change, oldvalue)
        VALUES (OLD.id, 'row', 'D', json_array(OLD.id, OLD.tenant, OLD.tms_user_id, OLD.host, 
                OLD.host_account, OLD.expires_at, OLD.created, OLD.updated));
END;

CREATE TRIGGER IF NOT EXISTS
    update_user_hosts_id
AFTER UPDATE ON
    user_hosts
FOR EACH ROW WHEN
    OLD.id != NEW.id
BEGIN
    INSERT INTO user_hosts_audit (refid, refcol, change, oldvalue, newvalue)
        VALUES (NEW.id, 'id', 'U', CAST(OLD.id as TEXT), CAST(NEW.id as TEXT));
END;

CREATE TRIGGER IF NOT EXISTS
    update_user_hosts_tenant
AFTER UPDATE ON
    user_hosts
FOR EACH ROW WHEN
    OLD.tenant != NEW.tenant
BEGIN
    INSERT INTO user_hosts_audit (refid, refcol, change, oldvalue, newvalue)
        VALUES (NEW.id, 'tenant', 'U', OLD.tenant, NEW.tenant);
END;

CREATE TRIGGER IF NOT EXISTS
    update_user_hosts_tms_user_id
AFTER UPDATE ON
    user_hosts
FOR EACH ROW WHEN
    OLD.tms_user_id != NEW.tms_user_id
BEGIN
    INSERT INTO user_hosts_audit (refid, refcol, change, oldvalue, newvalue)
        VALUES (NEW.id, 'tms_user_id', 'U', OLD.tms_user_id, NEW.tms_user_id);
END;

CREATE TRIGGER IF NOT EXISTS
    update_user_hosts_host
AFTER UPDATE ON
    user_hosts
FOR EACH ROW WHEN
    OLD.host != NEW.host
BEGIN
    INSERT INTO user_hosts_audit (refid, refcol, change, oldvalue, newvalue)
        VALUES (NEW.id, 'host', 'U', OLD.host, NEW.host);
END;

CREATE TRIGGER IF NOT EXISTS
    update_user_hosts_host_account
AFTER UPDATE ON
    user_hosts
FOR EACH ROW WHEN
    OLD.host_account != NEW.host_account
BEGIN
    INSERT INTO user_hosts_audit (refid, refcol, change, oldvalue, newvalue)
        VALUES (NEW.id, 'host_account', 'U', OLD.host_account, NEW.host_account);
END;

CREATE TRIGGER IF NOT EXISTS
    update_user_hosts_expires_at
AFTER UPDATE ON
    user_hosts
FOR EACH ROW WHEN
    OLD.expires_at != NEW.expires_at
BEGIN
    INSERT INTO user_hosts_audit (refid, refcol, change, oldvalue, newvalue)
        VALUES (NEW.id, 'expires_at', 'U', OLD.expires_at, NEW.expires_at);
END;

CREATE TRIGGER IF NOT EXISTS
    update_user_hosts_created
AFTER UPDATE ON
    user_hosts
FOR EACH ROW WHEN
    OLD.created != NEW.created
BEGIN
    INSERT INTO user_hosts_audit (refid, refcol, change, oldvalue, newvalue)
        VALUES (NEW.id, 'created', 'U', OLD.created, NEW.created);
END;

-- CREATE TABLE IF NOT EXISTS user_hosts
-- (
--     id                INTEGER PRIMARY KEY NOT NULL,
--     tenant            TEXT NOT NULL,
--     tms_user_id       TEXT NOT NULL,
--     host              TEXT NOT NULL,
--     host_account      TEXT NOT NULL,
--     expires_at        TEXT NOT NULL,
--     created           TEXT NOT NULL,
--     updated           TEXT NOT NULL,
--     FOREIGN KEY(tenant) REFERENCES tenants(tenant) ON UPDATE CASCADE ON DELETE RESTRICT,
--     FOREIGN KEY(tenant, tms_user_id) REFERENCES user_mfa(tenant, tms_user_id) ON UPDATE CASCADE ON DELETE CASCADE
-- ) STRICT;

-- -------------------------------------------------------------
-- TRIGGERS - delegations
-- -------------------------------------------------------------
CREATE TRIGGER IF NOT EXISTS
    insert_delegations
AFTER INSERT ON
    delegations
FOR EACH ROW 
BEGIN
    INSERT INTO delegations_audit (refid, refcol, change, newvalue)
        VALUES (NEW.id, 'row', 'I', json_array(NEW.id, NEW.tenant, NEW.client_id, NEW.client_user_id,
                NEW.expires_at, NEW.created, NEW.updated));
END;

CREATE TRIGGER IF NOT EXISTS
    delete_delegations
AFTER DELETE ON
    delegations
FOR EACH ROW 
BEGIN
    INSERT INTO delegations_audit (refid, refcol, change, oldvalue)
        VALUES (OLD.id, 'row', 'D', json_array(OLD.id, OLD.tenant, NEOLDW.client_id, OLD.client_user_id,
                OLD.expires_at, OLD.created, OLD.updated));
END;

CREATE TRIGGER IF NOT EXISTS
    update_delegations_id
AFTER UPDATE ON
    delegations
FOR EACH ROW WHEN
    OLD.id != NEW.id
BEGIN
    INSERT INTO delegations_audit (refid, refcol, change, oldvalue, newvalue)
        VALUES (NEW.id, 'id', 'U', CAST(OLD.id as TEXT), CAST(NEW.id as TEXT));
END;

CREATE TRIGGER IF NOT EXISTS
    update_delegations_tenant
AFTER UPDATE ON
    delegations
FOR EACH ROW WHEN
    OLD.tenant != NEW.tenant
BEGIN
    INSERT INTO delegations_audit (refid, refcol, change, oldvalue, newvalue)
        VALUES (NEW.id, 'tenant', 'U', OLD.tenant, NEW.tenant);
END;

CREATE TRIGGER IF NOT EXISTS
    update_delegations_client_id
AFTER UPDATE ON
    delegations
FOR EACH ROW WHEN
    OLD.client_id != NEW.client_id
BEGIN
    INSERT INTO delegations_audit (refid, refcol, change, oldvalue, newvalue)
        VALUES (NEW.id, 'client_id', 'U', OLD.client_id, NEW.client_id);
END;

CREATE TRIGGER IF NOT EXISTS
    update_delegations_client_user_id
AFTER UPDATE ON
    delegations
FOR EACH ROW WHEN
    OLD.client_user_id != NEW.client_user_id
BEGIN
    INSERT INTO delegations_audit (refid, refcol, change, oldvalue, newvalue)
        VALUES (NEW.id, 'client_user_id', 'U', OLD.client_user_id, NEW.client_user_id);
END;

CREATE TRIGGER IF NOT EXISTS
    update_delegations_expires_at
AFTER UPDATE ON
    delegations
FOR EACH ROW WHEN
    OLD.expires_at != NEW.expires_at
BEGIN
    INSERT INTO delegations_audit (refid, refcol, change, oldvalue, newvalue)
        VALUES (NEW.id, 'expires_at', 'U', OLD.expires_at, NEW.expires_at);
END;

CREATE TRIGGER IF NOT EXISTS
    update_delegations_created
AFTER UPDATE ON
    delegations
FOR EACH ROW WHEN
    OLD.created != NEW.created
BEGIN
    INSERT INTO delegations_audit (refid, refcol, change, oldvalue, newvalue)
        VALUES (NEW.id, 'created', 'U', OLD.created, NEW.created);
END;

-- CREATE TABLE IF NOT EXISTS delegations
-- (
--     id                INTEGER PRIMARY KEY NOT NULL,
--     tenant            TEXT NOT NULL,
--     client_id         TEXT NOT NULL,
--     client_user_id    TEXT NOT NULL,
--     expires_at        TEXT NOT NULL,
--     created           TEXT NOT NULL,
--     updated           TEXT NOT NULL,
--     FOREIGN KEY(tenant) REFERENCES tenants(tenant) ON UPDATE CASCADE ON DELETE RESTRICT,
--     FOREIGN KEY(tenant, client_user_id) REFERENCES user_mfa(tenant, tms_user_id) ON UPDATE CASCADE ON DELETE CASCADE,
--     FOREIGN KEY(client_id) REFERENCES clients(client_id) ON UPDATE CASCADE ON DELETE CASCADE 
-- ) STRICT;

-- -------------------------------------------------------------
-- TRIGGERS - pubkeys
-- -------------------------------------------------------------
CREATE TRIGGER IF NOT EXISTS
    insert_pubkeys
AFTER INSERT ON
    pubkeys
FOR EACH ROW 
BEGIN
    INSERT INTO pubkeys_audit (refid, refcol, change, newvalue)
        VALUES (NEW.id, 'row', 'I', json_array(NEW.id, NEW.tenant, NEW.client_id, NEW.client_user_id, NEW.host,
                NEW.host_account, NEW.public_key_fingerprint, NEW.public_key, NEW.key_type, NEW.key_bits,
                NEW.max_uses, NEW.remaining_uses, NEW.initial_ttl_minutes, NEW.expires_at, NEW.created, NEW.updated));
END;

CREATE TRIGGER IF NOT EXISTS
    delete_pubkeys
AFTER DELETE ON
    pubkeys
FOR EACH ROW 
BEGIN
    INSERT INTO pubkeys_audit (refid, refcol, change, oldvalue)
        VALUES (OLD.id, 'row', 'D', json_array(OLD.id, OLD.tenant, OLD.client_id, OLD.client_user_id, OLD.host,
                OLD.host_account, OLD.public_key_fingerprint, OLD.public_key, OLD.key_type, OLD.key_bits,
                OLD.max_uses, OLD.remaining_uses, OLD.initial_ttl_minutes, OLD.expires_at, OLD.created, OLD.updated));
END;

CREATE TRIGGER IF NOT EXISTS
    update_pubkeys_id
AFTER UPDATE ON
    pubkeys
FOR EACH ROW WHEN
    OLD.id != NEW.id
BEGIN
    INSERT INTO pubkeys_audit (refid, refcol, change, oldvalue, newvalue)
        VALUES (NEW.id, 'id', 'U', CAST(OLD.id as TEXT), CAST(NEW.id as TEXT));
END;

CREATE TRIGGER IF NOT EXISTS
    update_pubkeys_tenant
AFTER UPDATE ON
    pubkeys
FOR EACH ROW WHEN
    OLD.tenant != NEW.tenant
BEGIN
    INSERT INTO pubkeys_audit (refid, refcol, change, oldvalue, newvalue)
        VALUES (NEW.id, 'tenant', 'U', OLD.tenant, NEW.tenant);
END;

CREATE TRIGGER IF NOT EXISTS
    update_pubkeys_client_id
AFTER UPDATE ON
    pubkeys
FOR EACH ROW WHEN
    OLD.client_id != NEW.client_id
BEGIN
    INSERT INTO pubkeys_audit (refid, refcol, change, oldvalue, newvalue)
        VALUES (NEW.id, 'client_id', 'U', OLD.client_id, NEW.client_id);
END;

CREATE TRIGGER IF NOT EXISTS
    update_pubkeys_client_user_id
AFTER UPDATE ON
    pubkeys
FOR EACH ROW WHEN
    OLD.client_user_id != NEW.client_user_id
BEGIN
    INSERT INTO pubkeys_audit (refid, refcol, change, oldvalue, newvalue)
        VALUES (NEW.id, 'client_user_id', 'U', OLD.client_user_id, NEW.client_user_id);
END;

CREATE TRIGGER IF NOT EXISTS
    update_pubkeys_host
AFTER UPDATE ON
    pubkeys
FOR EACH ROW WHEN
    OLD.host != NEW.host
BEGIN
    INSERT INTO pubkeys_audit (refid, refcol, change, oldvalue, newvalue)
        VALUES (NEW.id, 'host', 'U', OLD.host, NEW.host);
END;

CREATE TRIGGER IF NOT EXISTS
    update_pubkeys_host_account
AFTER UPDATE ON
    pubkeys
FOR EACH ROW WHEN
    OLD.host_account != NEW.host_account
BEGIN
    INSERT INTO pubkeys_audit (refid, refcol, change, oldvalue, newvalue)
        VALUES (NEW.id, 'host_account', 'U', OLD.host_account, NEW.host_account);
END;

CREATE TRIGGER IF NOT EXISTS
    update_pubkeys_public_key_fingerprint
AFTER UPDATE ON
    pubkeys
FOR EACH ROW WHEN
    OLD.public_key_fingerprint != NEW.public_key_fingerprint
BEGIN
    INSERT INTO pubkeys_audit (refid, refcol, change, oldvalue, newvalue)
        VALUES (NEW.id, 'public_key_fingerprint', 'U', OLD.public_key_fingerprint, NEW.public_key_fingerprint);
END;

CREATE TRIGGER IF NOT EXISTS
    update_pubkeys_public_key
AFTER UPDATE ON
    pubkeys
FOR EACH ROW WHEN
    OLD.public_key != NEW.public_key
BEGIN
    INSERT INTO pubkeys_audit (refid, refcol, change, oldvalue, newvalue)
        VALUES (NEW.id, 'public_key', 'U', OLD.public_key, NEW.public_key);
END;

CREATE TRIGGER IF NOT EXISTS
    update_pubkeys_key_type
AFTER UPDATE ON
    pubkeys
FOR EACH ROW WHEN
    OLD.key_type != NEW.key_type
BEGIN
    INSERT INTO pubkeys_audit (refid, refcol, change, oldvalue, newvalue)
        VALUES (NEW.id, 'key_type', 'U', OLD.key_type, NEW.key_type);
END;

CREATE TRIGGER IF NOT EXISTS
    update_pubkeys_key_bits
AFTER UPDATE ON
    pubkeys
FOR EACH ROW WHEN
    OLD.key_bits != NEW.key_bits
BEGIN
    INSERT INTO pubkeys_audit (refid, refcol, change, oldvalue, newvalue)
        VALUES (NEW.id, 'key_bits', 'U', OLD.key_bits, NEW.key_bits);
END;

CREATE TRIGGER IF NOT EXISTS
    update_pubkeys_max_uses
AFTER UPDATE ON
    pubkeys
FOR EACH ROW WHEN
    OLD.max_uses != NEW.max_uses
BEGIN
    INSERT INTO pubkeys_audit (refid, refcol, change, oldvalue, newvalue)
        VALUES (NEW.id, 'max_uses', 'U', OLD.max_uses, NEW.max_uses);
END;

CREATE TRIGGER IF NOT EXISTS
    update_pubkeys_remaining_uses
AFTER UPDATE ON
    pubkeys
FOR EACH ROW WHEN
    OLD.remaining_uses != NEW.remaining_uses
BEGIN
    INSERT INTO pubkeys_audit (refid, refcol, change, oldvalue, newvalue)
        VALUES (NEW.id, 'remaining_uses', 'U', OLD.remaining_uses, NEW.remaining_uses);
END;

CREATE TRIGGER IF NOT EXISTS
    update_pubkeys_initial_ttl_minutes
AFTER UPDATE ON
    pubkeys
FOR EACH ROW WHEN
    OLD.initial_ttl_minutes != NEW.initial_ttl_minutes
BEGIN
    INSERT INTO pubkeys_audit (refid, refcol, change, oldvalue, newvalue)
        VALUES (NEW.id, 'initial_ttl_minutes', 'U', OLD.initial_ttl_minutes, NEW.initial_ttl_minutes);
END;

CREATE TRIGGER IF NOT EXISTS
    update_pubkeys_expires_at
AFTER UPDATE ON
    pubkeys
FOR EACH ROW WHEN
    OLD.expires_at != NEW.expires_at
BEGIN
    INSERT INTO pubkeys_audit (refid, refcol, change, oldvalue, newvalue)
        VALUES (NEW.id, 'expires_at', 'U', OLD.expires_at, NEW.expires_at);
END;

CREATE TRIGGER IF NOT EXISTS
    update_pubkeys_created
AFTER UPDATE ON
    pubkeys
FOR EACH ROW WHEN
    OLD.created != NEW.created
BEGIN
    INSERT INTO pubkeys_audit (refid, refcol, change, oldvalue, newvalue)
        VALUES (NEW.id, 'created', 'U', OLD.created, NEW.created);
END;

-- CREATE TABLE IF NOT EXISTS pubkeys
-- (
--     id                     INTEGER PRIMARY KEY NOT NULL,
--     tenant                 TEXT NOT NULL,
--     client_id              TEXT NOT NULL,
--     client_user_id         TEXT NOT NULL,
--     host                   TEXT NOT NULL,
--     host_account           TEXT NOT NULL,
--     public_key_fingerprint TEXT NOT NULL,
--     public_key             TEXT NOT NULL,
--     key_type               TEXT NOT NULL,
--     key_bits               INT  NOT NULL,
--     max_uses               INT  NOT NULL,
--     remaining_uses         INT  NOT NULL,
--     initial_ttl_minutes    INT  NOT NULL,
--     expires_at             TEXT NOT NULL,
--     created                TEXT NOT NULL,
--     updated                TEXT NOT NULL,
--     FOREIGN KEY(tenant) REFERENCES tenants(tenant) ON UPDATE CASCADE ON DELETE RESTRICT,
--     FOREIGN KEY(tenant, client_user_id) REFERENCES user_mfa(tenant, tms_user_id) ON UPDATE CASCADE ON DELETE CASCADE,
--     FOREIGN KEY(client_id) REFERENCES clients(client_id) ON UPDATE CASCADE ON DELETE CASCADE,
--     FOREIGN KEY(tenant, client_user_id, host, host_account) REFERENCES user_hosts(tenant, tms_user_id, host, host_account) ON UPDATE CASCADE ON DELETE CASCADE,
--     FOREIGN KEY(tenant, client_id, client_user_id) REFERENCES delegations(tenant, client_id, client_user_id) ON UPDATE CASCADE ON DELETE CASCADE
-- ) STRICT;
-- -------------------------------------------------------------
-- TRIGGERS - reservations
-- -------------------------------------------------------------
CREATE TRIGGER IF NOT EXISTS
    insert_reservations
AFTER INSERT ON
    reservations
FOR EACH ROW 
BEGIN
    INSERT INTO reservations_audit (refid, refcol, change, newvalue)
        VALUES (NEW.id, 'row', 'I', json_array(NEW.id, NEW.resid, NEW.tenant, NEW.client_id, NEW.client_user_id, 
                NEW.host, NEW.public_key_fingerprint, NEW.expires_at, NEW.created, NEW.updated));
END;

CREATE TRIGGER IF NOT EXISTS
    delete_reservations
AFTER DELETE ON
    reservations
FOR EACH ROW 
BEGIN
    INSERT INTO reservations_audit (refid, refcol, change, oldvalue)
        VALUES (OLD.id, 'row', 'D', json_array(OLD.id, OLD.resid, OLD.tenant, OLD.client_id, OLD.client_user_id, 
                OLD.host, OLD.public_key_fingerprint, OLD.expires_at, OLD.created, OLD.updated));
END;

CREATE TRIGGER IF NOT EXISTS
    update_reservations_id
AFTER UPDATE ON
    reservations
FOR EACH ROW WHEN
    OLD.id != NEW.id
BEGIN
    INSERT INTO reservations_audit (refid, refcol, change, oldvalue, newvalue)
        VALUES (NEW.id, 'id', 'U', CAST(OLD.id as TEXT), CAST(NEW.id as TEXT));
END;

CREATE TRIGGER IF NOT EXISTS
    update_reservations_resid
AFTER UPDATE ON
    reservations
FOR EACH ROW WHEN
    OLD.resid != NEW.resid
BEGIN
    INSERT INTO reservations_audit (refid, refcol, change, oldvalue, newvalue)
        VALUES (NEW.id, 'resid', 'U', OLD.resid, NEW.resid);
END;

CREATE TRIGGER IF NOT EXISTS
    update_reservations_tenant
AFTER UPDATE ON
    reservations
FOR EACH ROW WHEN
    OLD.tenant != NEW.tenant
BEGIN
    INSERT INTO reservations_audit (refid, refcol, change, oldvalue, newvalue)
        VALUES (NEW.id, 'tenant', 'U', OLD.tenant, NEW.tenant);
END;

CREATE TRIGGER IF NOT EXISTS
    update_reservations_client_id
AFTER UPDATE ON
    reservations
FOR EACH ROW WHEN
    OLD.client_id != NEW.client_id
BEGIN
    INSERT INTO reservations_audit (refid, refcol, change, oldvalue, newvalue)
        VALUES (NEW.id, 'client_id', 'U', OLD.client_id, NEW.client_id);
END;

CREATE TRIGGER IF NOT EXISTS
    update_reservations_client_user_id
AFTER UPDATE ON
    reservations
FOR EACH ROW WHEN
    OLD.client_user_id != NEW.client_user_id
BEGIN
    INSERT INTO reservations_audit (refid, refcol, change, oldvalue, newvalue)
        VALUES (NEW.id, 'client_user_id', 'U', OLD.client_user_id, NEW.client_user_id);
END;

CREATE TRIGGER IF NOT EXISTS
    update_reservations_host
AFTER UPDATE ON
    reservations
FOR EACH ROW WHEN
    OLD.host != NEW.host
BEGIN
    INSERT INTO reservations_audit (refid, refcol, change, oldvalue, newvalue)
        VALUES (NEW.id, 'host', 'U', OLD.host, NEW.host);
END;

CREATE TRIGGER IF NOT EXISTS
    update_reservations_public_key_fingerprint
AFTER UPDATE ON
    reservations
FOR EACH ROW WHEN
    OLD.public_key_fingerprint != NEW.public_key_fingerprint
BEGIN
    INSERT INTO reservations_audit (refid, refcol, change, oldvalue, newvalue)
        VALUES (NEW.id, 'public_key_fingerprint', 'U', OLD.public_key_fingerprint, NEW.public_key_fingerprint);
END;

CREATE TRIGGER IF NOT EXISTS
    update_reservations_expires_at
AFTER UPDATE ON
    reservations
FOR EACH ROW WHEN
    OLD.expires_at != NEW.expires_at
BEGIN
    INSERT INTO reservations_audit (refid, refcol, change, oldvalue, newvalue)
        VALUES (NEW.id, 'expires_at', 'U', OLD.expires_at, NEW.expires_at);
END;

CREATE TRIGGER IF NOT EXISTS
    update_reservations_created
AFTER UPDATE ON
    reservations
FOR EACH ROW WHEN
    OLD.created != NEW.created
BEGIN
    INSERT INTO reservations_audit (refid, refcol, change, oldvalue, newvalue)
        VALUES (NEW.id, 'created', 'U', OLD.created, NEW.created);
END;

-- CREATE TABLE IF NOT EXISTS reservations
-- (
--     id                     INTEGER PRIMARY KEY NOT NULL,
--     resid                  TEXT NOT NULL,
--     tenant                 TEXT NOT NULL,
--     client_id              TEXT NOT NULL,
--     client_user_id         TEXT NOT NULL,
--     host                   TEXT NOT NULL,
--     public_key_fingerprint TEXT NOT NULL,
--     expires_at             TEXT NOT NULL,
--     created                TEXT NOT NULL,
--     updated                TEXT NOT NULL,
--     FOREIGN KEY(tenant) REFERENCES tenants(tenant) ON UPDATE CASCADE ON DELETE RESTRICT,
--     FOREIGN KEY(tenant, client_user_id) REFERENCES user_mfa(tenant, tms_user_id) ON UPDATE CASCADE ON DELETE CASCADE,
--     FOREIGN KEY(client_id) REFERENCES clients(client_id) ON UPDATE CASCADE ON DELETE CASCADE,
--     FOREIGN KEY(public_key_fingerprint, host) REFERENCES pubkeys(public_key_fingerprint, host) ON UPDATE CASCADE ON DELETE CASCADE
-- ) STRICT;

-- -------------------------------------------------------------
-- TRIGGERS - admin
-- -------------------------------------------------------------
CREATE TRIGGER IF NOT EXISTS
    insert_admin
AFTER INSERT ON
    admin
FOR EACH ROW 
BEGIN
    INSERT INTO admin_audit (refid, refcol, change, newvalue)
        VALUES (NEW.id, 'row', 'I', json_array(NEW.id, NEW.tenant, NEW.admin_user, NEW.privilege, 
                NEW.created, NEW.updated));
END;

CREATE TRIGGER IF NOT EXISTS
    delete_admin
AFTER DELETE ON
    admin
FOR EACH ROW 
BEGIN
    INSERT INTO admin_audit (refid, refcol, change, oldvalue)
        VALUES (OLD.id, 'row', 'D', json_array(OLD.id, OLD.tenant, OLD.admin_user, OLD.privilege, 
                OLD.created, OLD.updated));
END;

CREATE TRIGGER IF NOT EXISTS
    update_admin_id
AFTER UPDATE ON
    admin
FOR EACH ROW WHEN
    OLD.id != NEW.id
BEGIN
    INSERT INTO admin_audit (refid, refcol, change, oldvalue, newvalue)
        VALUES (NEW.id, 'id', 'U', CAST(OLD.id as TEXT), CAST(NEW.id as TEXT));
END;

CREATE TRIGGER IF NOT EXISTS
    update_admin_tenant
AFTER UPDATE ON
    admin
FOR EACH ROW WHEN
    OLD.tenant != NEW.tenant
BEGIN
    INSERT INTO admin_audit (refid, refcol, change, oldvalue, newvalue)
        VALUES (NEW.id, 'tenant', 'U', OLD.tenant, NEW.tenant);
END;

CREATE TRIGGER IF NOT EXISTS
    update_admin_admin_user
AFTER UPDATE ON
    admin
FOR EACH ROW WHEN
    OLD.admin_user != NEW.admin_user
BEGIN
    INSERT INTO admin_audit (refid, refcol, change, oldvalue, newvalue)
        VALUES (NEW.id, 'admin_user', 'U', OLD.admin_user, NEW.admin_user);
END;

CREATE TRIGGER IF NOT EXISTS
    update_admin_admin_secret
AFTER UPDATE ON
    admin
FOR EACH ROW WHEN
    OLD.admin_secret != NEW.admin_secret
BEGIN
    INSERT INTO admin_audit (refid, refcol, change, oldvalue, newvalue)
        VALUES (NEW.id, 'admin_secret', 'U', '****', '****');
END;


CREATE TRIGGER IF NOT EXISTS
    update_admin_privilege
AFTER UPDATE ON
    admin
FOR EACH ROW WHEN
    OLD.privilege != NEW.privilege
BEGIN
    INSERT INTO admin_audit (refid, refcol, change, oldvalue, newvalue)
        VALUES (NEW.id, 'privilege', 'U', OLD.privilege, NEW.privilege);
END;

CREATE TRIGGER IF NOT EXISTS
    update_admin_created
AFTER UPDATE ON
    admin
FOR EACH ROW WHEN
    OLD.created != NEW.created
BEGIN
    INSERT INTO admin_audit (refid, refcol, change, oldvalue, newvalue)
        VALUES (NEW.id, 'created', 'U', OLD.created, NEW.created);
END;

-- CREATE TABLE IF NOT EXISTS admin
-- (
--     id                INTEGER PRIMARY KEY NOT NULL,
--     tenant            TEXT NOT NULL,
--     admin_user        TEXT NOT NULL,
--     admin_secret      TEXT NOT NULL,
--     privilege         TEXT NOT NULL,
--     created           TEXT NOT NULL,
--     updated           TEXT NOT NULL,
--     FOREIGN KEY(tenant) REFERENCES tenants(tenant) ON UPDATE CASCADE ON DELETE RESTRICT,
-- ) STRICT;

-- -------------------------------------------------------------
-- TRIGGERS - hosts
-- -------------------------------------------------------------
CREATE TRIGGER IF NOT EXISTS
    insert_hosts
AFTER INSERT ON
    hosts
FOR EACH ROW 
BEGIN
    INSERT INTO hosts_audit (refid, refcol, change, newvalue)
        VALUES (NEW.id, 'row', 'I', json_array(NEW.id, NEW.tenant, NEW.host, NEW.addr, NEW.created, NEW.updated));
END;

CREATE TRIGGER IF NOT EXISTS
    delete_hosts
AFTER DELETE ON
    hosts
FOR EACH ROW 
BEGIN
    INSERT INTO hosts_audit (refid, refcol, change, oldvalue)
        VALUES (OLD.id, 'row', 'D', json_array(OLD.id, OLD.tenant, OLD.host, OLD.addr, OLD.created, OLD.updated));
END;

CREATE TRIGGER IF NOT EXISTS
    update_hosts_id
AFTER UPDATE ON
    hosts
FOR EACH ROW WHEN
    OLD.id != NEW.id
BEGIN
    INSERT INTO hosts_audit (refid, refcol, change, oldvalue, newvalue)
        VALUES (NEW.id, 'id', 'U', CAST(OLD.id as TEXT), CAST(NEW.id as TEXT));
END;

CREATE TRIGGER IF NOT EXISTS
    update_hosts_tenant
AFTER UPDATE ON
    hosts
FOR EACH ROW WHEN
    OLD.tenant != NEW.tenant
BEGIN
    INSERT INTO hosts_audit (refid, refcol, change, oldvalue, newvalue)
        VALUES (NEW.id, 'tenant', 'U', OLD.tenant, NEW.tenant);
END;

CREATE TRIGGER IF NOT EXISTS
    update_hosts_host
AFTER UPDATE ON
    hosts
FOR EACH ROW WHEN
    OLD.host != NEW.host
BEGIN
    INSERT INTO hosts_audit (refid, refcol, change, oldvalue, newvalue)
        VALUES (NEW.id, 'host', 'U', OLD.host, NEW.host);
END;

CREATE TRIGGER IF NOT EXISTS
    update_hosts_addr
AFTER UPDATE ON
    hosts
FOR EACH ROW WHEN
    OLD.addr != NEW.addr
BEGIN
    INSERT INTO hosts_audit (refid, refcol, change, oldvalue, newvalue)
        VALUES (NEW.id, 'addr', 'U', OLD.addr, NEW.addr);
END;

CREATE TRIGGER IF NOT EXISTS
    update_hosts_created
AFTER UPDATE ON
    hosts
FOR EACH ROW WHEN
    OLD.created != NEW.created
BEGIN
    INSERT INTO hosts_audit (refid, refcol, change, oldvalue, newvalue)
        VALUES (NEW.id, 'created', 'U', OLD.created, NEW.created);
END;


-- CREATE TABLE IF NOT EXISTS hosts
-- (
--     id                INTEGER PRIMARY KEY NOT NULL,
--     tenant            TEXT NOT NULL,
--     host              TEXT NOT NULL,
--     addr              TEXT NOT NULL,
--     created           TEXT NOT NULL,
--     updated           TEXT NOT NULL
-- ) STRICT;



