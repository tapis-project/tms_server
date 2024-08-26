#![forbid(unsafe_code)]

use poem::Request;
use poem_openapi::{ OpenApi, payload::Json, Object, ApiResponse };
use anyhow::Result;
use futures::executor::block_on;

use crate::utils::errors::HttpResult;
use crate::utils::db_statements::UPDATE_USER_HOST_EXPIRY;
use crate::utils::tms_utils::{self, RequestDebug, timestamp_utc, timestamp_utc_to_str, calc_expires_at};
use crate::utils::authz::{authorize, AuthzTypes, get_tenant_header, X_TMS_TENANT};
use log::{error, info};

use crate::RUNTIME_CTX;

// ***************************************************************************
//                          Request/Response Definiions
// ***************************************************************************
pub struct UpdateUserHostsApi;

// ***************************************************************************
//                          Request/Response Definiions
// ***************************************************************************
#[derive(Object)]
pub struct ReqUpdateUserHosts
{
    tms_user_id: String,
    tenant: String,
    host: String,
    host_account: String,
    ttl_minutes: i32,  // negative means i32::MAX
}

#[derive(Object, Debug)]
pub struct RespUpdateUserHosts
{
    result_code: String,
    result_msg: String,
    fields_updated: i32,
    expires_at: String,
}

// Implement the debug record trait for logging.
impl RequestDebug for ReqUpdateUserHosts {   
    type Req = ReqUpdateUserHosts;
    fn get_request_info(&self) -> String {
        let mut s = String::with_capacity(255);
        s.push_str("  Request body:");
        s.push_str("\n    tms_user_id: ");
        s.push_str(&self.tms_user_id);
        s.push_str("\n    tenant: ");
        s.push_str(&self.tenant);
        s.push_str("\n    host: ");
        s.push_str(&self.host);
        s.push_str("\n    host_account: ");
        s.push_str(&self.host_account);
        s.push_str("\n    ttl_minutes: ");
        s.push_str(&self.ttl_minutes.to_string());
        s
    }
}

// ------------------- HTTP Status Codes -------------------
#[derive(Debug, ApiResponse)]
enum TmsResponse {
    #[oai(status = 200)]
    Http200(Json<RespUpdateUserHosts>),
    #[oai(status = 400)]
    Http400(Json<HttpResult>),
    #[oai(status = 401)]
    Http401(Json<HttpResult>),
    #[oai(status = 403)]
    Http403(Json<HttpResult>),
    #[oai(status = 500)]
    Http500(Json<HttpResult>),
}

fn make_http_200(resp: RespUpdateUserHosts) -> TmsResponse {
    TmsResponse::Http200(Json(resp))
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
impl UpdateUserHostsApi {
    #[oai(path = "/tms/userhosts/upd", method = "patch")]
async fn update_client(&self, http_req: &Request, req: Json<ReqUpdateUserHosts>) -> TmsResponse {
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
    
        // -------------------- Authorize ----------------------------
        // Currently, only the tenant admin can create a user hosts record.
        // When user authentication is implemented, we'll add user-own 
        // authorization and any additional validation.
        let allowed = [AuthzTypes::TenantAdmin];
        let authz_result = authorize(http_req, &allowed);
        if !authz_result.is_authorized() {
            let msg = format!("ERROR: NOT AUTHORIZED to update host for user {} in tenant {}.", req.tms_user_id, req.tenant);
            error!("{}", msg);
            return make_http_401(msg);
        }

        // -------------------- Process Request ----------------------
        // Process the request.
        match RespUpdateUserHosts::process(http_req, &req) {
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
impl RespUpdateUserHosts {
    /// Create a new response.
    fn new(result_code: &str, result_msg: String, num_updates: i32, expires_at: String) -> Self {
        Self {result_code: result_code.to_string(), result_msg, fields_updated: num_updates, expires_at}}

    /// Process the request.
    fn process(http_req: &Request, req: &ReqUpdateUserHosts) -> Result<TmsResponse, anyhow::Error> {
        // Conditional logging depending on log level.
        tms_utils::debug_request(http_req, req);

        // Insert the new key record.
        let (updates, expires_at) = block_on(update_user_host(req))?;
        
        // Log result and return response.
        let msg = format!("{} update(s) to tms_user_id {} completed", updates, req.tms_user_id);
        info!("{}", msg);
        Ok(make_http_200(RespUpdateUserHosts::new("0", msg, updates as i32, expires_at)))
    }
}

// ***************************************************************************
//                          Private Functions
// ***************************************************************************
// ---------------------------------------------------------------------------
// update_user_host:
// ---------------------------------------------------------------------------
async fn update_user_host(req: &ReqUpdateUserHosts) -> Result<(u64, String)> {
    // Get timestamp.
    let now = timestamp_utc();
    let current_ts = timestamp_utc_to_str(now);
    let ttl_minutes = if req.ttl_minutes < 0 {i32::MAX} else {req.ttl_minutes};
    let expires_at = calc_expires_at(now, ttl_minutes);

    // Get a connection to the db and start a transaction.  Uncommited transactions 
    // are automatically rolled back when they go out of scope. 
    // See https://docs.rs/sqlx/latest/sqlx/struct.Transaction.html.
    let mut tx = RUNTIME_CTX.db.begin().await?;

    // Update count.
    let mut updates: u64 = 0;

    // Issue the db update call.
    let result = sqlx::query(UPDATE_USER_HOST_EXPIRY)
        .bind(&expires_at)
        .bind(current_ts)
        .bind(&req.tms_user_id)
        .bind(&req.tenant)
        .bind(&req.host)
        .bind(&req.host_account)
        .execute(&mut *tx)
        .await?;
    updates += result.rows_affected();

    // Commit the transaction.
    tx.commit().await?;
    Ok((updates, expires_at))
}
