-- Initial TMS database schema.

-- Add tables needed for TMS Portal
--   TMS portal and server will use the same DB
-- 

-- ---------------------------------------
-- Identity Provider tables
-- ---------------------------------------
-- All cloud and resource IdPs
-- Example cloud IdPs: UT Austin, UC San Diego, Univ of Pittsburgh, ACCESS
--    - All through Globus via CILogin
-- Example resource providers: TACC, SDSC, PSC
CREATE TABLE IF NOT EXISTS identity_providers
(
    uuid                  UUID PRIMARY KEY  NOT NULL DEFAULT gen_random_uuid(),
    id                    TEXT              NOT NULL,
    name                  TEXT              NOT NULL,
    client_id             TEXT              NOT NULL,
    client_secret         TEXT              NOT NULL,
    identity_redirect_url TEXT              NOT NULL,
    oauth2_token_url      TEXT              NOT NULL,
    oauth2_jwks_url       TEXT,
    oauth2_public_key     TEXT,
    oidc_user_info_url    TEXT,
    scope                 TEXT,
    provider_type         TEXT              NOT NULL,
    supports_login        BOOLEAN           NOT NULL DEFAULT false,
    supports_resources    BOOLEAN           NOT NULL DEFAULT false,
    created               TIMESTAMPTZ       NOT NULL DEFAULT (NOW() AT TIME ZONE 'utc'),
    updated               TIMESTAMPTZ       NOT NULL DEFAULT (NOW() AT TIME ZONE 'utc'),
    UNIQUE (id)
);
ALTER TABLE identity_providers OWNER TO tms;

-- Identity provider types
CREATE TABLE IF NOT EXISTS identity_provider_types
(
    provider_type TEXT PRIMARY KEY            NOT NULL,
    created               TIMESTAMPTZ       NOT NULL DEFAULT (NOW() AT TIME ZONE 'utc'),
    updated               TIMESTAMPTZ       NOT NULL DEFAULT (NOW() AT TIME ZONE 'utc')
);
ALTER TABLE identity_provider_types OWNER TO tms;

-- Insert hard-coded types
INSERT INTO identity_provider_types (provider_type)
VALUES ('globus');
INSERT INTO identity_provider_types (provider_type)
VALUES ('tacc_tapis');

ALTER TABLE identity_providers
    ADD CONSTRAINT fk_provider FOREIGN KEY (provider_type) REFERENCES identity_provider_types (provider_type);

-- ---------------------------------------
-- keys table
-- ---------------------------------------
-- ???
CREATE TABLE IF NOT EXISTS keys
(
    kid             TEXT PRIMARY KEY            NOT NULL,
    jwt_public_key  TEXT                        NOT NULL,
    jwt_private_key TEXT                        NOT NULL,
    created               TIMESTAMPTZ       NOT NULL DEFAULT (NOW() AT TIME ZONE 'utc'),
    updated               TIMESTAMPTZ       NOT NULL DEFAULT (NOW() AT TIME ZONE 'utc')
);
ALTER TABLE keys OWNER TO tms;

-- ---------------------------------------
-- prtl_clients table
-- ---------------------------------------
-- ???
CREATE TABLE IF NOT EXISTS prtl_clients
(
    id      TEXT PRIMARY KEY            NOT NULL,
    name    TEXT                        NOT NULL,
    secret  TEXT                        NOT NULL,
    kid     TEXT                        NOT NULL,
    created               TIMESTAMPTZ       NOT NULL DEFAULT (NOW() AT TIME ZONE 'utc'),
    updated               TIMESTAMPTZ       NOT NULL DEFAULT (NOW() AT TIME ZONE 'utc'),
    constraint fk_kid FOREIGN KEY (kid) REFERENCES keys (kid)
);
ALTER TABLE prtl_clients OWNER TO tms;

