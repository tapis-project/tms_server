-- Initial TMS database schema.
-- Does not include foreign key definitions.

-- User Identity
-- 
-- Different user identities are referenced in this schema:
--  
--  1. tms_user_id - The user identity that TMS ensures has been validated by 
--        an IDP.
--  2. client_user_id - The user identity that TMS aquires from a client 
--        application on API calls initiated by the client.
--  3. host_account - the user's login account on a host.
--
-- The tms_user_id field is assigned during operations that do not include a
-- client application, such a when a user links one of their identities to a
-- host, establishes their MFA expiry or is registered as an administrator.
--
-- The client_user_id field is used when clients issue calls to TMS on behalf of 
-- their users.  These calls include requesting a new SSH key pair, delegating
-- access and making a reservation. 
--
-- Even though the tms_user_id and client_user_id are assigned in different 
-- ways at differnt times, various operations will depend on their values 
-- being the same.  For example, when a client requests a new key pair, TMS
-- will check that the tms_user_id in the user_hosts table matches the 
-- client_user_id in the request and in the delegations table.  In addtion,
-- the host_account in the user_hosts table must match the host_account in
-- the request.
--
-- Another way of thinking about tms_user_id and client_user_id is that they
-- are identities validated by some IDP.  For two identities to match--to 
-- represent the same user--they must have been validated by the same IDP.
-- The "tms_" and "client_" prefix on "user_id" simply indicates the component
-- that initiated the IDP authentication action.  
-- 
-- Hosts identify users by their host_accounts, which is the login account  
-- for a user.  No matter what identity a user authenticated to an IDP with, 
-- it's the host_account associated with that identity that is used to access
-- a target host.
--

PRAGMA foreign_keys = ON;

-- ---------------------------------------
-- tenants table
-- ---------------------------------------
-- This catalogs all TMS tenants.  Changing the tenant field
-- will cascade throughout the database, but for safety, tenant
-- records can only be deleted if no foreign key references 
-- to it exist.
CREATE TABLE IF NOT EXISTS tenants
(
    id            INTEGER PRIMARY KEY NOT NULL,
    tenant        TEXT NOT NULL,
    enabled       INTEGER NOT NULL CHECK (enabled IN (0, 1)),
    created       TEXT NOT NULL,
    updated       TEXT NOT NULL
) STRICT;

CREATE UNIQUE INDEX IF NOT EXISTS ten_tenant_idx ON tenants (tenant);

-- ---------------------------------------
-- clients table
-- ---------------------------------------
CREATE TABLE IF NOT EXISTS clients
(
    id            INTEGER PRIMARY KEY NOT NULL,
    tenant        TEXT NOT NULL,
    app_name      TEXT NOT NULL,
    app_version   TEXT NOT NULL,
    client_id     TEXT NOT NULL,
    client_secret TEXT NOT NULL,
    enabled       INTEGER NOT NULL CHECK (enabled IN (0, 1)),
    created       TEXT NOT NULL,
    updated       TEXT NOT NULL, 
    FOREIGN KEY(tenant) REFERENCES tenants(tenant) ON UPDATE CASCADE ON DELETE RESTRICT
) STRICT;

CREATE UNIQUE INDEX IF NOT EXISTS clts_app_idx ON clients (tenant, app_name, app_version);
CREATE UNIQUE INDEX IF NOT EXISTS clts_app_client_id_idx ON clients (tenant, client_id);
CREATE INDEX IF NOT EXISTS clts_updated_idx ON clients (updated);

-- ---------------------------------------
-- user_mfa table
-- ---------------------------------------
-- This table records when a user's MFA validation will expire.
CREATE TABLE IF NOT EXISTS user_mfa
(
    id                     INTEGER PRIMARY KEY NOT NULL,
    tenant                 TEXT NOT NULL,
    tms_user_id            TEXT NOT NULL,
    expires_at             TEXT NOT NULL,
    enabled                INTEGER NOT NULL CHECK (enabled IN (0, 1)),
    created                TEXT NOT NULL,
    updated                TEXT NOT NULL,
    FOREIGN KEY(tenant) REFERENCES tenants(tenant) ON UPDATE CASCADE ON DELETE RESTRICT
) STRICT; 

