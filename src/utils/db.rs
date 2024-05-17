#![forbid(unsafe_code)]

use anyhow::Result;
use log::{info, warn};
use chrono::{Utc, DateTime};

use futures::executor::block_on;
use crate::utils::tms_utils::{timestamp_utc, timestamp_utc_secs_to_str};
use crate::utils::db_statements::{INSERT_DELEGATIONS, INSERT_STD_TENANTS, INSERT_USER_HOSTS, INSERT_USER_MFA};
use crate::utils::config::{DEFAULT_TENANT, TEST_TENANT, SQLITE_TRUE};

use crate::RUNTIME_CTX;

use super::db_statements::INSERT_CLIENTS;

// ---------------------------------------------------------------------------
// create_std_tenants:
// ---------------------------------------------------------------------------
pub async fn create_std_tenants() -> Result<u64> {
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

    // Commit the transaction.
    tx.commit().await?;

    // Return the number of tenant insertions that took place.
    Ok(dft_result.rows_affected() + tst_result.rows_affected())
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
