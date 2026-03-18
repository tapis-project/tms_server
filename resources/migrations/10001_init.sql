-- Initial TMS database schema.

-- User Identity
-- 
-- Different user identities are referenced in this schema:
--  
--  1. tms_user_id - User identity that TMS ensures has been validated by an IDP.
--  2. client_user_id - User identity TMS acquires from client application on API calls initiated by client.
--  3. host_account - User's login account on a host.
--
-- The tms_user_id field is assigned during operations that do not include a client application, such a when a user
-- links one of their identities to a host, establishes their MFA expiry or is registered as an administrator.
--
-- The client_user_id field is used when clients issue calls to TMS on behalf of their users.
-- These calls include requesting a new SSH key pair, delegating access and making a reservation.
--
-- Even though the tms_user_id and client_user_id are assigned in different at different times, various operations
-- will depend on their values being the same. For example, when a client requests a new key pair, TMS will check that
-- the tms_user_id in the user_hosts table matches the client_user_id in the request and in the delegations table.
-- In addition, the host_account in the user_hosts table must match the host_account in the request.
--
-- Another way of thinking about tms_user_id and client_user_id is that they are identities validated by some IDP.
-- For two identities to match, i.e. to represent the same user, they must have been validated by the same IDP.
-- The "tms_" and "client_" prefix on "user_id" simply indicates which component initiated the IDP auth action.
-- 
-- Hosts identify users by their host_accounts, which is the login account for a user.
-- No matter what identity a user authenticated to an IDP with, it is the host_account associated with that identity
-- that is used to access a target host.

-- Create the schema and set the search path
CREATE SCHEMA IF NOT EXISTS tms AUTHORIZATION tms;
ALTER ROLE tms SET search_path = 'tms';
SET search_path TO tms;

-- ---------------------------------------
-- tenants table
-- ---------------------------------------
-- This catalogs all TMS tenants. Changing the tenant field will cascade throughout the database, but for safety,
-- tenant records can only be deleted if no foreign key references to it exist.
CREATE TABLE IF NOT EXISTS tenants
(
  id SERIAL PRIMARY KEY,
  tenant  TEXT NOT NULL UNIQUE,
  enabled BOOLEAN NOT NULL,
  created TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT (NOW() AT TIME ZONE 'utc'),
  updated TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT (NOW() AT TIME ZONE 'utc')
);
ALTER TABLE tenants OWNER TO tms;

-- ---------------------------------------
-- clients table
-- ---------------------------------------
CREATE TABLE IF NOT EXISTS clients
(
    id            INTEGER PRIMARY KEY NOT NULL,
    tenant        TEXT REFERENCES tenants(tenant) ON UPDATE CASCADE ON DELETE RESTRICT,
    app_name      TEXT NOT NULL,
    app_version   TEXT NOT NULL,
    client_id     TEXT NOT NULL,
    client_secret TEXT NOT NULL,
    enabled       BOOLEAN NOT NULL,
    created       TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT (NOW() AT TIME ZONE 'utc'),
    updated       TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT (NOW() AT TIME ZONE 'utc'),
    UNIQUE (tenant, app_name, app_version)
);
ALTER TABLE clients OWNER TO tms;
CREATE UNIQUE INDEX IF NOT EXISTS clients_tenant_client_idx ON clients (tenant, client_id);

-- ---------------------------------------
-- user_mfa table
-- ---------------------------------------
-- This table records when a user's MFA validation will expire.
CREATE TABLE IF NOT EXISTS user_mfa
(
    id                     INTEGER PRIMARY KEY NOT NULL,
    tenant                 TEXT REFERENCES tenants(tenant) ON UPDATE CASCADE ON DELETE RESTRICT,
    tms_user_id            TEXT NOT NULL,
    expires_at             TEXT NOT NULL,
    enabled                BOOLEAN NOT NULL,
    created                TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT (NOW() AT TIME ZONE 'utc'),
    updated                TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT (NOW() AT TIME ZONE 'utc'),
    UNIQUE (tenant, tms_user_id)
);
ALTER TABLE user_mfa OWNER TO tms;

-- ---------------------------------------
-- user_hosts table
-- ---------------------------------------
-- If both tms_user_id and host_account are set to "*",
-- then all users have their identity linked on host.
CREATE TABLE IF NOT EXISTS user_hosts
(
    id                INTEGER PRIMARY KEY NOT NULL,
    tenant            TEXT REFERENCES tenants(tenant) ON UPDATE CASCADE ON DELETE RESTRICT,
    tms_user_id       TEXT NOT NULL,
    host              TEXT NOT NULL,
    host_account      TEXT NOT NULL,
    expires_at        TEXT NOT NULL,
    created           TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT (NOW() AT TIME ZONE 'utc'),
    updated           TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT (NOW() AT TIME ZONE 'utc'),
    UNIQUE (tenant, tms_user_id, host, host_account),
    FOREIGN KEY(tenant, tms_user_id) REFERENCES user_mfa(tenant, tms_user_id) ON UPDATE CASCADE ON DELETE CASCADE
);
ALTER TABLE user_hosts OWNER TO tms;

-- ---------------------------------------
-- delegations table
-- ---------------------------------------
-- If client_user_id "*", then the delegation applies to all users.
CREATE TABLE IF NOT EXISTS delegations
(
    id                INTEGER PRIMARY KEY NOT NULL,
    tenant            TEXT REFERENCES tenants(tenant) ON UPDATE CASCADE ON DELETE RESTRICT,
    client_id         TEXT NOT NULL,
    client_user_id    TEXT NOT NULL,
    expires_at        TEXT NOT NULL,
    created           TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT (NOW() AT TIME ZONE 'utc'),
    updated           TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT (NOW() AT TIME ZONE 'utc'),
    UNIQUE (tenant, client_id, client_user_id),
    FOREIGN KEY(tenant, client_user_id) REFERENCES user_mfa(tenant, tms_user_id) ON UPDATE CASCADE ON DELETE CASCADE,
    FOREIGN KEY(tenant, client_id) REFERENCES clients(tenant, client_id) ON UPDATE CASCADE ON DELETE CASCADE
);
ALTER TABLE delegations OWNER TO tms;

