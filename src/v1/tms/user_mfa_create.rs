#![forbid(unsafe_code)]

use poem::Request;
use poem_openapi::{ OpenApi, payload::Json, Object, ApiResponse };
use anyhow::Result;

use crate::utils::errors::HttpResult;
use crate::utils::db_statements::{INSERT_USER_MFA, INSERT_USER_MFA_NOT_STRICT};
use crate::utils::db_types::UserMfaInput;
use crate::utils::authz::{authorize, AuthzTypes, get_tenant_header, X_TMS_TENANT}; 
use crate::utils::tms_utils::{self, timestamp_utc, timestamp_utc_to_str, calc_expires_at, 
                              RequestDebug, check_tenant_enabled};
use log::{error, info};

use crate::RUNTIME_CTX;

// Insert fails on conflict.        
const STRICT:bool = true;

// ***************************************************************************
//                          Request/Response Definiions
// ***************************************************************************
pub struct CreateUserMfaApi;

// ***************************************************************************
//                          Request/Response Definiions
// ***************************************************************************
#[derive(Object)]
pub struct ReqCreateUserMfa
{
    tenant: String,
    tms_user_id: String,
    ttl_minutes: i32,  // negative means i32::MAX
}

#[derive(Object, Debug)]
pub struct RespCreateUserMfa
{
    result_code: String,
    result_msg: String,
    tms_user_id: String,
    expires_at: String,
    enabled: bool,
}

// Implement the debug record trait for logging.
impl RequestDebug for ReqCreateUserMfa {   
    type Req = ReqCreateUserMfa;
    fn get_request_info(&self) -> String {
        let mut s = String::with_capacity(255);
        s.push_str("  Request body:");
        s.push_str("\n    tenant: ");
        s.push_str(&self.tenant);
        s.push_str("\n    tms_user_id: ");
        s.push_str(&self.tms_user_id);
        s.push_str("\n    tts_minutes: ");
        s.push_str(&self.ttl_minutes.to_string());
        s
    }
}

// ------------------- HTTP Status Codes -------------------
#[derive(Debug, ApiResponse)]
enum TmsResponse {
    #[oai(status = 201)]
    Http201(Json<RespCreateUserMfa>),
    #[oai(status = 400)]
    Http400(Json<HttpResult>),
    #[oai(status = 401)]
    Http401(Json<HttpResult>),
    #[oai(status = 403)]
    Http403(Json<HttpResult>),
    #[oai(status = 500)]
    Http500(Json<HttpResult>),
}

fn make_http_201(resp: RespCreateUserMfa) -> TmsResponse {
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
impl CreateUserMfaApi {
    #[oai(path = "/tms/usermfa", method = "post")]
    async fn create_client(&self, http_req: &Request, req: Json<ReqCreateUserMfa>) -> TmsResponse {
        // -------------------- Get Tenant Header --------------------
        // Get the required tenant header value.
        let hdr_tenant = match get_tenant_header(http_req) {
            Ok(t) => t,
            Err(e) => return make_http_400(e.to_string()),
        };

        // Check that the tenant specified in the header is the same as the one in the request body.
        if hdr_tenant != req.tenant {
            let msg = format!("ERROR: FORBIDDEN - The tenant in the {} header ({}) does not match the tenant in the request body ({})", 
                                      X_TMS_TENANT, hdr_tenant, req.tenant);
            error!("{}", msg);
            return make_http_403(msg);  
        }

        // Check tenant.
        if !check_tenant_enabled(&hdr_tenant).await {
            return make_http_400("Tenant not enabled.".to_string());
        }

        // -------------------- Authorize ----------------------------
        // Currently, only the tenant admin can create a user mfa record.
        // When user authentication is implemented, we'll add user-own 
        // authorization and any additional validation.
        let allowed = [AuthzTypes::TenantAdmin];
        let authz_result = authorize(http_req, &allowed).await;
        if !authz_result.is_authorized() {
            let msg = format!("ERROR: NOT AUTHORIZED to add a user MFA record in tenant {}.", req.tenant);
            error!("{}", msg);
            return make_http_401(msg);
        }

        // -------------------- Process Request ----------------------
        match RespCreateUserMfa::process(http_req, &req).await {
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
impl RespCreateUserMfa {
    /// Create a new response.
    fn new(result_code: &str, result_msg: String, tms_user_id: String, expires_at: String, enabled: bool,) -> Self {
        Self {result_code: result_code.to_string(), result_msg, tms_user_id, expires_at, enabled,}}

    /// Process the request.
    async fn process(http_req: &Request, req: &ReqCreateUserMfa) -> Result<TmsResponse, anyhow::Error> {
        // Conditional logging depending on log level.
        tms_utils::debug_request(http_req, req);

        // ------------------------ Time Values ------------------------ 
        // The ttl can be negative, which means maximum ttl.
        let ttl_minutes = if req.ttl_minutes < 0 {i32::MAX} else {req.ttl_minutes};

        // Use the same current UTC timestamp in all related time caculations..
        let now = timestamp_utc();
        let current_ts = timestamp_utc_to_str(now);
        let expires_at = calc_expires_at(now, ttl_minutes);

        // Create the input record.  Note that we save the hash of
        // the hex secret, but never the secret itself.  
        let input_record = UserMfaInput::new(
            req.tenant.clone(),
            req.tms_user_id.clone(),
            expires_at.clone(),
            1,
            current_ts.clone(), 
            current_ts,
        );

        // Insert the new key record.
        insert_user_mfa(input_record, STRICT).await?;
        info!("MFA for user '{}' created in tenant '{}' with experation at {}.", 
              req.tms_user_id, req.tenant, expires_at.clone());
        
        // Return the secret represented in hex.
        Ok(make_http_201(Self::new("0", "success".to_string(), 
                            req.tms_user_id.clone(), expires_at, true)))
    }
}

// ***************************************************************************
//                          Private Functions
// ***************************************************************************
// ---------------------------------------------------------------------------
// insert_user_mfa:
// ---------------------------------------------------------------------------
pub async fn insert_user_mfa(rec: UserMfaInput, strict: bool) -> Result<u64> {
    // Choose the query based on strictness requirement.
    let sql_query = if strict {INSERT_USER_MFA} else {INSERT_USER_MFA_NOT_STRICT};

    // Get a connection to the db and start a transaction.  Uncommited transactions 
    // are automatically rolled back when they go out of scope. 
    // See https://docs.rs/sqlx/latest/sqlx/struct.Transaction.html.
    let mut tx = RUNTIME_CTX.db.begin().await?;
    
    // Create the insert statement.
    let result = sqlx::query(sql_query)
        .bind(rec.tenant)
        .bind(rec.tms_user_id)
        .bind(rec.expires_at)
        .bind(rec.enabled)
        .bind(rec.created)
        .bind(rec.updated)
        .execute(&mut *tx)
        .await?;

    // Commit the transaction.
    tx.commit().await?;

    Ok(result.rows_affected())
}
