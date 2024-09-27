#![forbid(unsafe_code)]

use anyhow::{Result, anyhow};
use log::{info, warn};
use std::io::{self, Write};
use sqlx::Row;

use futures::executor::block_on;
use crate::utils::tms_utils::{timestamp_utc, timestamp_utc_secs_to_str, timestamp_str_to_datetime, 
                              create_hex_secret, hash_hex_secret, MAX_TMS_UTC};
use crate::utils::db_statements::{INSERT_DELEGATIONS, INSERT_STD_TENANTS, INSERT_USER_HOSTS, INSERT_USER_MFA};
use crate::utils::config::{DEFAULT_TENANT, TEST_TENANT, SQLITE_TRUE, DEFAULT_ADMIN_ID, PERM_ADMIN, TMS_ARGS};
use log::error;

use crate::RUNTIME_CTX;

use super::db_statements::{INSERT_ADMIN, INSERT_CLIENTS, GET_USER_MFA_ACTIVE, GET_USER_HOST_ACTIVE,
                           GET_DELEGATION_ACTIVE};

// ---------------------------------------------------------------------------
// create_std_tenants:
// ---------------------------------------------------------------------------
/** This method should only be called when the --install option is specified.  
 * It's a no-op if called during regular execution.
 */
pub async fn create_std_tenants() -> Result<u64> {
    // Guard against repeated initialization of standard tenants and admins.
    if !TMS_ARGS.install {
        return Ok(0);
    }

    // Get the timestamp string.
    let now = timestamp_utc();
    let current_ts = timestamp_utc_secs_to_str(now);

    // Get a connection to the db and start a transaction.
    let mut tx = RUNTIME_CTX.db.begin().await?;

    // -------- Insert the two standard tenants.
    let dft_result = sqlx::query(INSERT_STD_TENANTS)
        .bind(DEFAULT_TENANT)
        .bind(SQLITE_TRUE)
        .bind(&current_ts)
        .bind(&current_ts)
        .execute(&mut *tx)
        .await?;

    let tst_result = sqlx::query(INSERT_STD_TENANTS)
        .bind(TEST_TENANT)
        .bind(SQLITE_TRUE)
        .bind(&current_ts)
        .bind(&current_ts)
        .execute(&mut *tx)
        .await?;

    // Create admin user ids.
    let dft_key_str = create_hex_secret();
    let dft_key_hash = hash_hex_secret(&dft_key_str);
    let _dft_admin_result = sqlx::query(INSERT_ADMIN)
        .bind(DEFAULT_TENANT)
        .bind(DEFAULT_ADMIN_ID)
        .bind(&dft_key_hash)
        .bind(PERM_ADMIN)
        .bind(&current_ts)
        .bind(&current_ts)
        .execute(&mut *tx)
        .await?;

    let tst_key_str = create_hex_secret();
    let tst_key_hash = hash_hex_secret(&tst_key_str);
    let _tst_admin_result = sqlx::query(INSERT_ADMIN)
        .bind(TEST_TENANT)
        .bind(DEFAULT_ADMIN_ID)
        .bind(&tst_key_hash)
        .bind(PERM_ADMIN)
        .bind(&current_ts)
        .bind(&current_ts)
        .execute(&mut *tx)
        .await?;

    // Commit the transaction.
    tx.commit().await?;

    // --- MOST IMPORTANT ---
    // One time printout of the admin secrets for the two tenants.
    print_admin_secret_message(&dft_key_str, &tst_key_str)?; 

    // Return the number of tenant insertions that took place.
    Ok(dft_result.rows_affected() + tst_result.rows_affected())
}

// ---------------------------------------------------------------------------
// print_admin_secret_message:
// ---------------------------------------------------------------------------
/** Print one-time message to stdout that contains the admin_user and admin_secret
 * for the two standard tenents.  This only happens when the --install option was
 * specified and this program terminates after installation with the secret 
 * information visible to user.
 */
fn print_admin_secret_message(dft_key_str: &String, tst_key_str: &String) -> Result<()> {
    // Compile time literal concatenation.
    let prefix = concat!(
        "\n***************************************************************************",
        "\n***************************************************************************",
        "\n**** Below are the administrator user IDs and passwords for the        ****",
        "\n**** standard tenants created at installation time.  The passwords are ****",
        "\n**** NOT saved by TMS, only hashes of them are saved.  Please store    ****",
        "\n**** the passwords permanently in a safe place accessible to TMS       ****",
        "\n**** administrators.                                                   ****",
        "\n****                                                                   ****",
        "\n****        THIS IS THE ONLY TIME THESE PASSWORDS ARE SHOWN.           ****",
        "\n****                                                                   ****",
        "\n****      THE PASSWORDS ARE NOT RECOVERABLE IF THEY ARE LOST!          ****",
        "\n****                                                                   ****");

    // Add the runtime suffix.
    let msg = prefix.to_string() +     
        "\n**** Tenant: " + DEFAULT_TENANT + "                                                   ****" +
        "\n**** Administrator ID: " + DEFAULT_ADMIN_ID + "                                         ****" +
        "\n**** Password: " + dft_key_str + "        ****" +
        "\n****                                                                   ****" +
        "\n**** Tenant: " + TEST_TENANT + "                                                      ****" +
        "\n**** Administrator ID: " + DEFAULT_ADMIN_ID + "                                         ****" +
        "\n**** Password: " + tst_key_str + "        ****" +
        "\n****                                                                   ****" +
        "\n***************************************************************************" +
        "\n***************************************************************************\n\n";

    // Write the one-time message to the terminal.
    io::stdout().write_all(msg.as_bytes())?;   
    Ok(())
}

