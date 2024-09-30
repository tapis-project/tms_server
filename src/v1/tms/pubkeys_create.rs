#![forbid(unsafe_code)]

use poem::Request;
use poem_openapi::{ OpenApi, payload::Json, Object, ApiResponse };
use anyhow::{Result, anyhow};

use futures::executor::block_on;

use crate::utils::authz::{authorize, AuthzTypes, get_tenant_header, get_client_id_header};
use crate::utils::errors::HttpResult;
use crate::utils::keygen::{self, KeyType};
use crate::utils::db_types::PubkeyInput;
use crate::utils::db_statements::INSERT_PUBKEYS;
use crate::utils::db::check_pubkey_dependencies;
use crate::utils::tms_utils::{self, timestamp_utc, timestamp_utc_to_str, calc_expires_at, RequestDebug};
use log::{error, info};

use crate::RUNTIME_CTX;

// ***************************************************************************
//                          Request/Response Definiions
// ***************************************************************************
pub struct NewSshKeysApi;

#[derive(Object)]
pub struct ReqNewSshKeys
{
    client_user_id: String,
    host: String,
    host_account: String,
    num_uses: i32,     // negative means i32::MAX
    ttl_minutes: i32,  // negative means i32::MAX
    key_type: Option<String>,  // RSA, ECDSA, ED25519, DEFAULT (=ED25519)   
}

#[derive(Object, Debug)]
struct RespNewSshKeys
{
    result_code: String,
    result_msg: String,
    private_key: String,
    public_key: String,
    public_key_fingerprint: String,
    key_type: String,
    key_bits: String,
    max_uses: String,
    remaining_uses: String,
    expires_at: String,
}

// Implement the debug record trait for logging.
impl RequestDebug for ReqNewSshKeys {   
    type Req = ReqNewSshKeys;
    fn get_request_info(&self) -> String {
        let mut s = String::with_capacity(255);
        s.push_str("  Request body:");
        s.push_str("\n    client_user_id: ");
        s.push_str(&self.client_user_id);
        s.push_str("\n    host: ");
        s.push_str(&self.host);
        s.push_str("\n    host_account: ");
        s.push_str(&self.host_account);
        s.push_str("\n    num_uses: ");
        s.push_str(&self.num_uses.to_string());
        s.push_str("\n    ttl_minutes: ");
        s.push_str(&self.ttl_minutes.to_string());
        s.push_str("\n    key_type: ");
        let kt = match &self.key_type {
            Some(k) => k,
            None => "None",
        };
        s.push_str(kt);
        s.push('\n');
        s
    }
}

// Extracted header values to complete request input
#[derive(Debug)]
struct NewSshKeysExtension
{
    client_id: String,
    tenant: String,
}

impl NewSshKeysExtension {
    fn new(client_id: String, tenant: String,) -> Self 
    { Self {client_id, tenant} }  
}

// ------------------- HTTP Status Codes -------------------
#[derive(Debug, ApiResponse)]
enum TmsResponse {
    #[oai(status = 201)]
    Http201(Json<RespNewSshKeys>),
    #[oai(status = 400)]
    Http400(Json<HttpResult>),
    #[oai(status = 401)]
    Http401(Json<HttpResult>),
    #[oai(status = 403)]
    Http403(Json<HttpResult>),
    #[oai(status = 500)]
    Http500(Json<HttpResult>),
}

fn make_http_201(resp: RespNewSshKeys) -> TmsResponse {
    TmsResponse::Http201(Json(resp))
}
fn make_http_400(msg: String) -> TmsResponse {
    TmsResponse::Http400(Json(HttpResult::new(400.to_string(), msg)))
}
fn make_http_401(msg: String) -> TmsResponse {
    TmsResponse::Http401(Json(HttpResult::new(401.to_string(), msg)))
}
fn make_http_403(msg: String) -> TmsResponse {
    TmsResponse::Http403(Json(HttpResult::new(403.to_string(), msg)))
}
fn make_http_500(msg: String) -> TmsResponse {
    TmsResponse::Http500(Json(HttpResult::new(500.to_string(), msg)))    
}

// ***************************************************************************
//                             OpenAPI Endpoint
// ***************************************************************************
#[OpenApi]
impl NewSshKeysApi {
    #[oai(path = "/tms/pubkeys/creds", method = "post")]
    async fn get_new_ssh_keys(&self, http_req: &Request, req: Json<ReqNewSshKeys>) -> TmsResponse {
        match RespNewSshKeys::process(http_req, &req) {
            Ok(r) => r,
            Err(e) => {
                // Assume a server fault if a raw error came through.
                let msg = "ERROR: ".to_owned() + e.to_string().as_str();
                error!("{}", msg);
                make_http_500(msg)
            }
        }
    }
}

// ***************************************************************************
//                          Request/Response Methods
// ***************************************************************************
impl RespNewSshKeys {
    #[allow(clippy::too_many_arguments)]
    fn new(result_code: &str, result_msg: &str, private_key: String, public_key: String, 
           public_key_fingerprint: String, key_type: String, key_bits: String,
           max_uses: String, remaining_uses: String, expires_at: String) -> Self {
        Self {result_code: result_code.to_string(), 
              result_msg: result_msg.to_string(), 
              private_key, public_key, public_key_fingerprint,
              key_type, key_bits, max_uses, remaining_uses, expires_at,
            }
    }

