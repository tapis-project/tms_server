#![forbid(unsafe_code)]

//use ssh_key::private::{ KeypairData, PrivateKey, RsaKeypair };
use poem_openapi::{ OpenApi, payload::Json, Object };
use anyhow::{Result, anyhow};

use std::convert::TryInto;
use futures::executor::block_on;
use chrono::{Utc, DateTime, Duration};

use crate::utils::keygen::{self, KeyType};
use crate::utils::db_types::PubkeyInput;
use crate::utils::db_statements::INSERT_PUBKEYS;
use crate::utils::tms_utils::{timestamp_utc, timestamp_utc_to_str, timestamp_utc_secs_to_str};
use log::{info, error};

use crate::RUNTIME_CTX;

// ***************************************************************************
//                          Request/Response Definiions
// ***************************************************************************
pub struct NewSshKeysApi;

#[derive(Object)]
struct ReqNewSshKeys
{
    client_id: String,
    client_secret: String,
    tenant: String,
    client_user_id: String,
    host: String,
    num_uses: u32,     // 0 means unlimited
    ttl_minutes: u32,  // 0 means unlimited
    key_type: Option<String>,  // RSA, ECDSA, ED25519, DEFAULT (=RSA)   
}

#[derive(Object)]
struct RespNewSshKeys
{
    result_code: String,
    result_msg: String,
    private_key: String,
    public_key: String,
    public_key_fingerprint: String,
    key_type: String,
    key_bits: String,
    remaining_uses: String,
    expires_at: String,
}

// ***************************************************************************
//                             OpenAPI Endpoint
// ***************************************************************************
#[OpenApi]
impl NewSshKeysApi {
    #[oai(path = "/tms/creds/sshkeys", method = "post")]
    async fn get_new_ssh_keys(&self, req: Json<ReqNewSshKeys>) -> Json<RespNewSshKeys> {
        let resp = match RespNewSshKeys::process(&req) {
            Ok(r) => r,
            Err(e) => {
                let msg = "ERROR: ".to_owned() + e.to_string().as_str();
                error!("{}", msg);
                RespNewSshKeys::new("1", msg.as_str(), "".to_string(), "".to_string(), 
                                    "".to_string(), "".to_string(), "".to_string(), 
                                    "".to_string(), "".to_string() )},
        };

        Json(resp)
    }
}

// ***************************************************************************
//                          Request/Response Methods
// ***************************************************************************
impl RespNewSshKeys {
    fn new(result_code: &str, result_msg: &str, private_key: String, public_key: String, 
           public_key_fingerprint: String, key_type: String, key_bits: String,
           remaining_uses: String, expires_at: String) -> Self {
        Self {result_code: result_code.to_string(), 
              result_msg: result_msg.to_string(), 
              private_key, public_key, public_key_fingerprint,
              key_type, key_bits, remaining_uses, expires_at,
            }
    }

    fn process(req: &ReqNewSshKeys) -> Result<RespNewSshKeys, anyhow::Error> {
        // ------------------------ Generate Keys ------------------------
        // Get the caller's key type or use default.
        let key_type_str = match &req.key_type {
            Some(k) => k.as_str(),
            None => "RSA",
        };
        let key_type_upper = key_type_str.to_uppercase();

        // Get the enumerated key type.
        let key_type = match key_type_upper.as_str() {
            "RSA" => KeyType::Rsa,
            "ECDSA" => KeyType::Ecdsa,
            "ED25519" => KeyType::Ed25519,
            _ => KeyType::Rsa,
        };

        // Generate the new key pair.
        let keyinfo = match keygen::generate_key(key_type) {
            Ok(k) => k,
            Err(e) => {
                return Result::Err(anyhow!(e));
            }
        };
        
        // ------------------------ Update Database --------------------
        // Safely convert u32s to i32s.
        let max_uses: i32 = match req.num_uses.try_into(){
            Ok(num) => num,
            Err(_) => i32::MAX,
        };
        let ttl_minutes: i32 = match req.ttl_minutes.try_into(){
            Ok(num) => num,
            Err(_) => i32::MAX,
        };

        // Use the same current UTC timestamp in all related time caculations..
        let now = timestamp_utc();
        let current_ts = timestamp_utc_to_str(now);
        let expires_at = calc_expires_at(now, ttl_minutes);
        let remaining_uses = calc_remaining_uses(max_uses);

        // Create the input record.
        let input_record = PubkeyInput::new(
            req.tenant.clone(),
            req.client_user_id.clone(), 
            req.host.clone(), 
            keyinfo.public_key_fingerprint.clone(), 
            keyinfo.public_key.clone(), 
            keyinfo.key_type.clone(), 
            keyinfo.key_bits, 
            max_uses, 
            remaining_uses, 
            ttl_minutes, 
            expires_at.clone(), 
            current_ts.clone(), 
            current_ts,
        );

        // Insert the new key record.
        block_on(insert_new_key(input_record))?;
        info!("Key pair created for {}@{} for host {}.", req.client_user_id, req.tenant, req.host);

        // Success! Zero key bits means a fixed key length.
        Ok(Self::new("0", "success", 
                    keyinfo.private_key, 
                    keyinfo.public_key, 
                    keyinfo.public_key_fingerprint,
                    keyinfo.key_type,
                    keyinfo.key_bits.to_string(),
    remaining_uses.to_string(),
                    expires_at,))
    }
}

// ***************************************************************************
//                          Private Functions
// ***************************************************************************
// ---------------------------------------------------------------------------
// insert_new_key:
// ---------------------------------------------------------------------------
async fn insert_new_key(rec: PubkeyInput) -> Result<u64> {
    // Get a connection to the db and start a transaction.
    let mut tx = RUNTIME_CTX.db.begin().await?;
    
    // Create the insert statement.
    let result = sqlx::query(INSERT_PUBKEYS)
        .bind(rec.tenant)
        .bind(rec.client_user_id)
        .bind(rec.host)
        .bind(rec.public_key_fingerprint)
        .bind(rec.public_key)
        .bind(rec.key_type)
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

    Ok(result.rows_affected())
}

// ---------------------------------------------------------------------------
// calc_expires_at:
// ---------------------------------------------------------------------------
fn calc_expires_at(now : DateTime<Utc>, ttl_minutes : i32) -> String {
    if ttl_minutes <= 0 {
        timestamp_utc_secs_to_str(DateTime::<Utc>::MAX_UTC)
    } else {
        timestamp_utc_secs_to_str(now + Duration::minutes(ttl_minutes as i64))
    }
}

// ---------------------------------------------------------------------------
// calc_remaining_uses:
// ---------------------------------------------------------------------------
fn calc_remaining_uses(max_uses : i32) -> i32 {
    if max_uses <= 0 {
        i32::MAX
    } else {
        max_uses
    }
}
