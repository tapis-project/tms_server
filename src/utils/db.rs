#![forbid(unsafe_code)]

use anyhow::Result;
use log::{info, warn, error};

use futures::executor::block_on;
use crate::utils::tms_utils::{timestamp_utc, timestamp_utc_secs_to_str};
use crate::utils::db_statements::INSERT_STD_TENANTS;
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
        .bind(&current_ts)
        .bind(&current_ts)
        .execute(&mut *tx)
        .await?;

    let tst_result = sqlx::query(INSERT_STD_TENANTS)
        .bind(TEST_TENANT)
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
            if b {
                info!("Test records inserted in test tenant.");
            } 
        }
        Err(e) => {
            warn!("Ignoring error while inserting test records into test tenant: {}", e.to_string());
        }
    };

    // match block_on(temp()) {
    //     Ok(b) => {
    //         if b {
    //             info!("******** Test records DELETED in test tenant.");
    //         } 
    //     }
    //     Err(e) => {
    //         warn!("********* Ignoring error while DELETING test records into test tenant: {}", e.to_string());
    //     }

    // };
}

// ---------------------------------------------------------------------------
// create_test_data:
// ---------------------------------------------------------------------------
/** This function either experiences an error or returns true (false is never returned). */
async fn create_test_data() -> Result<bool> {
    // Constants used locally.
    const TEST_APP: &str = "testapp1";
    const TEST_CLIENT: &str = "testclient1";
    const TEST_SECRET: &str = "secret";

    // Get the timestamp string.
    let now = timestamp_utc();
    let current_ts = timestamp_utc_secs_to_str(now);

    // Get a connection to the db and start a transaction.
    let mut tx = RUNTIME_CTX.db.begin().await?;

    // -------- Populate clients
    info!("********* at insert");
    let result = sqlx::query(INSERT_CLIENTS)
        .bind(TEST_TENANT)
        .bind(TEST_APP)
        .bind("1.0")
        .bind(TEST_CLIENT)
        .bind(TEST_SECRET)
        .bind(SQLITE_TRUE)
        .bind(&current_ts)
        .bind(&current_ts)
        .execute(&mut *tx)
        .await;

    // Commit the transaction.
    info!("*********** at commit");
    tx.commit().await;
    
    Ok(true)
}

async fn temp() -> Result<bool> {
    let mut tx2 = RUNTIME_CTX.db.begin().await?;
    let r = sqlx::query("DELETE FROM clients")
        .execute(&mut *tx2)
        .await?;
    info!("*********** delete rows affected = {}", r.rows_affected());
    if let Err(e) = tx2.commit().await {
        error!("********** DELETE failed (rows={}): {}", r.rows_affected() , e.to_string());
    } 
    info!("*********** delete rows affected after commit = {}", r.rows_affected());
    Ok(true)
}
