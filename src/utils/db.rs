#![forbid(unsafe_code)]

use anyhow::{Result, anyhow};
use log::{info, warn};
use std::io::{self, Write};
use chrono::{Utc, DateTime};
use sqlx::Row;

use crate::utils::tms_utils::{timestamp_utc, timestamp_utc_secs_to_str, timestamp_str_to_datetime, create_hex_secret, hash_hex_secret, MAX_TMS_UTC_STR, timestamp_utc_to_str, calc_expires_at};
use crate::utils::db_statements::{INSERT_DELEGATIONS, INSERT_PUBKEYS, INSERT_USER_HOSTS, INSERT_USER_MFA, SEL_CLIENT_EXISTS, SEL_PUBKEY_EXISTS};
use crate::utils::config::{DEFAULT_ADMIN_ID, PERM_ADMIN, TMS_CMD_ARGS, DB_TRUE, TEST_CLIENT, TEST_APP, TEST_CLIENT_SECRET};

use log::error;

use crate::RUNTIME_CTX;
use crate::utils::db_types::{ClientInput, PubkeyInput};
use crate::utils::keygen;
use crate::utils::keygen::KeyType;
use super::db_statements::{GET_DELEGATION_ACTIVE, GET_DELEGATION_EXISTS, GET_RESERVATION_FOR_EXTEND,
                           GET_USER_HOST_ACTIVE, GET_USER_HOST_EXISTS, GET_USER_MFA_ACTIVE,
                           GET_USER_MFA_EXISTS, INSERT_ADMIN, INSERT_CLIENT,
                           SELECT_PUBKEY_HOST_ACCOUNT, UPDATE_CLIENT_ENABLED, SEL_DELEGATION_EXISTS};

const TEST_USER: &str = "testuser";
const TEST_HOST: &str = "testhost";
const TEST_HOST_ACCOUNT: &str = "testhostaccount";
const MAX_USES: i32 = i32::MAX;
const MAX_TTL_MINUTES: i32 = i32::MAX;
const KEY_TYPE: KeyType = KeyType::Ed25519;

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

/*
 * Insert a client pubkey record
 */
pub async fn insert_new_client(rec: ClientInput) -> Result<u64> {
    let mut tx = RUNTIME_CTX.db.begin().await?;
    // Create the insert statement.
    let result = sqlx::query(INSERT_CLIENT)
        .bind(rec.app_name.clone())
        .bind(rec.client_id.clone())
        .bind(rec.client_secret)
        .bind(rec.enabled)
        .bind(rec.created)
        .bind(rec.updated)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    info!("New client created. ClientId: {} App: '{}' enabled: {} created: {} updated: {}",
          rec.client_id, rec.app_name, rec.enabled, rec.created, rec.updated);
    Ok(result.rows_affected())
}

/*
 * Insert a new pubkey record if there is not at least one already associated with host+host_account
 * For testing purposes as long as there is at least one we should be good.
 */
pub async fn insert_new_test_pubkey_if_none(test_user: String, test_host: String,
                                            test_host_acct: String) -> Result<u64> {
    let mut tx = RUNTIME_CTX.db.begin().await?;

    // Check for existing record, create only if needed
    // Note: We check for any pubkey, not for a specific pubkey.
    let skip_create: bool = sqlx::query_scalar(SEL_PUBKEY_EXISTS)
        .bind(test_host_acct.clone())
        .bind(test_host.clone())
        .fetch_one(&mut *tx).await?;
    if skip_create { return Ok(0) }

    // Generate the new key pair.
    let keyinfo = match keygen::generate_key(KEY_TYPE) {
        Ok(k) => k,
        Err(e) => { return Result::Err(anyhow!(e)); }
    };
    let now  = timestamp_utc();
    let expires_at  = calc_expires_at(now, MAX_TTL_MINUTES);
    let remaining_uses = MAX_USES;
    // Create the input record.
    let input_record = PubkeyInput::new(
        TEST_CLIENT.to_string(),
        test_user.clone(),
        test_host.clone(),
        test_host_acct.clone(),
        keyinfo.public_key_fingerprint.clone(),
        keyinfo.public_key.clone(),
        keyinfo.key_type.clone(),
        keyinfo.key_bits,
        MAX_USES,
        remaining_uses,
        MAX_TTL_MINUTES,
        expires_at.clone(),
        now.clone(),
        now.clone(),
    );

    info!("Creating keypair for user: {} host: {} host_acct {}", test_user, test_host, test_host_acct);
    // Create the insert statement.
    let result = sqlx::query(INSERT_PUBKEYS)
        .bind(input_record.client_id)
        .bind(input_record.client_user_id.clone())
        .bind(input_record.host.clone())
        .bind(input_record.host_account)
        .bind(input_record.public_key_fingerprint)
        .bind(input_record.public_key)
        .bind(input_record.key_type.clone())
        .bind(input_record.key_bits)
        .bind(input_record.max_uses)
        .bind(input_record.remaining_uses)
        .bind(input_record.initial_ttl_minutes)
        .bind(input_record.expires_at)
        .bind(input_record.created)
        .bind(input_record.updated)
        .execute(&mut *tx)
        .await?;
    // Commit the transaction.
    tx.commit().await?;
    info!("Created keypair for user: {} host: {} host_acct {}", test_user, test_host, test_host_acct);
    Ok(result.rows_affected())
}
/*
 * Insert a new pubkey record
 */