CREATE UNIQUE INDEX IF NOT EXISTS umfa_name_idx ON user_mfa (tenant, tms_user_id);
CREATE INDEX IF NOT EXISTS umfa_expires_idx ON user_mfa (expires_at);
CREATE INDEX IF NOT EXISTS umfa_updated_idx ON user_mfa (updated);

-- ---------------------------------------
-- user_hosts table
-- ---------------------------------------
-- If both tms_user_id and host_account are set to "*",
-- then all users have their identity linked on host.
CREATE TABLE IF NOT EXISTS user_hosts
(
    id                INTEGER PRIMARY KEY NOT NULL,
    tenant            TEXT NOT NULL,
    tms_user_id       TEXT NOT NULL,
    host              TEXT NOT NULL,
    host_account      TEXT NOT NULL,
    expires_at        TEXT NOT NULL,
    created           TEXT NOT NULL,
    updated           TEXT NOT NULL,
    FOREIGN KEY(tenant) REFERENCES tenants(tenant) ON UPDATE CASCADE ON DELETE RESTRICT,
    FOREIGN KEY(tenant, tms_user_id) REFERENCES user_mfa(tenant, tms_user_id) ON UPDATE CASCADE ON DELETE CASCADE
) STRICT;

CREATE UNIQUE INDEX IF NOT EXISTS uh_user_name_idx ON user_hosts (tenant, tms_user_id, host, host_account);
CREATE INDEX IF NOT EXISTS uhost_host_idx ON user_hosts (host);
CREATE INDEX IF NOT EXISTS uhost_host_account_idx ON user_hosts (host_account);
CREATE INDEX IF NOT EXISTS uhost_expires_idx ON user_hosts (expires_at);
CREATE INDEX IF NOT EXISTS uhost_updated_idx ON user_hosts (updated);

-- ---------------------------------------
-- delegations table
-- ---------------------------------------
-- If client_user_id "*", then the delegation applies to all users.
CREATE TABLE IF NOT EXISTS delegations
(
    id                INTEGER PRIMARY KEY NOT NULL,
    tenant            TEXT NOT NULL,
    client_id         TEXT NOT NULL,
    client_user_id    TEXT NOT NULL,
    expires_at        TEXT NOT NULL,
    created           TEXT NOT NULL,
    updated           TEXT NOT NULL,
    FOREIGN KEY(tenant) REFERENCES tenants(tenant) ON UPDATE CASCADE ON DELETE RESTRICT,
    FOREIGN KEY(tenant, client_user_id) REFERENCES user_mfa(tenant, tms_user_id) ON UPDATE CASCADE ON DELETE CASCADE,
    FOREIGN KEY(tenant, client_id) REFERENCES clients(tenant, client_id) ON UPDATE CASCADE ON DELETE CASCADE 
) STRICT;

CREATE UNIQUE INDEX IF NOT EXISTS delg_user_client_idx ON delegations (tenant, client_id, client_user_id);
CREATE INDEX IF NOT EXISTS delg_expires_idx ON delegations (expires_at);
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
    created                TEXT NOT NULL,
    updated                TEXT NOT NULL,
    FOREIGN KEY(tenant) REFERENCES tenants(tenant) ON UPDATE CASCADE ON DELETE RESTRICT,
    FOREIGN KEY(tenant, client_user_id) REFERENCES user_mfa(tenant, tms_user_id) ON UPDATE CASCADE ON DELETE CASCADE,
    FOREIGN KEY(tenant, client_id) REFERENCES clients(tenant, client_id) ON UPDATE CASCADE ON DELETE CASCADE,
    FOREIGN KEY(tenant, client_user_id, host, host_account) REFERENCES user_hosts(tenant, tms_user_id, host, host_account) ON UPDATE CASCADE ON DELETE CASCADE,
    FOREIGN KEY(tenant, client_id, client_user_id) REFERENCES delegations(tenant, client_id, client_user_id) ON UPDATE CASCADE ON DELETE CASCADE
) STRICT;

