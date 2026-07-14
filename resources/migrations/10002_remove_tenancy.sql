--
-- Remove tenants table and related columns
--

-- =============================================================
-- Update all foreign key relations involving tenants table
-- =============================================================
------------------------------------------
-- Drop single column FKs
------------------------------------------
ALTER TABLE hosts DROP CONSTRAINT hosts_tenant_fkey;
ALTER TABLE admin DROP CONSTRAINT admin_tenant_fkey;
ALTER TABLE reservations DROP CONSTRAINT reservations_tenant_fkey;
ALTER TABLE pubkeys DROP CONSTRAINT pubkeys_tenant_fkey;
ALTER TABLE delegations DROP CONSTRAINT delegations_tenant_fkey;
ALTER TABLE user_hosts DROP CONSTRAINT user_hosts_tenant_fkey;
ALTER TABLE user_mfa DROP CONSTRAINT user_mfa_tenant_fkey;
ALTER TABLE clients DROP CONSTRAINT clients_tenant_fkey;

------------------------------------------------------------------------
-- Add updated unique constrains.
-- NOTE: Some of these are needed for the multi-column FK updates.
------------------------------------------------------------------------
ALTER TABLE hosts ADD CONSTRAINT hosts_host_addr_key UNIQUE (host, addr);
ALTER TABLE delegations ADD CONSTRAINT delegations_client_id_client_user_id_key UNIQUE (client_id,client_user_id);
ALTER TABLE user_hosts ADD CONSTRAINT user_hosts_tms_user_id_host_host_account_key UNIQUE (tms_user_id, host, host_account);
ALTER TABLE user_mfa ADD CONSTRAINT user_mfa_tms_user_id_key UNIQUE (tms_user_id);
ALTER TABLE clients ADD CONSTRAINT clients_client_id UNIQUE (client_id);
ALTER TABLE clients ADD CONSTRAINT clients_app_name_app_version_key UNIQUE (app_name,app_version);

------------------------------------------
-- Update multi-column FKs
------------------------------------------
ALTER TABLE reservations DROP CONSTRAINT reservations_tenant_client_id_fkey;
ALTER TABLE reservations ADD FOREIGN KEY(client_id) REFERENCES clients(client_id) ON UPDATE CASCADE ON DELETE CASCADE;
ALTER TABLE reservations DROP CONSTRAINT reservations_tenant_client_user_id_fkey;
ALTER TABLE reservations ADD FOREIGN KEY(client_user_id) REFERENCES user_mfa(tms_user_id) ON UPDATE CASCADE ON DELETE CASCADE;

ALTER TABLE pubkeys DROP CONSTRAINT pubkeys_tenant_client_user_id_host_host_account_fkey;
ALTER TABLE pubkeys ADD FOREIGN KEY(client_user_id, host, host_account) REFERENCES user_hosts(tms_user_id, host, host_account) ON UPDATE CASCADE ON DELETE CASCADE;
ALTER TABLE pubkeys DROP CONSTRAINT pubkeys_tenant_client_user_id_fkey;
ALTER TABLE pubkeys ADD FOREIGN KEY(client_user_id) REFERENCES user_mfa(tms_user_id) ON UPDATE CASCADE ON DELETE CASCADE;
ALTER TABLE pubkeys DROP CONSTRAINT pubkeys_tenant_client_id_fkey;
ALTER TABLE pubkeys ADD FOREIGN KEY(client_id) REFERENCES clients(client_id) ON UPDATE CASCADE ON DELETE CASCADE;
ALTER TABLE pubkeys DROP CONSTRAINT pubkeys_tenant_client_id_client_user_id_fkey;
ALTER TABLE pubkeys ADD FOREIGN KEY(client_id, client_user_id) REFERENCES delegations(client_id, client_user_id) ON UPDATE CASCADE ON DELETE CASCADE;

ALTER TABLE delegations DROP CONSTRAINT delegations_tenant_client_id_fkey;
ALTER TABLE delegations ADD FOREIGN KEY(client_id) REFERENCES clients(client_id) ON UPDATE CASCADE ON DELETE CASCADE;
ALTER TABLE delegations DROP CONSTRAINT delegations_tenant_client_user_id_fkey;
ALTER TABLE delegations ADD FOREIGN KEY(client_user_id) REFERENCES user_mfa(tms_user_id) ON UPDATE CASCADE ON DELETE CASCADE;

ALTER TABLE user_hosts DROP CONSTRAINT user_hosts_tenant_tms_user_id_fkey;
ALTER TABLE user_hosts ADD FOREIGN KEY(tms_user_id) REFERENCES user_mfa(tms_user_id) ON UPDATE CASCADE ON DELETE CASCADE;

------------------------------------------
-- Remove old unique constraints
------------------------------------------
ALTER TABLE hosts DROP CONSTRAINT hosts_tenant_host_addr_key;
ALTER TABLE admin DROP CONSTRAINT admin_tenant_admin_user_key;
ALTER TABLE delegations DROP CONSTRAINT delegations_tenant_client_id_client_user_id_key;
ALTER TABLE user_hosts DROP CONSTRAINT user_hosts_tenant_tms_user_id_host_host_account_key;
ALTER TABLE user_mfa DROP CONSTRAINT user_mfa_tenant_tms_user_id_key;
ALTER TABLE clients DROP CONSTRAINT clients_tenant_app_name_app_version_key;

------------------------------------------
-- Update indexes
------------------------------------------
DROP INDEX IF EXISTS delg_user_client_idx;
CREATE UNIQUE INDEX IF NOT EXISTS delg_user_client_idx ON delegations (client_id, client_user_id);
DROP INDEX IF EXISTS clients_tenant_client_idx;
CREATE UNIQUE INDEX IF NOT EXISTS clients_tenant_client_idx ON clients (client_id);

------------------------------------------
-- Only one admin user for the "default" tenant.
-- No longer have tenancy, so remove any other admins
------------------------------------------
DELETE FROM admin where tenant <> 'default';

------------------------------------------
-- Remove tenant column from tables
------------------------------------------
ALTER TABLE hosts DROP COLUMN tenant;
ALTER TABLE admin DROP COLUMN tenant;
ALTER TABLE reservations DROP COLUMN tenant;
ALTER TABLE pubkeys DROP COLUMN tenant;
ALTER TABLE delegations DROP COLUMN tenant;
ALTER TABLE user_hosts DROP COLUMN tenant;
ALTER TABLE user_mfa DROP COLUMN tenant;
ALTER TABLE clients DROP COLUMN tenant;


------------------------------------------
-- Last action is to drop the tenants table
------------------------------------------
DROP TABLE IF EXISTS tenants;
