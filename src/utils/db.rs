#![forbid(unsafe_code)]

use anyhow::{Result, anyhow};
use log::{info};
use std::io::{self, Write};
use chrono::{Utc, DateTime};
use sqlx::Row;

use crate::utils::tms_utils::{timestamp_utc, create_hex_secret, hash_hex_secret, MAX_TMS_UTC_STR, timestamp_utc_to_str, calc_expires_at};
use crate::utils::db_statements::{INSERT_DELEGATIONS, INSERT_PUBKEYS, INSERT_RP_LOGIN, SEL_CLIENT_EXISTS,
                                  SEL_PUBKEY_EXISTS, SEL_IDP_EXISTS, INSERT_IDP, INSERT_TMS_IDENTITY};
use crate::utils::config::{DEFAULT_ADMIN_ID, PERM_ADMIN, TMS_CMD_ARGS, DB_TRUE, TEST_CLIENT, TEST_APP, TEST_CLIENT_SECRET};

use log::error;
use crate::RUNTIME_CTX;
use crate::utils::db_types::{ClientInput, IdPInput, PubkeyInput};
use crate::utils::keygen;
use crate::utils::keygen::KeyType;
use super::db_statements::{GET_DELEGATION_ACTIVE, GET_DELEGATION_EXISTS, GET_RESERVATION_FOR_EXTEND,
                           GET_RP_LOGIN_ACTIVE, GET_RP_LOGIN_EXISTS, INSERT_ADMIN, INSERT_CLIENT,
                           SELECT_PUBKEY_HOST_ACCOUNT, UPDATE_CLIENT_ENABLED, SEL_DELEGATION_EXISTS};

const TEST_IDP_ID: &str = "danger_mode_idp";
const TEST_IDP_NAME: &str = "Fake Test IdP";
const TEST_IDP_CLIENT_ID: &str = "12345678-1234-1234-1234-abcdefghtest";
const TEST_IDP_CLIENT_SECRET: &str = "FakeTestdf894adfduG89JRazpE6DCDvkrM";
const TEST_IDP_REDIRECT_URL: &str = "https://auth.fake.test.org/v2/oauth2/authorize";
const TEST_IDP_TOKEN_URL: &str = "https://auth.fake.test.org/v2/oauth2/token";
const TEST_IDP_PROVIDER_TYPE: &str = "danger_mode";
const TEST_RP_ID: &str = "test_fake_rp";
const TEST_RP_NAME: &str = "Fake Test RP";
const TEST_IDP_SUPPORTS_LOGIN: bool = true;
const TEST_IDP_SUPPORTS_RESOURCES: bool = false;
const TEST_TMS_USER_BASE: &str = "testtmsuser";
const TEST_TMS_USER_DOMAIN: &str = "DangerModeTestIdP";
const TEST_HOST: &str = "testhost";
const TEST_HOST_ACCOUNT: &str = "testhostaccount";
const TEST_RP_ACCOUNT: &str = "testrpaccount";
const TEST_FIXED_RP_ACCT: &str  = "testrpaccount101";
const TEST_FIXED_FINGERPRINT: &str= "SHA256:0EddP3z8IwV4YqzewwoiJVyfKhmFj4VlsDBZqCaan24";
const TEST_RECORD_CNT: i32 = 101;
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
 * Insert a IdP record
 */
pub async fn insert_new_idp(rec: IdPInput) -> Result<u64> {
    let mut tx = RUNTIME_CTX.db.begin().await?;
    // Create the insert statement.
    let result = sqlx::query(INSERT_IDP)
        .bind(rec.id.clone())
        .bind(rec.name.clone())
        .bind(rec.client_id.clone())
        .bind(rec.client_secret.clone())
        .bind(rec.identity_redirect_url.clone())
        .bind(rec.oauth2_token_url.clone())
        .bind(rec.provider_type.clone())
        .bind(rec.supports_login)
        .bind(rec.supports_resources)
        .bind(rec.created)
        .bind(rec.updated)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    info!("New IdP created. Id: {} ClientId: {} Name: {} ProviderType: {} SupportsLogin: {} SupportsResources: {}",
          rec.id, rec.client_id, rec.name, rec.provider_type, rec.supports_login, rec.supports_resources);
    Ok(result.rows_affected())
}

/*
 * Insert a client pubkey record
 */
pub async fn insert_new_client(rec: ClientInput) -> Result<u64> {
    let mut tx = RUNTIME_CTX.db.begin().await?;
    // Create the insert statement.
    let result = sqlx::query(INSERT_CLIENT)
        .bind(rec.name.clone())
        .bind(rec.client_id.clone())
        .bind(rec.secret)
        .bind(rec.enabled)
        .bind(rec.created)
        .bind(rec.updated)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    info!("New client created. ClientId: {} App: '{}' enabled: {} created: {} updated: {}",
          rec.client_id, rec.name, rec.enabled, rec.created, rec.updated);
    Ok(result.rows_affected())
}

