#![forbid(unsafe_code)]

use poem::Request;
use poem_openapi::{ OpenApi, payload::Json, Object, ApiResponse };
use anyhow::Result;
use futures::executor::block_on;

use crate::utils::errors::HttpResult;
use crate::utils::db_statements::{INSERT_DELEGATIONS, INSERT_DELEGATIONS_NOT_STRICT};
use crate::utils::db_types::DelegationInput;
use crate::utils::authz::{authorize, AuthzTypes, get_tenant_header, X_TMS_TENANT}; 
use crate::utils::tms_utils::{self, timestamp_utc, timestamp_utc_to_str, calc_expires_at, RequestDebug, check_tenant_enabled};
use log::{error, info};

use crate::RUNTIME_CTX;

// Insert fails on conflict.        
const STRICT:bool = true;

// ***************************************************************************
//                          Request/Response Definiions
// ***************************************************************************
pub struct CreateDelegationsApi;

// ***************************************************************************
//                          Request/Response Definiions
// ***************************************************************************
#[derive(Object)]
pub struct ReqCreateDelegations
{
    tenant: String,
    client_id: String,
    client_user_id: String,
    ttl_minutes: i32,  // negative means i32::MAX
}

#[derive(Object, Debug)]
pub struct RespCreateDelegations
{
    result_code: String,
    result_msg: String,
    client_id: String,
    client_user_id: String,
    expires_at: String,
}

// Implement the debug record trait for logging.
impl RequestDebug for ReqCreateDelegations {   
    type Req = ReqCreateDelegations;
    fn get_request_info(&self) -> String {
        let mut s = String::with_capacity(255);
        s.push_str("  Request body:");
        s.push_str("\n    tenant: ");
        s.push_str(&self.tenant);
        s.push_str("\n    client_id: ");
        s.push_str(&self.client_id);
        s.push_str("\n    client_user_id: ");
        s.push_str(&self.client_user_id);
        s.push_str("\n    tts_minutes: ");
        s.push_str(&self.ttl_minutes.to_string());
        s
    }
}

// ------------------- HTTP Status Codes -------------------
#[derive(Debug, ApiResponse)]
enum TmsResponse {
    #[oai(status = 201)]
    Http201(Json<RespCreateDelegations>),
    #[oai(status = 400)]
    Http400(Json<HttpResult>),
    #[oai(status = 401)]
    Http401(Json<HttpResult>),
    #[oai(status = 403)]
    Http403(Json<HttpResult>),
    #[oai(status = 500)]
    Http500(Json<HttpResult>),
}

fn make_http_201(resp: RespCreateDelegations) -> TmsResponse {
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
impl CreateDelegationsApi {
    #[oai(path = "/tms/delegations", method = "post")]
    async fn create_client(&self, http_req: &Request, req: Json<ReqCreateDelegations>) -> TmsResponse {
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
        // Currently, only the tenant admin can create a delegation record.
        // When user authentication is implemented, we'll add user-own 
        // authorization and any additional validation.
        let allowed = [AuthzTypes::TenantAdmin];
        let authz_result = authorize(http_req, &allowed).await;
        if !authz_result.is_authorized() {
            let msg = format!("ERROR: NOT AUTHORIZED to add a client delegation record in tenant {}.", req.tenant);
            error!("{}", msg);
            return make_http_401(msg);
        }

        // -------------------- Process Request ----------------------
        match RespCreateDelegations::process(http_req, &req) {
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
impl RespCreateDelegations {
    /// Create a new response.
    fn new(result_code: &str, result_msg: String, client_id: String, client_user_id: String, 
           expires_at: String,) -> Self {
        Self {result_code: result_code.to_string(), result_msg, client_id, client_user_id, expires_at,}}

    /// Process the request.
    fn process(http_req: &Request, req: &ReqCreateDelegations) -> Result<TmsResponse, anyhow::Error> {
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
        let input_record = DelegationInput::new(
            req.tenant.clone(),
            req.client_id.clone(),
            req.client_user_id.clone(),
            expires_at.clone(),
            current_ts.clone(), 
            current_ts,
        );

        // Insert the new key record.
        block_on(insert_delegation(input_record, STRICT))?;
        info!("Delegation for user '{}' to client '{}' created in tenant '{}' with expiration at {}.", 
              req.client_user_id, req.client_id, req.tenant, expires_at);
        
        // Return the secret represented in hex.
        Ok(make_http_201(Self::new("0", "success".to_string(), 
                         req.client_id.clone(), req.client_user_id.clone(), expires_at,)))
    }
}

// ***************************************************************************
//                          Private Functions
// ***************************************************************************
// ---------------------------------------------------------------------------
// insert_delegation:
// ---------------------------------------------------------------------------
pub async fn insert_delegation(rec: DelegationInput, strict: bool) -> Result<u64> {
    // Choose the query based on strictness requirement.
    let sql_query = if strict {INSERT_DELEGATIONS} else {INSERT_DELEGATIONS_NOT_STRICT};

    // Get a connection to the db and start a transaction.  Uncommited transactions 
    // are automatically rolled back when they go out of scope. 
    // See https://docs.rs/sqlx/latest/sqlx/struct.Transaction.html.
    let mut tx = RUNTIME_CTX.db.begin().await?;
    
    // Create the insert statement.
    let result = sqlx::query(sql_query)
        .bind(rec.tenant)
        .bind(rec.client_id)
        .bind(rec.client_user_id)
        .bind(rec.expires_at)
        .bind(rec.created)
        .bind(rec.updated)
        .execute(&mut *tx)
        .await?;

    // Commit the transaction.
    tx.commit().await?;

    Ok(result.rows_affected())
}