    fn process(http_req: &Request, req: &ReqNewSshKeys) -> Result<TmsResponse, anyhow::Error> {
        // Conditional logging depending on log level.
        tms_utils::debug_request(http_req, req);

        // -------------------- Extract Headers ----------------------
        // Get the headers used in this function.
        let req_ext = match get_header_values(http_req) {
            Ok(h) => h,
            Err(e) => {
                return Ok(make_http_400(e.to_string()));
            }
        };

        // -------------------- Authorize ----------------------------
        // Only the client and tenant admin can query a client record.
        let allowed = [AuthzTypes::ClientOwn];
        let authz_result = authorize(http_req, &allowed);
        if !authz_result.is_authorized() {
            let msg = format!("ERROR: NOT AUTHORIZED Credential mismatch for client {} in tenant {}.", 
                                      req_ext.client_id, req_ext.tenant);
            error!("{}", msg);
            return Ok(make_http_401(msg));
        }

        // --------------------- Check Expirations -----------------------
        // The 3 tables whose expiration times need to be checked before we create this key are:
        //
        //  user_mfa - use tenant and client_user_id to target unique record
        //  delegations - use tenant, client_id and client_user_id to target unique record
        //  user_hosts - use tenant, client_user_id, host and host_account to target unique record
        //
        // Each of the above tables are queried using values that define a unique index on the
        // target table.  This guarantees that either 0 or 1 record will be returned.  In the 
        // former case, the pubkey key cannot be created because one of its foriegn keys doesn't
        // exist.  In the latter case, we have to check that the retrieved record has not 
        // expired.
        //
        // This method returns an detailed error message that indicates which table did not
        // contain the required values and whether the error resulted from a missing or 
        // expired record.  
        match block_on(check_pubkey_dependencies(&req_ext.tenant, &req_ext.client_id, 
                                        &req.client_user_id, &req.host, &req.host_account))
        {
            Ok(_) => (),
            Err(e) => {
                let msg = format!("Missing or expired dependency: {}", e);
                error!("{}", msg);
                if msg.contains("INTERNAL ERROR:") {return Ok(make_http_500(msg));}
                else {return Ok(make_http_403(msg));}

            } 
        }

        // ------------------------ Generate Keys ------------------------
        // Get the caller's key type or use default.
        let key_type_str = match &req.key_type {
            Some(k) => k.as_str(),
            None => "ED25519",
        };
        let key_type_upper = key_type_str.to_uppercase();

        // Get the enumerated key type.
        let key_type = match key_type_upper.as_str() {
            "RSA" => KeyType::Rsa,
            "ECDSA" => KeyType::Ecdsa,
            "ED25519" => KeyType::Ed25519,
            _ => KeyType::Ed25519,
        };

        // Generate the new key pair.
        let keyinfo = match keygen::generate_key(key_type) {
            Ok(k) => k,
            Err(e) => {
                return Result::Err(anyhow!(e));
            }
        };
        
        // ------------------------ Update Database --------------------
        // Interpret numeric input.
        let max_uses = if req.num_uses < 0 {i32::MAX} else {req.num_uses};
        let ttl_minutes = if req.ttl_minutes < 0 {i32::MAX} else {req.ttl_minutes};

        // Use the same current UTC timestamp in all related time caculations.
        // We also use the original requested ttl_minutes to calculate expires_at
        // so that we get a uniform maximum uniform datetime rather then one that
        // changes with current time when req.ttl_minutes = -1.
        let now  = timestamp_utc();
        let current_ts  = timestamp_utc_to_str(now);
        let expires_at  = calc_expires_at(now, req.ttl_minutes); 
        let remaining_uses = max_uses;

        // Create the input record.
        let input_record = PubkeyInput::new(
            req_ext.tenant.clone(),
            req_ext.client_id.clone(),
            req.client_user_id.clone(), 
            req.host.clone(), 
            req.host_account.clone(),
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
        info!("A key of type '{}' created for '{}@{}' for host '{}' expires at {} and has {} remaining uses.", 
            keyinfo.key_type.clone(), req.client_user_id, req_ext.tenant, req.host, expires_at, remaining_uses);

        // Success! Zero key bits means a fixed key length.
        Ok(make_http_201(Self::new("0", "success", 
                    keyinfo.private_key, 
                    keyinfo.public_key, 
                    keyinfo.public_key_fingerprint,
                    keyinfo.key_type,
                    keyinfo.key_bits.to_string(),
                    max_uses.to_string(),
    remaining_uses.to_string(),
                    expires_at,)))
    }
}

// ***************************************************************************
//                          Private Functions
// ***************************************************************************
// ---------------------------------------------------------------------------
// insert_new_key:
// ---------------------------------------------------------------------------
async fn insert_new_key(rec: PubkeyInput) -> Result<u64> {
    // Get a connection to the db and start a transaction.  Uncommited transactions 
    // are automatically rolled back when they go out of scope. 
    // See https://docs.rs/sqlx/latest/sqlx/struct.Transaction.html.
    let mut tx = RUNTIME_CTX.db.begin().await?;
    
    // Create the insert statement.
    let result = sqlx::query(INSERT_PUBKEYS)
        .bind(rec.tenant)
        .bind(rec.client_id)
        .bind(rec.client_user_id)
        .bind(rec.host)
        .bind(rec.host_account)
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
// get_header_values:
// ---------------------------------------------------------------------------
fn get_header_values(http_req: &Request) -> Result<NewSshKeysExtension> {
    // Get the required header values.
    let hdr_client_id = get_client_id_header(http_req)?;
    let hdr_tenant = get_tenant_header(http_req)?;

    Ok(NewSshKeysExtension::new(hdr_client_id, hdr_tenant))
}
