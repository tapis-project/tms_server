#![forbid(unsafe_code)]

use poem::Request;
use poem_openapi::{ OpenApi, payload::Json, Object, ApiResponse };
use anyhow::Result;
use futures::executor::block_on;

use crate::utils::errors::HttpResult;
use crate::utils::db_statements::DELETE_USER_HOST;
use crate::utils::tms_utils::{self, RequestDebug, check_tenant_enabled};
use crate::utils::authz::{authorize, get_tenant_header, AuthzTypes, X_TMS_TENANT};
use log::{error, info};

use crate::RUNTIME_CTX;

// ***************************************************************************
//                          Request/Response Definiions
// ***************************************************************************
pub struct DeleteUserHostsApi;

// ***************************************************************************
//                          Request/Response Definiions
// ***************************************************************************
#[derive(Object)]
pub struct ReqDeleteUserHosts
{
    tms_user_id: String,
    tenant: String,
    host: String,
    host_account: String,
}

#[derive(Object, Debug)]
pub struct RespDeleteUserHosts
{
    result_code: String,
    result_msg: String,
    num_deleted: u32,
}

// Implement the debug record trait for logging.
impl RequestDebug for ReqDeleteUserHosts {   
    type Req = ReqDeleteUserHosts;
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
        s
    }
}

// ------------------- HTTP Status Codes -------------------
#[derive(Debug, ApiResponse)]
enum TmsResponse {
    #[oai(status = 200)]
    Http200(Json<RespDeleteUserHosts>),
    #[oai(status = 400)]
    Http400(Json<HttpResult>),
    #[oai(status = 401)]
    Http401(Json<HttpResult>),
    #[oai(status = 403)]
    Http403(Json<HttpResult>),
    #[oai(status = 500)]
    Http500(Json<HttpResult>),
}

fn make_http_200(resp: RespDeleteUserHosts) -> TmsResponse {
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
impl DeleteUserHostsApi {
    #[oai(path = "/tms/userhosts/del", method = "delete")]
    async fn delete_user_host_api(&self, http_req: &Request, req: Json<ReqDeleteUserHosts>) -> TmsResponse {
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
        if !check_tenant_enabled(&hdr_tenant) {
            return make_http_400("Tenant not enabled.".to_string());
        }

        // -------------------- Authorize ----------------------------
        // Currently, only the tenant admin can delete a user hosts record.
        // When user authentication is implemented, we'll add user-own 
        // authorization and any additional validation.
        let allowed = [AuthzTypes::TenantAdmin];
        let authz_result = authorize(http_req, &allowed);
        if !authz_result.is_authorized() {
            let msg = format!("ERROR: NOT AUTHORIZED to delete host {} for user {} in tenant {}.", 
                                      req.host, req.tms_user_id, req.tenant);
            error!("{}", msg);
            return make_http_401(msg);
        }

        // -------------------- Process Request ----------------------
        // Process the request.
        match RespDeleteUserHosts::process(http_req, &req) {
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
impl RespDeleteUserHosts {
    /// Create a new response.
    fn new(result_code: &str, result_msg: String, num_deleted: u32) -> Self {
        Self {result_code: result_code.to_string(), result_msg, num_deleted}}

    /// Process the request.
    fn process(http_req: &Request, req: &ReqDeleteUserHosts) -> Result<TmsResponse, anyhow::Error> {
        // Conditional logging depending on log level.
        tms_utils::debug_request(http_req, req);

        // Insert the new key record.
        let deletes = block_on(delete_user_host(req))?;
        
        // Log result and return response.
        let msg = 
            if deletes < 1 {format!("Host {} NOT FOUND for user {} - Nothing deleted", req.host, req.tms_user_id)}
            else {format!("User host {} deleted for user {} and account {}", req.host, req.tms_user_id, req.host_account)};
        info!("{}", msg);
        Ok(make_http_200(RespDeleteUserHosts::new("0", msg, deletes as u32)))
    }
}

// ***************************************************************************
//                          Private Functions
// ***************************************************************************
// ---------------------------------------------------------------------------
// delete_user_host:
// ---------------------------------------------------------------------------
async fn delete_user_host(req: &ReqDeleteUserHosts) -> Result<u64> {
    // Get a connection to the db and start a transaction.  Uncommited transactions 
    // are automatically rolled back when they go out of scope. 
    // See https://docs.rs/sqlx/latest/sqlx/struct.Transaction.html.
    let mut tx = RUNTIME_CTX.db.begin().await?;

    // Deletion count.
    let mut deletes: u64 = 0;

    // Issue the db delete call.
    let result = sqlx::query(DELETE_USER_HOST)
        .bind(&req.tms_user_id)
        .bind(&req.tenant)
        .bind(&req.host)
        .bind(&req.host_account)
        .execute(&mut *tx)
        .await?;
    deletes += result.rows_affected();

    // Commit the transaction.
    tx.commit().await?;
    Ok(deletes)
}