// ---------------------------------------------------------------------------
// check_test_data:
// ---------------------------------------------------------------------------
pub fn check_test_data() {

    // Assume we are initializing for the first time and need
    // to populate the test tenant with some dummy data.
    match block_on(create_test_data()) {
        Ok(b) => {
            if b {info!("Test records inserted into test tenant.");} 
        }
        Err(e) => {
            warn!("****** Ignoring error while inserting test records into test tenant: {}", e);
        }
    };
}

// ---------------------------------------------------------------------------
// create_test_data:
// ---------------------------------------------------------------------------
/** This function either experiences an error or returns true (false is never returned). */
async fn create_test_data() -> Result<bool> {
    // Constants used locally.
    const TEST_APP: &str = "testapp1";
    const TEST_APP_VERS: &str = "1.0";
    const TEST_CLIENT: &str = "testclient1";
    let   test_secret: String = hash_hex_secret(&"secret1".to_string());
    const TEST_USER: &str = "testuser1";
    const TEST_HOST: &str = "testhost1";
    const TEST_HOST_ACCOUNT: &str = "testhostaccount1";

    // Get the timestamp string.
    let now = timestamp_utc();
    let current_ts = timestamp_utc_secs_to_str(now);

    // Get a connection to the db and start a transaction.
    let mut tx = RUNTIME_CTX.db.begin().await?;

    // -------- Populate clients
    sqlx::query(INSERT_CLIENTS)
        .bind(TEST_TENANT)
        .bind(TEST_APP)
        .bind(TEST_APP_VERS)
        .bind(TEST_CLIENT)
        .bind(test_secret)
        .bind(SQLITE_TRUE)
        .bind(&current_ts)
        .bind(&current_ts)
        .execute(&mut *tx)
        .await?;

    // -------- Populate user_mfa
    sqlx::query(INSERT_USER_MFA)
        .bind(TEST_TENANT)
        .bind(TEST_USER)
        .bind(MAX_TMS_UTC)
        .bind(SQLITE_TRUE)
        .bind(&current_ts)
        .bind(&current_ts)
        .execute(&mut *tx)
        .await?;

        // -------- Populate user_hosts
    sqlx::query(INSERT_USER_HOSTS)
        .bind(TEST_TENANT)
        .bind(TEST_USER)
        .bind(TEST_HOST)
        .bind(TEST_HOST_ACCOUNT)
        .bind(MAX_TMS_UTC)
        .bind(&current_ts)
        .bind(&current_ts)
        .execute(&mut *tx)
        .await?;

    // -------- Populate delegations
    sqlx::query(INSERT_DELEGATIONS)
        .bind(TEST_TENANT)
        .bind(TEST_CLIENT)
        .bind(TEST_USER)
        .bind(MAX_TMS_UTC)
        .bind(&current_ts)
        .bind(&current_ts)
        .execute(&mut *tx)
        .await?;

    // Commit the transaction.
    tx.commit().await?;
    
    Ok(true)
}

// ---------------------------------------------------------------------------
// check_pubkey_dependencies:
// ---------------------------------------------------------------------------
/** When creating a public key or a reservation on a public key we must check
 * that the user's MFA, user/host mapping and client delegation are currently 
 * active.  Active means that the records exist in their respective tables, are
 * enabled and have not expired.
 * 
 * We return as soon as we encounter any dependency that cannot be fulfilled or
 * any other type of error.  The database transaction is read-only, so exiting
 * abruptly causes the transaction to roll back, which frees up the database 
 * just as commit.
 */