-- The unique index limits the use of a key to a single host, which makes it possible
-- for the foreign key of the reservations table to be defined.
CREATE UNIQUE INDEX IF NOT EXISTS pubk_fprint_idx ON pubkeys (public_key_fingerprint, host);
CREATE INDEX IF NOT EXISTS pubk_tenant_user_idx ON pubkeys (tenant, client_user_id, host, host_account);
CREATE INDEX IF NOT EXISTS pubk_expires_idx ON pubkeys (expires_at);
CREATE INDEX IF NOT EXISTS pubk_remaining_uses_idx ON pubkeys (remaining_uses);
CREATE INDEX IF NOT EXISTS pubk_updated_idx ON pubkeys (updated);

-- ---------------------------------------
-- reservations table
-- ---------------------------------------
-- Record a record for each tenant/client_id/client_user_id/host/fingerprint combination
-- in a single reservation (i.e., with the same resid).
CREATE TABLE IF NOT EXISTS reservations
(
    id                     INTEGER PRIMARY KEY NOT NULL,
    resid                  TEXT NOT NULL,
    tenant                 TEXT NOT NULL,
    client_id              TEXT NOT NULL,
    client_user_id         TEXT NOT NULL,
    host                   TEXT NOT NULL,
    public_key_fingerprint TEXT NOT NULL,
    expires_at             TEXT NOT NULL,
    created                TEXT NOT NULL,
    updated                TEXT NOT NULL,
    FOREIGN KEY(tenant) REFERENCES tenants(tenant) ON UPDATE CASCADE ON DELETE RESTRICT,
    FOREIGN KEY(tenant, client_user_id) REFERENCES user_mfa(tenant, tms_user_id) ON UPDATE CASCADE ON DELETE CASCADE,
    FOREIGN KEY(tenant, client_id) REFERENCES clients(tenant, client_id) ON UPDATE CASCADE ON DELETE CASCADE,
    FOREIGN KEY(public_key_fingerprint, host) REFERENCES pubkeys(public_key_fingerprint, host) ON UPDATE CASCADE ON DELETE CASCADE
) STRICT;

CREATE UNIQUE INDEX IF NOT EXISTS res_unique_tuple_idx ON reservations (resid, tenant, client_id, client_user_id, host, public_key_fingerprint);
CREATE INDEX IF NOT EXISTS res_resid_idx ON reservations (resid);
CREATE INDEX IF NOT EXISTS res_tenant_user_idx ON reservations (tenant, client_user_id); 
CREATE INDEX IF NOT EXISTS res_client_idx ON reservations (client_id);
CREATE INDEX IF NOT EXISTS res_expires_idx ON reservations (expires_at);
CREATE INDEX IF NOT EXISTS res_updated_idx ON reservations (updated);

-- ---------------------------------------
-- admin table
-- ---------------------------------------
-- ENUMS not supported and column checks cannot be altered,
-- so valid privileges enforced in application code.  Note that
-- the admin_user and tms_user_id's defined in other tables
-- (such as user_mfa, user_hosts, etc.) are in different namespaces.
-- The admin_user/admin_secret are used for authn/authz on 
-- administrative APIs only; tms_user_id's are actual users of TMS. 
CREATE TABLE IF NOT EXISTS admin
(
    id                INTEGER PRIMARY KEY NOT NULL,
    tenant            TEXT NOT NULL,
    admin_user        TEXT NOT NULL,
    admin_secret      TEXT NOT NULL,
    privilege         TEXT NOT NULL,
    created           TEXT NOT NULL,
    updated           TEXT NOT NULL,
    FOREIGN KEY(tenant) REFERENCES tenants(tenant) ON UPDATE CASCADE ON DELETE RESTRICT
) STRICT;

CREATE UNIQUE INDEX IF NOT EXISTS adm_user_idx ON admin (tenant, admin_user);
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
    tenant            TEXT NOT NULL,
    host              TEXT NOT NULL,
    addr              TEXT NOT NULL,
    created           TEXT NOT NULL,
    updated           TEXT NOT NULL,
    FOREIGN KEY(tenant) REFERENCES tenants(tenant) ON UPDATE CASCADE ON DELETE RESTRICT
) STRICT;

CREATE UNIQUE INDEX IF NOT EXISTS host_host_addr_idx ON hosts (tenant, host, addr);
CREATE INDEX IF NOT EXISTS host_updated_idx ON hosts (updated);
