#![forbid(unsafe_code)]

use anyhow::{Result, anyhow};
use log::{info, warn};
use std::io::{self, Write};
use chrono::{Utc, DateTime};
use sqlx::Row;

use futures::executor::block_on;
use crate::utils::tms_utils::{timestamp_utc, timestamp_utc_secs_to_str, timestamp_str_to_datetime,
                              create_hex_secret, hash_hex_secret, MAX_TMS_UTC_STR};
use crate::utils::db_statements::{INSERT_DELEGATIONS, INSERT_USER_HOSTS, INSERT_USER_MFA};
use crate::utils::config::{DEFAULT_ADMIN_ID, PERM_ADMIN, TMS_CMD_ARGS, DB_TRUE};
use log::error;

use crate::RUNTIME_CTX;

use super::db_statements::{GET_DELEGATION_ACTIVE, GET_DELEGATION_EXISTS, GET_RESERVATION_FOR_EXTEND,
                           GET_USER_HOST_ACTIVE, GET_USER_HOST_EXISTS, GET_USER_MFA_ACTIVE,
                           GET_USER_MFA_EXISTS, INSERT_ADMIN, INSERT_CLIENTS,
                           SELECT_PUBKEY_HOST_ACCOUNT};

/** Multiple Query Transactions
 * 
 * A note on concurrency and the multiple query transactions contained in this file
 * and others in TMS.  The sqlite documentation on concurrency indicates that locks
 * are acquired on database files, not at the row or table level.  One can only assume
 * that the lock holders are threads, whether in the same or different processes.  
 * 
 * To avoid the possibility of deadlocks in TMS, avoid mixing read and write operations 
 * on multiple tables in the same transaction.  In places where that is necessary, make 
 * sure there are no other transactions that issue multiple SQL calls on different 
 * tables in a different order, which could lead to conflicts and deadlocks.
*/

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
        "\n**** Below please find the administrator user ID and password created  ****",
        "\n**** at installation time.                                             ****",
        "\n****                                                                   ****",
        "\n**** WARNING: The passwords are NOT saved by TMS, only hashes of them  ****",
        "\n**** are saved. Please store the passwords permanently in a safe place ****",
        "\n**** accessible to TMS administrators.                                 ****",
        "\n****                                                                   ****",
        "\n****        THIS IS THE ONLY TIME THE PASSWORD IS SHOWN.               ****",
        "\n****                                                                   ****",
        "\n****      ADMIN PASSWORDS ARE NOT RECOVERABLE IF THEY ARE LOST!        ****",
        "\n****                                                                   ****");

    // Add the runtime suffix.
    let msg = prefix.to_string() +
        "\n**** Administrator ID: " + DEFAULT_ADMIN_ID + "                                         ****" +
        "\n**** Password: " + dft_key_str + "        ****" +
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
    // Assume we are initializing for the first time and need to create dummy test data.
    match block_on(create_test_data()) {
        Ok(b) => {
            if b {info!("Test records inserted");}
        }
        Err(e) => {warn!("****** Ignoring error while inserting test records. Error: {}", e);}
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

    // Max expires_at
    let max_tms_utc = DateTime::parse_from_rfc3339(MAX_TMS_UTC_STR).unwrap().with_timezone(&Utc);

    // Get the timestamp string.
    let now = timestamp_utc();

    // Get a connection to the db and start a transaction.
    let mut tx = RUNTIME_CTX.db.begin().await?;

    // -------- Populate clients
    sqlx::query(INSERT_CLIENTS)
        .bind(TEST_APP)
        .bind(TEST_APP_VERS)
        .bind(TEST_CLIENT)
        .bind(test_secret)
        .bind(DB_TRUE)
        .bind(now)
        .bind(now)
        .execute(&mut *tx)
        .await?;

    // -------- Populate user_mfa
    sqlx::query(INSERT_USER_MFA)
        .bind(TEST_USER)
        .bind(max_tms_utc)
        .bind(DB_TRUE)
        .bind(now)
        .bind(now)
        .execute(&mut *tx)
        .await?;

        // -------- Populate user_hosts
    sqlx::query(INSERT_USER_HOSTS)
        .bind(TEST_USER)
        .bind(TEST_HOST)
        .bind(TEST_HOST_ACCOUNT)
        .bind(max_tms_utc)
        .bind(now)
        .bind(now)
        .execute(&mut *tx)
        .await?;

    // -------- Populate delegations
    sqlx::query(INSERT_DELEGATIONS)
        .bind(TEST_CLIENT)
        .bind(TEST_USER)
        .bind(max_tms_utc)
        .bind(now)
        .bind(now)
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
 * 
 * Note that message that contains "INTERNAL ERROR:" should trigger a 500 http 
 * return code.
 */
pub async fn check_pubkey_dependencies(client_id: &String, client_user_id: &String,
                                       host: &String, host_account: &String)
    -> Result<()>
{
    // Get a connection to the db and start a transaction.
    let mut tx = RUNTIME_CTX.db.begin().await?;

    // -------- Check user_mfa dependency
    let mfa_row = sqlx::query(GET_USER_MFA_ACTIVE)
        .bind(client_user_id)
        .fetch_optional(&mut *tx)
        .await?;

    match mfa_row {
        Some(row) => {
            // Unpack row.
            let expires_at: DateTime<Utc> = row.get(0);
            let enabled: bool = row.get(1);

            // Check whether the user's mfa is enabled.
            if enabled != DB_TRUE {
                let msg = format!("Required user MFA record for user ID {} is disabled.",
                                          client_user_id);
                error!("{}", msg);
                return Result::Err(anyhow!(msg));
            }

            // Check whether the mfa has expired.
            if expires_at < timestamp_utc() {
                let msg = format!("Required user MFA record for user ID '{}' expired at {}.",
                                          client_user_id, expires_at);
                error!("{}", msg);
                return Result::Err(anyhow!(msg));
            }
        },
        None => {
            let msg = format!("Required user MFA record not found for user ID {}.", client_user_id);
            error!("{}", msg);
            return Result::Err(anyhow!(msg));
        }
    };

    // -------- Check user_hosts dependency
    let host_row = sqlx::query(GET_USER_HOST_ACTIVE)
        .bind(client_user_id)
        .bind(host)
        .bind(host_account)
        .fetch_optional(&mut *tx)
        .await?;

        match host_row {
            Some(row) => {
                // Unpack row.
                let expires_at: DateTime<Utc> = row.get(0);
    
                // Check whether the user host mapping has expired.
                if expires_at < timestamp_utc() {
                    let msg = format!("Required user host record for user {} with account {} on host {} expired at {}.",
                                              client_user_id, host_account, host, expires_at);
                    error!("{}", msg);
                    return Result::Err(anyhow!(msg));
                }
            },
            None => {
                let msg = format!("Required user host record not found for user {} with account {} on host {}.",
                                          client_user_id, host_account, host);
                error!("{}", msg);
                return Result::Err(anyhow!(msg));
            }
        };
    
    // -------- Check delegations dependency
    let delg_row = sqlx::query(GET_DELEGATION_ACTIVE)
        .bind(client_id)
        .bind(client_user_id)
        .fetch_optional(&mut *tx)
        .await?;

        match delg_row {
            Some(row) => {
                // Unpack row.
                let expires_at: DateTime<Utc> = row.get(0);
    
                // Check whether the delegation has expired.
                if expires_at < timestamp_utc() {
                    let msg = format!("Required delegation record for client {} and client_user_id {} \
                                              in expired at {}.", client_id, client_user_id, expires_at);
                    error!("{}", msg);
                    return Result::Err(anyhow!(msg));
                }
            },
            None => {
                let msg = format!("Required delegation record not found for client {} and client_user_id {}.",
                                          client_id, client_user_id);
                error!("{}", msg);
                return Result::Err(anyhow!(msg));
            }
        };
    
    // Commit the transaction.
    tx.commit().await?;

    // All checks passed.
    Ok(())
}

// ---------------------------------------------------------------------------
// check_parent_reservation:
// ---------------------------------------------------------------------------
/** This function is used to validate reservation extension requests by checking
 * database state. 
 * 
 * Reservation Constraints
 * ----------------------- 
 * When extending a reservation we need to check that these conditions hold on
 * that reservation:
 * 
 *  - The designated parent reservation is not a itself a child of another reservation.
 *  - The parent reservation has not expired.
 * 
 * We identify a child reservation by the fact that its parent_resid is different
 * than its resid.  TMS limits the parent/child tree to a depth of 2. 
 * 
 * Other Constraints
 * -----------------
 * The user_mfa, user_hosts and delegations tables must also contain records that the
 * new extended reservation will depend on.
 * 
 *  - user_mfa - the user must have an mfa record
 *  - user_hosts - the user must have established a link to the reservation's host
 *  - delegations - the user must of delegated access to the reservation's client 
 * 
 * Validating these constraints before actually submitting the reservation extension
 * request allows us to return meaningful messages to users on error. The final arbiter, 
 * however, are foriegn key constraints on the reservation table that take place when
 * the new reservation is created.
 * 
 * Parameters
 * ----------
 * The resid parameter designates the candidate parent reservation for a new extended reservation.
 * The client_id are used to guarantee that clients can only extend their own reservations.
 * The host specifies the where the public key represented by the public_key_fingerprint can be applied.
 *   
 * Note that message that contains "INTERNAL ERROR:" should trigger a 500 http 
 * return code.
 */
pub async fn check_parent_reservation(resid: &String, client_id: &String, client_user_id: &String,
                                      host: &String, public_key_fingerprint: &String)
-> Result<DateTime<Utc>>
{
    // Get a connection to the db and start a transaction.
    let mut tx = RUNTIME_CTX.db.begin().await?;

    // -------- Check reservations dependency
    let res_row = sqlx::query(GET_RESERVATION_FOR_EXTEND)
        .bind(resid)
        .bind(client_id)
        .fetch_optional(&mut *tx)
        .await?;

    // Check the candidate parent reservation and save its expiration time.    
    let expires_at: DateTime<Utc>;
    match res_row {
        Some(row) => {
            // Unpack row.
            let parent_resid: String = row.get(0);
            expires_at = row.get(1);

            // Make sure the parent reservation is not also a child of another reservation.
            // Top-level reservations have their parent_resid set to their own resid, so if
            // the resid used to retrieve the reservation differs from that reservation's
            // parent, then we know the retrieved reservation is already a child. 
            if *resid != parent_resid {
                let msg = format!("Reservation {} cannot be designated as parent for another reservation \
                                          because it is already a child of reservation {}.",
                                            resid, parent_resid);
                error!("{}", msg);
                return Result::Err(anyhow!(msg));
            }

            // Check whether the reservation has expired.
            if expires_at < timestamp_utc() {
                let msg = format!("Parent reservation {} for client {} expired at {}.",
                                            resid, client_id, expires_at);
                error!("{}", msg);
                return Result::Err(anyhow!(msg));
            }
        },
        None => {
            let msg = format!("NOT_FOUND: Reservation {} not found for client {}.",
                                        resid, client_id);
            error!("{}", msg);
            return Result::Err(anyhow!(msg));
        }
    };  

    // -------- Check user_mfa dependency
    let mfa_row = sqlx::query(GET_USER_MFA_EXISTS)
        .bind(client_user_id)
        .fetch_optional(&mut *tx)
        .await?;
    match mfa_row {
        Some(_) => (),
        None => {
            let msg = format!("No MFA entry found for user {}.", client_user_id);
            error!("{}", msg);
            return Result::Err(anyhow!(msg));
        }
    };

    // -------- Check user_hosts dependency
    // First get host account.
    let pkey_row = sqlx::query(SELECT_PUBKEY_HOST_ACCOUNT)
        .bind(client_id)
        .bind(host)
        .bind(public_key_fingerprint)
        .fetch_optional(&mut *tx)
        .await?; 
    let host_account: String = match pkey_row {
        Some(h) => h.get(0),
        None => {
            let msg = format!("Unable to retrieve host account from pubkey record for client {} on host {} with fingerprint {}.",
                                        client_id, host, public_key_fingerprint);
            error!("{}", msg);
            return Result::Err(anyhow!(msg));
        }    
    };

    let host_row = sqlx::query(GET_USER_HOST_EXISTS)
        .bind(client_user_id)
        .bind(host)
        .bind(&host_account)
        .fetch_optional(&mut *tx)
        .await?;
    match host_row {
        Some(_) => (),
        None => {
            let msg = format!("No user/host mapping found for user {} for account {} on host {}.",
                                        client_user_id, host_account, host);
            error!("{}", msg);
            return Result::Err(anyhow!(msg));
        }
    };

    // -------- Check delegation dependency
    let delg_row = sqlx::query(GET_DELEGATION_EXISTS)
        .bind(client_id)
        .bind(client_user_id)
        .fetch_optional(&mut *tx)
        .await?;
    match delg_row {
        Some(_) => (),
        None => {
            let msg = format!("No delegation to client {} found for user {}.", client_id, client_user_id);
            error!("{}", msg);
            return Result::Err(anyhow!(msg));
        }
    };

    // Commit the transaction.
    tx.commit().await?;

    // All checks passed.
    Ok(expires_at)
}