pub async fn check_pubkey_dependencies(tenant: &String, client_id: &String, 
                                             client_user_id: &String, host: &String, 
                                             host_account: &String)
    -> Result<()>
{
    // Get a connection to the db and start a transaction.
    let mut tx = RUNTIME_CTX.db.begin().await?;

    // -------- Check user_mfa dependency
    let mfa_row = sqlx::query(GET_USER_MFA_ACTIVE)
        .bind(client_user_id)
        .bind(tenant)
        .fetch_optional(&mut *tx)
        .await?;

    match mfa_row {
        Some(row) => {
            // Unpack row.
            let expires_at: String = row.get(0);
            let enabled: i32 = row.get(1);

            // Check whether the user's mfa is enabled.
            if enabled != SQLITE_TRUE {
                let msg = format!("Required user MFA record for user ID {} in tenant {} is disabled.",
                                          client_user_id, tenant);
                error!("{}", msg);
                return Result::Err(anyhow!(msg));
            }

            // Parse the user's mfa expires_at timestamp.
            let expires_at_utc= match timestamp_str_to_datetime(&expires_at) {
                Ok(utc) => utc,
                Err(e) => {
                    // This should not happen since we are the only ones that write the database.
                    let msg = format!("INTERNAL ERROR: Unable to parse user_mfa expires_at value '{}' for user {}@{}: {}", 
                                              expires_at, client_user_id, tenant, e);
                    error!("{}", msg);
                    return Result::Err(anyhow!(msg));
                }
            };

            // Check whether the mfa has expired.
            if expires_at_utc < timestamp_utc() {
                let msg = format!("Required user MFA record for user ID {} in tenant {} expired at {}.",
                                          client_user_id, tenant, expires_at);
                error!("{}", msg);
                return Result::Err(anyhow!(msg));
            }
        },
        None => {
            let msg = format!("Required user MFA record not found for user ID {} in tenant {}.",
                                      client_user_id, tenant);
            error!("{}", msg);
            return Result::Err(anyhow!(msg));
        }
    };

    // -------- Check user_hosts dependency
    let host_row = sqlx::query(GET_USER_HOST_ACTIVE)
        .bind(client_user_id)
        .bind(tenant)
        .bind(host)
        .bind(host_account)
        .fetch_optional(&mut *tx)
        .await?;

        match host_row {
            Some(row) => {
                // Unpack row.
                let expires_at: String = row.get(0);
    
                // Parse the user host mapping's expires_at timestamp.
                let expires_at_utc= match timestamp_str_to_datetime(&expires_at) {
                    Ok(utc) => utc,
                    Err(e) => {
                        // This should not happen since we are the only ones that write the database.
                        let msg = format!("INTERNAL ERROR: Unable to parse user_hosts expires_at value '{}' \
                                                  for user {}@{} with account {} on host {}: {}", 
                                                  expires_at, client_user_id, tenant, host_account, host, e);
                        error!("{}", msg);
                        return Result::Err(anyhow!(msg));
                    }
                };
    
                // Check whether the user host mapping has expired.
                if expires_at_utc < timestamp_utc() {
                    let msg = format!("Required user host record for user {}@{} with account {} on host {} expired at {}.",
                                              client_user_id, tenant, host_account, host, expires_at);
                    error!("{}", msg);
                    return Result::Err(anyhow!(msg));
                }
            },
            None => {
                let msg = format!("Required user host record not found for user {}@{} with account {} on host {}.",
                                          client_user_id, tenant, host_account, host);
                error!("{}", msg);
                return Result::Err(anyhow!(msg));
            }
        };
    
    // -------- Check delegations dependency
    let delg_row = sqlx::query(GET_DELEGATION_ACTIVE)
        .bind(tenant)
        .bind(client_id)
        .bind(client_user_id)
        .fetch_optional(&mut *tx)
        .await?;

        match delg_row {
            Some(row) => {
                // Unpack row.
                let expires_at: String = row.get(0);
    
                // Parse the user's delegation's expires_at timestamp.
                let expires_at_utc= match timestamp_str_to_datetime(&expires_at) {
                    Ok(utc) => utc,
                    Err(e) => {
                        // This should not happen since we are the only ones that write the database.
                        let msg = format!("INTERNAL ERROR: Unable to parse the delegation expires_at value '{}' \
                                                  for client {} and client_user_id {} in tenant {}: {}", 
                                                  expires_at, client_id, client_user_id, tenant, e);
                        error!("{}", msg);
                        return Result::Err(anyhow!(msg));
                    }
                };
    
                // Check whether the delegation has expired.
                if expires_at_utc < timestamp_utc() {
                    let msg = format!("Required delegation record for client {} and client_user_id {} \
                                              in tenant {} expired at {}.",
                                              client_id, client_user_id, tenant, expires_at);
                    error!("{}", msg);
                    return Result::Err(anyhow!(msg));
                }
            },
            None => {
                let msg = format!("Required delegation record not found for client {} and client_user_id {} in tenant {}.",
                                          client_id, client_user_id, tenant);
                error!("{}", msg);
                return Result::Err(anyhow!(msg));
            }
        };
    
    // Commit the transaction.
    tx.commit().await?;

    // All checks passed.
    Ok(())
}