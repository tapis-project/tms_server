#![forbid(unsafe_code)]

use poem::Request;
use poem_openapi::{ OpenApi, payload::Json, Object, ApiResponse };
use anyhow::Result;

use crate::utils::errors::HttpResult;
use crate::utils::db_statements::DELETE_DELEGATION;
use crate::utils::tms_utils::{self, RequestDebug, check_tenant_enabled};
use crate::utils::authz::{authorize, get_tenant_header, AuthzTypes, X_TMS_TENANT};
use log::{error, info};

use crate::RUNTIME_CTX;

// ***************************************************************************
//                          Request/Response Definiions
// ***************************************************************************
pub struct DeleteDelegationsApi;

// ***************************************************************************
//                          Request/Response Definiions
// ***************************************************************************
#[derive(Object)]
pub struct ReqDeleteDelegations
{
    client_id: String,
    client_user_id: String,
    tenant: String,
}

#[derive(Object, Debug)]
pub struct RespDeleteDelegations
{
    result_code: String,
    result_msg: String,
    num_deleted: u32,
}

// Implement the debug record trait for logging.
impl RequestDebug for ReqDeleteDelegations {   
    type Req = ReqDeleteDelegations;
    fn get_request_info(&self) -> String {
        let mut s = String::with_capacity(255);
        s.push_str("  Request body:");
        s.push_str("\n    client_id: ");
        s.push_str(&self.client_id);
        s.push_str("\n    client_user_id: ");
        s.push_str(&self.client_user_id);
        s.push_str("\n    tenant: ");
        s.push_str(&self.tenant);
        s
    }
}

// ------------------- HTTP Status Codes -------------------
#[derive(Debug, ApiResponse)]
enum TmsResponse {
    #[oai(status = 200)]
    Http200(Json<RespDeleteDelegations>),
    #[oai(status = 400)]
    Http400(Json<HttpResult>),
    #[oai(status = 401)]
    Http401(Json<HttpResult>),
    #[oai(status = 403)]
    Http403(Json<HttpResult>),
    #[oai(status = 500)]
    Http500(Json<HttpResult>),
}

fn make_http_200(resp: RespDeleteDelegations) -> TmsResponse {
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
impl DeleteDelegationsApi {
    #[oai(path = "/tms/delegations/del", method = "delete")]
    async fn delete_delegation_api(&self, http_req: &Request, req: Json<ReqDeleteDelegations>) -> TmsResponse {
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
        // Currently, only the tenant admin can delete a delegation record.
        // When user authentication is implemented, we'll add user-own 
        // authorization and any additional validation.
        let allowed = [AuthzTypes::TenantAdmin];
        let authz_result = authorize(http_req, &allowed).await;
        if !authz_result.is_authorized() {
            let msg = format!("ERROR: NOT AUTHORIZED to delete delegation for user {} to client {} in tenant {}.", 
                                      req.client_user_id, req.client_id, req.tenant);
            error!("{}", msg);
            return make_http_401(msg);
        }

        // -------------------- Process Request ----------------------
        // Process the request.
        match RespDeleteDelegations::process(http_req, &req).await {
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
impl RespDeleteDelegations {
    /// Create a new response.
    fn new(result_code: &str, result_msg: String, num_deleted: u32) -> Self {
        Self {result_code: result_code.to_string(), result_msg, num_deleted}}

    /// Process the request.
    async fn process(http_req: &Request, req: &ReqDeleteDelegations) -> Result<TmsResponse, anyhow::Error> {
        // Conditional logging depending on log level.
        tms_utils::debug_request(http_req, req);

        // Insert the new key record.
        let deletes = delete_delegation(req).await?;
        
        // Log result and return response.
        let msg = 
            if deletes < 1 {format!("Delegation to client {} NOT FOUND for user {} - Nothing deleted", req.client_id, req.client_user_id)}
            else {format!("User delegation deleted for user {} and client {}", req.client_user_id, req.client_id)};
        info!("{}", msg);
        Ok(make_http_200(RespDeleteDelegations::new("0", msg, deletes as u32)))
    }
}

// ***************************************************************************
//                          Private Functions
// ***************************************************************************
// ---------------------------------------------------------------------------
// delete_delegation:
// ---------------------------------------------------------------------------
async fn delete_delegation(req: &ReqDeleteDelegations) -> Result<u64> {
    // Get a connection to the db and start a transaction.  Uncommited transactions 
    // are automatically rolled back when they go out of scope. 
    // See https://docs.rs/sqlx/latest/sqlx/struct.Transaction.html.
    let mut tx = RUNTIME_CTX.db.begin().await?;

    // Deletion count.
    let mut deletes: u64 = 0;

    // Issue the db delete call.
    let result = sqlx::query(DELETE_DELEGATION)
        .bind(&req.client_id)
        .bind(&req.client_user_id)
        .bind(&req.tenant)
        .execute(&mut *tx)
        .await?;
    deletes += result.rows_affected();

    // Commit the transaction.
    tx.commit().await?;
    Ok(deletes)
}
