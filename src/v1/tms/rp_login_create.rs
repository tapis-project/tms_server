#![forbid(unsafe_code)]

use poem::Request;
use poem_openapi::{ OpenApi, payload::Json, Object, ApiResponse };
use anyhow::Result;
use chrono::{DateTime, Utc};

use crate::utils::errors::HttpResult;
use crate::utils::db_statements::{INSERT_RP_LOGIN, INSERT_RP_LOGIN_NOT_STRICT};
use crate::utils::db_types::RPLoginInput;
use crate::utils::authz::{authorize, AuthzTypes};
use crate::utils::tms_utils::{self, timestamp_utc, RequestDebug};
use log::{error, info};
use crate::RUNTIME_CTX;
use crate::utils::config::DB_TRUE;

// Insert fails on conflict.        
const STRICT:bool = true;

// ***************************************************************************
//                          Request/Response Definitions
// ***************************************************************************
pub struct CreateRPLoginApi;

// ***************************************************************************
//                          Request/Response Definitions
// ***************************************************************************
#[derive(Object)]
pub struct ReqCreateRPLogin
{
    tms_identity: String,
    rp_id: String,
    rp_account: String,
    ttl_minutes: i32,  // negative means i32::MAX
}

#[derive(Object, Debug)]
pub struct RespCreateRPLogin
{
    result_code: String,
    result_msg: String,
    tms_identity: String,
    rp_id: String,
    rp_account: String,
    enabled: bool,
}

// Implement the debug record trait for logging.
impl RequestDebug for ReqCreateRPLogin {
    type Req = ReqCreateRPLogin;
    fn get_request_info(&self) -> String {
        let mut s = String::with_capacity(255);
        s.push_str("  Request body:");
        s.push_str("\n    tms_identity: ");
        s.push_str(&self.tms_identity);
        s.push_str("\n    rp_id: ");
        s.push_str(&self.rp_id);
        s.push_str("\n    rp_account: ");
        s.push_str(&self.rp_account);
        s.push_str("\n    tts_minutes: ");
        s.push_str(&self.ttl_minutes.to_string());
        s
    }
}

// ------------------- HTTP Status Codes -------------------
#[derive(Debug, ApiResponse)]
enum TmsResponse {
    #[oai(status = 201)]
    Http201(Json<RespCreateRPLogin>),
    #[oai(status = 400)]
    Http400(Json<HttpResult>),
    #[oai(status = 401)]
    Http401(Json<HttpResult>),
    #[oai(status = 403)]
    Http403(Json<HttpResult>),
    #[oai(status = 500)]
    Http500(Json<HttpResult>),
}

fn make_http_201(resp: RespCreateRPLogin) -> TmsResponse {
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
impl CreateRPLoginApi {
    #[oai(path = "/tms/rPLogin", method = "post")]
    async fn create_rp_login(&self, http_req: &Request, req: Json<ReqCreateRPLogin>) -> TmsResponse {
        // -------------------- Authorize ----------------------------
        // Currently, only the admin can create a user rp_login record.
        // When user authentication is implemented, we'll add user-own 
        // authorization and any additional validation.
        let allowed = [AuthzTypes::TmsAdmin];
        let authz_result = authorize(http_req, &allowed).await;
        if !authz_result.is_authorized() {
            let msg = format!("ERROR: NOT AUTHORIZED to add a user RP_LOGIN record.");
            error!("{}", msg);
            return make_http_401(msg);
        }

        // -------------------- Process Request ----------------------
        match RespCreateRPLogin::process(http_req, &req).await {
            Ok(r) => r,
            Err(e) => {
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
impl RespCreateRPLogin {
    /// Create a new response.
    fn new(result_code: &str, result_msg: String, tms_identity: String, rp_id: String,
           rp_account: String, enabled: bool,) -> Self {
        Self {result_code: result_code.to_string(), result_msg, tms_identity, rp_id, rp_account, enabled,}}

    /// Process the request.
    async fn process(http_req: &Request, req: &ReqCreateRPLogin) -> Result<TmsResponse, anyhow::Error> {
        // Conditional logging depending on log level.
        tms_utils::debug_request(http_req, req);

        // ------------------------ Time Values ------------------------ 
        // The ttl can be negative, which means maximum ttl.
        let ttl_minutes = if req.ttl_minutes < 0 {i32::MAX} else {req.ttl_minutes};

        // Use the same current UTC timestamp in all related time caculations..
        let now = timestamp_utc();

        // Create the input record.  Note that we save the hash of
        // the hex secret, but never the secret itself.  
        let input_record = RPLoginInput::new(
            req.tms_identity.clone(),
            req.rp_id.clone(),
            req.rp_account.clone(),
            DB_TRUE,
            now.clone(),
            now.clone(),
            now // TODO/TBD this is last_login. Just use now? Or allow for it to be passed in somehow?
        );

        // Insert the new key record.
        insert_rp_login(input_record, STRICT).await?;
        info!("RP_LOGIN for tms_identity: {} rp_id: {} rp_account: {} last_login: {}.",
               req.tms_identity, req.rp_id, req.rp_account, now);
        
        // Return the secret represented in hex.
        Ok(make_http_201(Self::new("0", "success".to_string(), req.tms_identity.clone(),
                                   req.rp_id.clone(), req.rp_account.clone(), true)))
    }
}

// ***************************************************************************
//                          Private Functions
// ***************************************************************************
// ---------------------------------------------------------------------------
// insert_rp_login:
// ---------------------------------------------------------------------------
pub async fn insert_rp_login(rec: RPLoginInput, strict: bool) -> Result<u64> {
    // Choose the query based on strictness requirement.
    let sql_query = if strict { INSERT_RP_LOGIN } else { INSERT_RP_LOGIN_NOT_STRICT };

    // Get a connection to the db and start a transaction.  Uncommited transactions 
    // are automatically rolled back when they go out of scope. 
    // See https://docs.rs/sqlx/latest/sqlx/struct.Transaction.html.
    let mut tx = RUNTIME_CTX.db.begin().await?;
    
    // Create the insert statement.
    let result = sqlx::query(sql_query)
        .bind(rec.tms_identity)
        .bind(rec.rp_id)
        .bind(rec.rp_account)
        .bind(rec.enabled)
        .bind(rec.created)
        .bind(rec.updated)
        .bind(rec.last_login)
        .execute(&mut *tx)
        .await?;

    // Commit the transaction.
    tx.commit().await?;

    Ok(result.rows_affected())
}