-- CREATE TABLE IF NOT EXISTS clients
-- (
--     id SERIAL PRIMARY KEY,
--     tenant        TEXT REFERENCES tenants(tenant) ON UPDATE CASCADE ON DELETE RESTRICT,
--     app_name      TEXT NOT NULL,
--     app_version   TEXT NOT NULL,
--     client_id     TEXT NOT NULL,
--     client_secret TEXT NOT NULL,
--     enabled       BOOLEAN NOT NULL,
--     created       TIMESTAMPTZ NOT NULL DEFAULT (NOW() AT TIME ZONE 'utc'),
--     updated       TIMESTAMPTZ NOT NULL DEFAULT (NOW() AT TIME ZONE 'utc'),
--     UNIQUE (tenant, app_name, app_version)
-- );
-- ALTER TABLE clients OWNER TO tms;
-- CREATE UNIQUE INDEX IF NOT EXISTS clients_tenant_client_idx ON clients (tenant, client_id);

-- ---------------------------------------
-- configuration table
-- ---------------------------------------
-- ???
CREATE TABLE IF NOT EXISTS configuration
(
    config_name  TEXT PRIMARY KEY            NOT NULL,
    config_value JSONB                       NOT NULL,
    created               TIMESTAMPTZ       NOT NULL DEFAULT (NOW() AT TIME ZONE 'utc'),
    updated               TIMESTAMPTZ       NOT NULL DEFAULT (NOW() AT TIME ZONE 'utc')
);
ALTER TABLE configuration OWNER TO tms;

-- ---------------------------------------
-- allowed_redirects table
-- ---------------------------------------
-- Allowable re-directs for each client
CREATE TABLE IF NOT EXISTS allowed_redirects
(
    uri       TEXT                        NOT NULL,
    client_id TEXT                        NOT NULL,
    created               TIMESTAMPTZ       NOT NULL DEFAULT (NOW() AT TIME ZONE 'utc'),
    updated               TIMESTAMPTZ       NOT NULL DEFAULT (NOW() AT TIME ZONE 'utc'),
    constraint fk_client_id FOREIGN KEY (client_id) REFERENCES clients (id)
);
ALTER TABLE allowed_redirects OWNER TO tms;

-- ---------------------------------------
-- resource_provider_account_logins table
-- ---------------------------------------
-- TODO ???
-- ---------------------------------------
-- user_mfa table
-- ---------------------------------------

-- TODO Merge with below table? Or create new table and migrate old records?
-- - This table records when a user's MFA validation will expire.
-- CREATE TABLE IF NOT EXISTS user_mfa
-- (
--     id                     SERIAL PRIMARY KEY,
--     tenant                 TEXT REFERENCES tenants(tenant) ON UPDATE CASCADE ON DELETE RESTRICT,
--     tms_user_id            TEXT NOT NULL,
--     expires_at             TIMESTAMPTZ NOT NULL DEFAULT (NOW() AT TIME ZONE 'utc'),
--     enabled                BOOLEAN NOT NULL,
--     created                TIMESTAMPTZ NOT NULL DEFAULT (NOW() AT TIME ZONE 'utc'),
--     updated                TIMESTAMPTZ NOT NULL DEFAULT (NOW() AT TIME ZONE 'utc'),
--     UNIQUE (tenant, tms_user_id)
-- );

-- TODO This table records when a user's MFA validation will expire.
-- CREATE TABLE IF NOT EXISTS user_mfa
-- changed name because it's not really user_mfa.  I don't have strong feelings about what we call it though.
CREATE TABLE IF NOT EXISTS resource_provider_account_logins
(
    id                          SERIAL PRIMARY KEY,
    tms_identity                 TEXT NOT NULL,
    resource_provider_account   TEXT NOT NULL,
    resource_provider_uuid      UUID NOT NULL,
    last_login                  TIMESTAMPTZ NOT NULL DEFAULT (NOW() AT TIME ZONE 'utc'),
    enabled                     BOOLEAN NOT NULL,
    created                     TIMESTAMPTZ NOT NULL DEFAULT (NOW() AT TIME ZONE 'utc'),
    updated                     TIMESTAMPTZ NOT NULL DEFAULT (NOW() AT TIME ZONE 'utc'),
    UNIQUE (tms_identity, resource_provider_uuid, resource_provider_account),
    FOREIGN KEY(resource_provider_uuid) REFERENCES identity_providers(uuid)
    );
ALTER TABLE resource_provider_account_logins OWNER TO tms;