/*
 * Insert a new pubkey record if there is not at least one already associated with host+host_account
 * For testing purposes as long as there is at least one we should be good.
 *
 * If asked to generate for 'testuser001' use a fixed pubkey fingerprint.
 * Fingerprint will not be correct but having at lease one fixed value is convenient for testing.
 */
pub async fn insert_new_test_pubkey_if_none(test_tms_identity: String, test_rp_acct: String,
                                            test_host: String, test_host_acct: String) -> Result<u64> {
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
        Err(e) => { return Err(anyhow!(e)); }
    };
    // Determine the fingerprint.
    let pubkey_fingerprint =
        if test_rp_acct == TEST_FIXED_RP_ACCT { String::from(TEST_FIXED_FINGERPRINT) }
        else { keyinfo.public_key_fingerprint };
    let now  = timestamp_utc();
    let expires_at  = calc_expires_at(now, MAX_TTL_MINUTES);
    let remaining_uses = MAX_USES;
    // Create the input record.
    let input_record = PubkeyInput::new(
        TEST_CLIENT.to_string(),
        test_tms_identity.clone(),
        TEST_RP_ID.to_string(),
        test_rp_acct.clone(),
        test_host.clone(),
        test_host_acct.clone(),
        pubkey_fingerprint.clone(),
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

    info!("Creating keypair for ClientId: {} TmsId: {} RPId: {} RPAcct: {} Host: {} HostAcct: {}",
          TEST_CLIENT, test_tms_identity, TEST_RP_ID, test_rp_acct, test_host, test_host_acct);
    // Create the insert statement.
    let result = sqlx::query(INSERT_PUBKEYS)
        .bind(input_record.client_id)
        .bind(input_record.tms_identity.clone())
        .bind(TEST_RP_ID)
        .bind(input_record.rp_account.clone())
        .bind(input_record.host.clone())
        .bind(input_record.host_account)
        .bind(input_record.public_key_fingerprint.clone())
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
    info!("Created keypair for ClientId: {} TmsId: {} RpId: {} RpAcct: {} Host: {} HostAcct: {}",
          TEST_CLIENT, test_tms_identity, TEST_RP_ID, test_rp_acct, test_host, test_host_acct);
    info!("Pubkey fingerprint: {}", input_record.public_key_fingerprint);
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
        .bind(rec.client_id.clone())
        .bind(rec.tms_identity.clone())
        .bind(rec.rp_id.clone())
        .bind(rec.rp_account.clone())
        .bind(rec.host.clone())
        .bind(rec.host_account.clone())
        .bind(rec.public_key_fingerprint.clone())
        .bind(rec.public_key.clone())
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
    info!("pubkey record created. ClientId: {} TmsId: {} RpId: {} RpAcct: {} Host: {} HostAcct: {} ExpiresAt: {} RemainingUses: {} KeyType: {}.",
           rec.client_id, rec.tms_identity, rec.rp_id, rec.rp_account, rec.host, rec.host_account,
           rec.expires_at, rec.remaining_uses, rec.key_type);
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
// create_test_idp:
// ---------------------------------------------------------------------------
/** This function either experiences an error or returns true (false is never returned). */
pub async fn create_test_idp() -> Result<u64> {
    let mut tx = RUNTIME_CTX.db.begin().await?;
    // If client already exists then we are done
    let skip_create: bool = sqlx::query_scalar(SEL_IDP_EXISTS)
        .bind(TEST_IDP_ID)
        .fetch_one(&mut *tx).await?;
    if skip_create {return Ok(0)}

    let test_idp_client_secret_hash: String = hash_hex_secret(&TEST_IDP_CLIENT_SECRET.to_string());
    let now = timestamp_utc();
    // Create the IdP
    // Create the input record. Note we save the hash of the hex secret, but never the secret.
    let idp_input = IdPInput::new(
        TEST_IDP_ID.to_string(),
        TEST_IDP_NAME.to_string(),
        TEST_IDP_CLIENT_ID.to_string(),
        test_idp_client_secret_hash,
        TEST_IDP_REDIRECT_URL.to_string(),
        TEST_IDP_TOKEN_URL.to_string(),
        TEST_IDP_PROVIDER_TYPE.to_string(),
        TEST_IDP_SUPPORTS_LOGIN,
        TEST_IDP_SUPPORTS_RESOURCES,
        now.clone(),
        now.clone()
    );
    let inserts = insert_new_idp(idp_input).await?;
    Ok(inserts)
}

// ---------------------------------------------------------------------------
// create_test_idp:
// ---------------------------------------------------------------------------
/** This function either experiences an error or returns true (false is never returned). */
pub async fn create_test_rp() -> Result<u64> {
    let mut tx = RUNTIME_CTX.db.begin().await?;
    // If client already exists then we are done
    let skip_create: bool = sqlx::query_scalar(SEL_IDP_EXISTS)
        .bind(TEST_RP_ID)
        .fetch_one(&mut *tx).await?;
    if skip_create {return Ok(0)}

    let test_idp_client_secret_hash: String = hash_hex_secret(&TEST_IDP_CLIENT_SECRET.to_string());
    let now = timestamp_utc();
    // Create the RP
    // Create the input record. Note we save the hash of the hex secret, but never the secret.
    let idp_input = IdPInput::new(
        TEST_RP_ID.to_string(),
        TEST_RP_NAME.to_string(),
        TEST_IDP_CLIENT_ID.to_string(),
        test_idp_client_secret_hash,
        TEST_IDP_REDIRECT_URL.to_string(),
        TEST_IDP_TOKEN_URL.to_string(),
        TEST_IDP_PROVIDER_TYPE.to_string(),
        TEST_IDP_SUPPORTS_LOGIN,
        TEST_IDP_SUPPORTS_RESOURCES,
        now.clone(),
        now.clone()
    );
    let inserts = insert_new_idp(idp_input).await?;
    Ok(inserts)
}

// ------------------------------------------------------------------------------------------------
// create_test_data:
// Create records in tables: tms_identities, resource_provider_logins, delegations
// ------------------------------------------------------------------------------------------------
/*
 This function either experiences an error or returns true (false is never returned).
 Use now timestamp for created, updated and last_login
 */
pub async fn create_test_data() -> Result<u64> {
    // Max expires_at
    let max_tms_utc = DateTime::parse_from_rfc3339(MAX_TMS_UTC_STR).unwrap().with_timezone(&Utc);
    // Get the timestamp string.
    let now = timestamp_utc();

    // Create records for 101 test users in the test client. Do this in a txn
    // User 101 will have a fixed pubkey fingerprint to support smoke test getPubKey scenario.
    // Get a connection to the db and start a transaction.
    let mut insert_count = 0;
    for n in 1..=TEST_RECORD_CNT {
        let test_tms_identity = format!("{}{:03}@{}", TEST_TMS_USER_BASE, n, TEST_TMS_USER_DOMAIN);
        let test_host = format!("{}{:03}", TEST_HOST, n);
        let test_host_acct = format!("{}{:03}", TEST_HOST_ACCOUNT, n);
        let test_rp_account = format!("{}{:03}", TEST_RP_ACCOUNT, n);
        let mut tx = RUNTIME_CTX.db.begin().await?;

        // Check for existing record. If found then continue;
        // Note: checking for a delegation record is enough since the delegation records reference the rp_login record as a foreign key.
        let skip_create: bool = sqlx::query_scalar(SEL_DELEGATION_EXISTS)
            .bind(TEST_CLIENT)
            .bind(test_tms_identity.clone())
            .bind(TEST_RP_ID)
            .bind(test_rp_account.clone())
            .fetch_one(&mut *tx).await?;
        if skip_create {continue};

        // First create a TMS identity in table tms_identities
        info!("Creating TMS identity record for TMS user: {}", test_tms_identity.clone());
        sqlx::query(INSERT_TMS_IDENTITY)
            .bind(test_tms_identity.clone())
            .execute(&mut *tx)
            .await?;

        info!("Creating test records for tms identity: {} rp_id: {} rp_account {} host: {} host_acct {}",
              test_tms_identity, TEST_RP_ID, test_rp_account, test_host, test_host_acct);
        // -------- Populate rp_login
        sqlx::query(INSERT_RP_LOGIN)
            .bind(test_tms_identity.clone())
            .bind(TEST_RP_ID)
            .bind(test_rp_account.clone())
            .bind(DB_TRUE)
            .bind(now)
            .bind(now)
            .bind(now)
            .execute(&mut *tx)
            .await?;

        // -------- Populate delegations
        sqlx::query(INSERT_DELEGATIONS)
            .bind(TEST_CLIENT)
            .bind(test_tms_identity.clone())
            .bind(TEST_RP_ID)
            .bind(test_rp_account.clone())
            .bind(max_tms_utc)
            .bind(now)
            .bind(now)
            .execute(&mut *tx)
            .await?;
        insert_count += 1;
        // Commit the transaction.
        tx.commit().await?;
        info!("Created delegation records for tms identity: {} host: {} host_acct {}",
              test_tms_identity, test_host, test_host_acct);
    }

    Ok(insert_count)
}

// ---------------------------------------------------------------------------
// create_test_keys:
// ---------------------------------------------------------------------------
/**
 * This function either experiences an error or returns true (false is never returned).
 * User 101 will have a fixed pubkey fingerprint to smoke test.
 */
pub async fn create_test_keys() -> Result<u64> {
    // For each test user create one pubkey entry, ignore generated private key
    let mut insert_count = 0;
    for n in 1..=TEST_RECORD_CNT {
        let test_tms_identity = format!("{}{:03}@{}", TEST_TMS_USER_BASE, n, TEST_TMS_USER_DOMAIN);
        let test_rp_acct = format!("{}{:03}", TEST_RP_ACCOUNT, n);
        let test_host = format!("{}{:03}", TEST_HOST, n);
        let test_host_acct = format!("{}{:03}", TEST_HOST_ACCOUNT, n);

        // Create a new test pubkey for user if none exists.
        // This should return 0 if one already exists and 1 if a new one was created
        let inserts = insert_new_test_pubkey_if_none(test_tms_identity, test_rp_acct,
                                                          test_host, test_host_acct).await?;
        insert_count += inserts;
    }
    Ok(insert_count)
}

// ---------------------------------------------------------------------------
// check_pubkey_dependencies:
// ---------------------------------------------------------------------------
/**
 * When creating a public key or a reservation on a public key we must check that the user's
 * RP_LOGIN and client delegation are currently active. Active means that the records exist in their
 *   respective tables, are enabled and have not expired.
 * 
 * We return as soon as we encounter any dependency that cannot be fulfilled or any other type of
 * error. The database transaction is read-only, so exiting abruptly causes the transaction to roll
 * back, which frees up the database just as commit.
 * 
 * Note that message that contains "INTERNAL ERROR:" should trigger a 500 http return code.
 */
pub async fn check_pubkey_dependencies(tms_identity: &String, rp_id: &String, rp_account: &String)
    -> Result<()>
{
    // Get a connection to the db and start a transaction.
    let mut tx = RUNTIME_CTX.db.begin().await?;

    // -------- Check rp_login dependency
    let rplogin_row = sqlx::query(GET_RP_LOGIN_ACTIVE)
        .bind(tms_identity)
        .bind(rp_id)
        .bind(rp_account)
        .fetch_optional(&mut *tx)
        .await?;

    match rplogin_row {
        Some(row) => {
            // Unpack row.
            let enabled: bool = row.get(0);

            // Check whether the user's rplogin is enabled.
            if enabled != DB_TRUE {
                let msg = format!("Required RP_LOGIN record is disabled. TmsId: {} RpId: {} RpAcct: {}",
                                  tms_identity, rp_id, rp_account);
                error!("{}", msg);
                return Result::Err(anyhow!(msg));
            }
        },
        None => {
            let msg = format!("Required user RP_LOGIN record not found. msId: {} RpId: {} RpAcct: {}",
                              tms_identity, rp_id, rp_account);
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
 * The rp_login and delegations tables must also contain records that the
 * new extended reservation will depend on.
 * 
 *  - rp_login - the user must have an rplogin record
 *  - delegations - the user must have delegated access to the reservation's client
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
 * Note that message that contains "INTERNAL ERROR:" should trigger a 500 http
 * return code.
 */
pub async fn check_parent_reservation(resid: &String, client_id: &String, tms_identity: &String,
                                      rp_id: &String, rp_account: &String, host: &String,
                                      public_key_fingerprint: &String) -> Result<DateTime<Utc>>
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

    // -------- Check rp_login dependency
    let rplogin_row = sqlx::query(GET_RP_LOGIN_EXISTS)
        .bind(tms_identity)
        .bind(rp_id)
        .bind(rp_account)
        .fetch_optional(&mut *tx)
        .await?;
    match rplogin_row {
        Some(_) => (),
        None => {
            let msg = format!("No RP_LOGIN entry found. TmsId: {} RpId: {} RpAcct: {}",
                                     tms_identity, rp_id, rp_account);
            error!("{}", msg);
            return Result::Err(anyhow!(msg));
        }
    };

    // -------- Check delegation dependency
    let delg_row = sqlx::query(GET_DELEGATION_EXISTS)
        .bind(client_id)
        .bind(tms_identity)
        .bind(rp_id)
        .bind(rp_account)
        .fetch_optional(&mut *tx)
        .await?;
    match delg_row {
        Some(_) => (),
        None => {
            let msg = format!("No delegation record found. ClientId: {} TmsId: {} RpId: {} RpAcct: {}",
                                     client_id, tms_identity, rp_id, rp_account);
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

// ***************************************************************************
//                          Private Functions
// ***************************************************************************