pub async fn insert_new_pubkey(rec: PubkeyInput) -> Result<u64> {
    // Get a connection to the db and start a transaction.  Uncommited transactions 
    // are automatically rolled back when they go out of scope. 
    // See https://docs.rs/sqlx/latest/sqlx/struct.Transaction.html.
    let mut tx = RUNTIME_CTX.db.begin().await?;
    // Create the insert statement.
    let result = sqlx::query(INSERT_PUBKEYS)
        .bind(rec.client_id)
        .bind(rec.client_user_id.clone())
        .bind(rec.host.clone())
        .bind(rec.host_account)
        .bind(rec.public_key_fingerprint)
        .bind(rec.public_key)
        .bind(rec.key_type.clone())
        .bind(rec.key_bits)
        .bind(rec.max_uses)
        .bind(rec.remaining_uses)
        .bind(rec.initial_ttl_minutes)
        .bind(rec.expires_at)
        .bind(rec.created)
        .bind(rec.updated)
        .execute(&mut *tx)
        .await?;
    // Commit the transaction.
    tx.commit().await?;
    info!("A key of type '{}' created for '{}' for host '{}' expires at {} and has {} remaining uses.", 
            rec.key_type.clone(), rec.client_user_id, rec.host, rec.expires_at, rec.remaining_uses);
    Ok(result.rows_affected())
}

/*
 * Create the default admin user ~~admin
 * This method should only be called when the --install option is specified.  
 * It's a no-op if called during regular execution.
 */
pub async fn create_default_admin() -> Result<u64> {
    // Guard against repeated initialization of admin.
    if !TMS_CMD_ARGS.install {
        return Ok(0);
    }

    // Get the timestamp string.
    let now = timestamp_utc();

    // Get a connection to the db and start a transaction.
    let mut tx = RUNTIME_CTX.db.begin().await?;

    // Create admin user ids.
    let dft_key_str = create_hex_secret();
    let dft_key_hash = hash_hex_secret(&dft_key_str);
    let dft_admin_result = sqlx::query(INSERT_ADMIN)
        .bind(DEFAULT_ADMIN_ID)
        .bind(&dft_key_hash)
        .bind(PERM_ADMIN)
        .bind(now)
        .bind(now)
        .execute(&mut *tx)
        .await?;

    // Commit the transaction.
    tx.commit().await?;

    // --- MOST IMPORTANT ---
    // One time printout of the admin secret.
    print_admin_secret_message(&dft_key_str)?;

    // Return the number of insertions that took place.
    Ok(dft_admin_result.rows_affected())
}

// ---------------------------------------------------------------------------
// print_admin_secret_message:
// ---------------------------------------------------------------------------
/*
 * Print one-time message to stdout that contains the admin_user and admin_secret for the
 * default admin user. This only happens when the --install option was specified and this program
 * terminates after installation with the secret information visible to user.
 */
