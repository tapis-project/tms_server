#![forbid(unsafe_code)]

use anyhow::Result;
use log::{info, warn};
use chrono::{Utc, DateTime};
use rand_core::{RngCore, OsRng};
use hex;
use sha2::{Sha512, Digest};
use std::io::{self, Write};

use futures::executor::block_on;
use crate::utils::tms_utils::{timestamp_utc, timestamp_utc_secs_to_str};
use crate::utils::db_statements::{INSERT_DELEGATIONS, INSERT_STD_TENANTS, INSERT_USER_HOSTS, INSERT_USER_MFA};
use crate::utils::config::{DEFAULT_TENANT, TEST_TENANT, SQLITE_TRUE, DEFAULT_ADMIN_ID, PERM_ADMIN, TMS_ARGS};

use crate::RUNTIME_CTX;

use super::db_statements::{INSERT_ADMIN, INSERT_CLIENTS};

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
// create_hex_secret:
// ---------------------------------------------------------------------------
/** Get 24 bytes of random bits and convert them to a hex string. */
fn create_hex_secret() -> String {
    let mut dft_key = [0u8; 24];
    OsRng.fill_bytes(&mut dft_key);
    hex::encode(dft_key)
}

// ---------------------------------------------------------------------------
// hash_hex_secret:
// ---------------------------------------------------------------------------
/** Take a hex secret as provided to the user and hash it for storage in the
 * database.
 */
fn hash_hex_secret(hex_str: &String) -> String {
    let mut hasher = Sha512::new();
    hasher.update(hex_str);
    let raw = hasher.finalize();
    hex::encode(raw)
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
            warn!("****** Ignoring error while inserting test records into test tenant: {}", e.to_string());
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
    const TEST_SECRET: &str = "secret1";
    const TEST_USER: &str = "testuser1";
    const TEST_HOST: &str = "testhost1";
    const TEST_HOST_ACCOUNT: &str = "testhostaccount1";

    // Get the timestamp string.
    let now = timestamp_utc();
    let current_ts = timestamp_utc_secs_to_str(now);
    let max_datetime = timestamp_utc_secs_to_str(DateTime::<Utc>::MAX_UTC);

    // Get a connection to the db and start a transaction.
    let mut tx = RUNTIME_CTX.db.begin().await?;

    // -------- Populate clients
    sqlx::query(INSERT_CLIENTS)
        .bind(TEST_TENANT)
        .bind(TEST_APP)
        .bind(TEST_APP_VERS)
        .bind(TEST_CLIENT)
        .bind(TEST_SECRET)
        .bind(SQLITE_TRUE)
        .bind(&current_ts)
        .bind(&current_ts)
        .execute(&mut *tx)
        .await?;

    // -------- Populate user_mfa
    sqlx::query(INSERT_USER_MFA)
        .bind(TEST_TENANT)
        .bind(TEST_USER)
        .bind(&max_datetime)
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
        .bind(&max_datetime)
        .bind(&current_ts)
        .bind(&current_ts)
        .execute(&mut *tx)
        .await?;

    // -------- Populate delegations
    sqlx::query(INSERT_DELEGATIONS)
        .bind(TEST_TENANT)
        .bind(TEST_CLIENT)
        .bind(TEST_USER)
        .bind(&max_datetime)
        .bind(&current_ts)
        .bind(&current_ts)
        .execute(&mut *tx)
        .await?;

    // Commit the transaction.
    tx.commit().await?;
    
    Ok(true)
}