CREATE UNIQUE INDEX IF NOT EXISTS delg_user_client_idx ON delegations (tenant, client_id, client_user_id);

-- ---------------------------------------
-- pubkeys table
-- ---------------------------------------
-- Public keys tie users and hosts together. The same public key can be used on multiple hosts, though that practice
--   is discouraged. A user can have multiple keys defined for a host, but application code must ensure that a key can
--   only be used by a single user in a single tenant.
-- Note that UNIQUE (public_key_fingerprint, host) limits the use of a key to a single host, which makes it possible
--   for the foreign key of the reservations table to be defined.
CREATE TABLE IF NOT EXISTS pubkeys
(
    id                     INTEGER PRIMARY KEY NOT NULL,
    tenant                 TEXT REFERENCES tenants(tenant) ON UPDATE CASCADE ON DELETE RESTRICT,
    client_id              TEXT NOT NULL,
    client_user_id         TEXT NOT NULL,
    host                   TEXT NOT NULL,
    host_account           TEXT NOT NULL,
    public_key_fingerprint TEXT NOT NULL,
    public_key             TEXT NOT NULL,
    key_type               TEXT NOT NULL,
    key_bits               INT  NOT NULL,
    max_uses               INT  NOT NULL CHECK (max_uses >= 0),
    remaining_uses         INT  NOT NULL CHECK (remaining_uses >= 0),
    initial_ttl_minutes    INT  NOT NULL CHECK (initial_ttl_minutes >= 0),
    expires_at             TEXT NOT NULL,
    created                TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT (NOW() AT TIME ZONE 'utc'),
    updated                TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT (NOW() AT TIME ZONE 'utc'),
    UNIQUE (public_key_fingerprint, host),
    FOREIGN KEY(tenant, client_user_id) REFERENCES user_mfa(tenant, tms_user_id) ON UPDATE CASCADE ON DELETE CASCADE,
    FOREIGN KEY(tenant, client_id) REFERENCES clients(tenant, client_id) ON UPDATE CASCADE ON DELETE CASCADE,
    FOREIGN KEY(tenant, client_user_id, host, host_account) REFERENCES user_hosts(tenant, tms_user_id, host, host_account) ON UPDATE CASCADE ON DELETE CASCADE,
    FOREIGN KEY(tenant, client_id, client_user_id) REFERENCES delegations(tenant, client_id, client_user_id) ON UPDATE CASCADE ON DELETE CASCADE
);
ALTER TABLE pubkeys OWNER TO tms;

-- ---------------------------------------
-- reservations table
-- ---------------------------------------
-- A record for each tenant/client_id/client_user_id/host/fingerprint combination in a single reservation.
--   In other words, resid is unique.
CREATE TABLE IF NOT EXISTS reservations
(
    id                     INTEGER PRIMARY KEY NOT NULL,
    resid                  TEXT NOT NULL UNIQUE,
    parent_resid           TEXT NOT NULL,
    tenant                 TEXT REFERENCES tenants(tenant) ON UPDATE CASCADE ON DELETE RESTRICT,
    client_id              TEXT NOT NULL,
    client_user_id         TEXT NOT NULL,
    host                   TEXT NOT NULL,
    public_key_fingerprint TEXT NOT NULL,
    expires_at             TEXT NOT NULL,
    created                TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT (NOW() AT TIME ZONE 'utc'),
    updated                TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT (NOW() AT TIME ZONE 'utc'),
    FOREIGN KEY(tenant, client_user_id) REFERENCES user_mfa(tenant, tms_user_id) ON UPDATE CASCADE ON DELETE CASCADE,
    FOREIGN KEY(tenant, client_id) REFERENCES clients(tenant, client_id) ON UPDATE CASCADE ON DELETE CASCADE,
    FOREIGN KEY(public_key_fingerprint, host) REFERENCES pubkeys(public_key_fingerprint, host) ON UPDATE CASCADE ON DELETE CASCADE
);
ALTER TABLE reservations OWNER TO tms;

-- ---------------------------------------
-- admin table
-- ---------------------------------------
-- Note that the admin_user here and the tms_user_id in other tables (such as user_mfa, user_hosts, etc.) are in
-- different namespaces. The admin_user/admin_secret are used for authn/authz on administrative APIs only.
-- tms_user_id's are actual users of TMS.
CREATE TABLE IF NOT EXISTS admin
(
    id                INTEGER PRIMARY KEY NOT NULL,
    tenant            TEXT REFERENCES tenants(tenant) ON UPDATE CASCADE ON DELETE RESTRICT,
    admin_user        TEXT NOT NULL,
    admin_secret      TEXT NOT NULL,
    privilege         TEXT NOT NULL,
    created           TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT (NOW() AT TIME ZONE 'utc'),
    updated           TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT (NOW() AT TIME ZONE 'utc'),
    UNIQUE (tenant, admin_user)
);
ALTER TABLE admin OWNER TO tms;

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
    tenant            TEXT REFERENCES tenants(tenant) ON UPDATE CASCADE ON DELETE RESTRICT,
    host              TEXT NOT NULL,
    addr              TEXT NOT NULL,
    created           TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT (NOW() AT TIME ZONE 'utc'),
    updated           TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT (NOW() AT TIME ZONE 'utc'),
    UNIQUE (tenant, host, addr)
);
ALTER TABLE hosts OWNER TO tms;