fn print_admin_secret_message(dft_key_str: &String) -> Result<()> {
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
// create_test_client:
// ---------------------------------------------------------------------------
/** This function either experiences an error or returns true (false is never returned). */
pub async fn create_test_client() -> Result<u64> {
    let mut tx = RUNTIME_CTX.db.begin().await?;
    // If client already exists then we are done
    let skip_create: bool = sqlx::query_scalar(SEL_CLIENT_EXISTS)
        .bind(TEST_CLIENT)
        .fetch_one(&mut *tx).await?;
    if skip_create {return Ok(0)}

    let test_client_secret_hash: String = hash_hex_secret(&TEST_CLIENT_SECRET.to_string());
    let now = timestamp_utc();
    // Create the client
    // Create the input record. Note we save the hash of the hex secret, but never the secret.
    let client_input = ClientInput::new(
        TEST_APP.to_string(),
        TEST_CLIENT.to_string(),
        test_client_secret_hash,
        DB_TRUE,
        now.clone(),
        now.clone(),
    );
    let inserts = insert_new_client(client_input).await?;
    Ok(inserts)
}

// ---------------------------------------------------------------------------
// create_test_data:
// ---------------------------------------------------------------------------
/** This function either experiences an error or returns true (false is never returned). */
pub async fn create_test_data() -> Result<u64> {
    // Max expires_at
    let max_tms_utc = DateTime::parse_from_rfc3339(MAX_TMS_UTC_STR).unwrap().with_timezone(&Utc);
    // Get the timestamp string.
    let now = timestamp_utc();

    // Create records for 100 test users in the test client. Do this in a txn
    // Get a connection to the db and start a transaction.
    let mut insert_count = 0;
    for n in 1..=100 {
        let test_user = format!("{}{:03}", TEST_USER, n);
        let test_host = format!("{}{:03}", TEST_HOST, n);
        let test_host_acct = format!("{}{:03}", TEST_HOST_ACCOUNT, n);
        let mut tx = RUNTIME_CTX.db.begin().await?;

        // Check for existing record. If found then continue;
        // Note: checking for a delegation record is  enough since the delegation and user_hosts
        //       records reference the user_mfa record as a foreign key.
        let skip_create: bool = sqlx::query_scalar(SEL_DELEGATION_EXISTS)
            .bind(TEST_CLIENT)
            .bind(test_user.clone())
            .fetch_one(&mut *tx).await?;
        if skip_create {continue};
        info!("Creating delegation records for user: {} host: {} host_acct {}", test_user, test_host, test_host_acct);
        // -------- Populate user_mfa
        sqlx::query(INSERT_USER_MFA)
            .bind(test_user.clone())
            .bind(max_tms_utc)
            .bind(DB_TRUE)
            .bind(now)
            .bind(now)
            .execute(&mut *tx)
            .await?;

        // -------- Populate user_hosts
        sqlx::query(INSERT_USER_HOSTS)
            .bind(test_user.clone())
            .bind(test_host.clone())
            .bind(test_host_acct.clone())
            .bind(max_tms_utc)
            .bind(now)
            .bind(now)
            .execute(&mut *tx)
            .await?;

        // -------- Populate delegations
        sqlx::query(INSERT_DELEGATIONS)
            .bind(TEST_CLIENT)
            .bind(test_user.clone())
            .bind(max_tms_utc)
            .bind(now)
            .bind(now)
            .execute(&mut *tx)
            .await?;
        insert_count += 1;
        // Commit the transaction.
        tx.commit().await?;
        info!("Created delegation records for user: {} host: {} host_acct {}", test_user, test_host, test_host_acct);
    }

    Ok(insert_count)
}

// ---------------------------------------------------------------------------
// create_test_keys:
// ---------------------------------------------------------------------------
/** This function either experiences an error or returns true (false is never returned). */
pub async fn create_test_keys() -> Result<u64> {
    // For each test user create one pubkey entry, ignore generated private key
    let mut insert_count = 0;
    for n in 1..=100 {
        let test_user = format!("{}{:03}", TEST_USER, n);
        let test_host = format!("{}{:03}", TEST_HOST, n);
        let test_host_acct = format!("{}{:03}", TEST_HOST_ACCOUNT, n);

        // Create a new test pubkey for user if none exists.
        // This should return 0 if one already exists and 1 if a new one was created
        let inserts = insert_new_test_pubkey_if_none(test_user, test_host, test_host_acct).await?;
        insert_count += inserts;
    }
    Ok(insert_count)
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
// ---------------------------------------------------------------------------
// set_test_enabled_internal:
// ---------------------------------------------------------------------------
pub async fn set_test_enabled_internal(test_client: &String, enabled: bool) -> Result<u64>
{
    // Get timestamp.
    let now = timestamp_utc();
    let current_ts = timestamp_utc_to_str(now);
    // Update count.
    let mut updates: u64 = 0;
    info!("Updating client enabled flag. Client Id: {} enabled: {} updated: {}", test_client, enabled, current_ts);
    // Get a connection to the db and start a transaction.
    let mut tx = RUNTIME_CTX.db.begin().await?;
    // Issue the db update call.
    let result = sqlx::query(UPDATE_CLIENT_ENABLED)
        .bind(enabled)
        .bind(now)
        .bind(test_client)
        .execute(&mut *tx)
        .await?;
    updates += result.rows_affected();
    // Commit the transaction.
    tx.commit().await?;
    Ok(updates)
}
