#![forbid(unsafe_code)]

use poem::Request;
use poem_openapi::{ OpenApi, payload::Json, Object, param::Path };
use anyhow::Result;
use futures::executor::block_on;

use crate::utils::db_statements::DELETE_USER_MFA;
use crate::utils::tms_utils::{self, RequestDebug};
use crate::utils::authz::{authorize, get_tenant_header, AuthzTypes};
use log::{error, info};

use crate::RUNTIME_CTX;

// ***************************************************************************
//                          Request/Response Definiions
// ***************************************************************************
pub struct DeleteUserMfaApi;

// ***************************************************************************
//                          Request/Response Definiions
// ***************************************************************************
#[derive(Object)]
pub struct ReqDeleteUserMfa
{
    tms_user_id: String,
    tenant: String,
}

#[derive(Object)]
pub struct RespDeleteUserMfa
{
    result_code: String,
    result_msg: String,
    num_deleted: u32,
}

// Implement the debug record trait for logging.
impl RequestDebug for ReqDeleteUserMfa {   
    type Req = ReqDeleteUserMfa;
    fn get_request_info(&self) -> String {
        let mut s = String::with_capacity(255);
        s.push_str("  Request body:");
        s.push_str("\n    tms_user_id: ");
        s.push_str(&self.tms_user_id);
        s.push_str("\n    tenant: ");
        s.push_str(&self.tenant);
        s
    }
}

// ***************************************************************************
//                             OpenAPI Endpoint
// ***************************************************************************
#[OpenApi]
impl DeleteUserMfaApi {
    #[oai(path = "/tms/usermfa/:tms_user_id", method = "delete")]
    async fn delete_client(&self, http_req: &Request, tms_user_id: Path<String>) -> Json<RespDeleteUserMfa> {
        // -------------------- Get Tenant Header --------------------
        // Get the required tenant header value.
        let hdr_tenant = match get_tenant_header(http_req) {
            Ok(t) => t,
            Err(e) => return Json(RespDeleteUserMfa::new("1", e.to_string(), 0)),
        };
        
        // Package the request parameters.
        let req = ReqDeleteUserMfa {tms_user_id: tms_user_id.to_string(), tenant: hdr_tenant};

        // -------------------- Authorize ----------------------------
        // Currently, only the tenant admin can create a user mfa record.
        // When user authentication is implemented, we'll add user-own 
        // authorization and any additional validation.
        let allowed = [AuthzTypes::TenantAdmin];
        let authz_result = authorize(http_req, &allowed);
        if !authz_result.is_authorized() {
            let msg = format!("ERROR: NOT AUTHORIZED to delete MFA for user {} in tenant {}.", req.tms_user_id, req.tenant);
            error!("{}", msg);
            return Json(RespDeleteUserMfa::new("1", msg, 0));
        }

        // -------------------- Process Request ----------------------
        // Process the request.
        let resp = match RespDeleteUserMfa::process(http_req, &req) {
            Ok(r) => r,
            Err(e) => {
                let msg = "ERROR: ".to_owned() + e.to_string().as_str();
                error!("{}", msg);
                RespDeleteUserMfa::new("1", msg, 0)},
        };

        Json(resp)
    }
}

// ***************************************************************************
//                          Request/Response Methods
// ***************************************************************************
impl RespDeleteUserMfa {
    /// Create a new response.
    fn new(result_code: &str, result_msg: String, num_deleted: u32) -> Self {
        Self {result_code: result_code.to_string(), result_msg, num_deleted}}

    /// Process the request.
    fn process(http_req: &Request, req: &ReqDeleteUserMfa) -> Result<RespDeleteUserMfa, anyhow::Error> {
        // Conditional logging depending on log level.
        tms_utils::debug_request(http_req, req);

        // Insert the new key record.
        let deletes = block_on(delete_user_mfa(req))?;
        
        // Log result and return response.
        let msg = 
            if deletes < 1 {format!("MFA user {} NOT FOUND - Nothing deleted", req.tms_user_id)}
            else {format!("MFA user {} deleted", req.tms_user_id)};
        info!("{}", msg);
        Ok(RespDeleteUserMfa::new("0", msg, deletes as u32))
    }
}

// ***************************************************************************
//                          Private Functions
// ***************************************************************************
// ---------------------------------------------------------------------------
// delete_client:
// ---------------------------------------------------------------------------
async fn delete_user_mfa(req: &ReqDeleteUserMfa) -> Result<u64> {
    // Get a connection to the db and start a transaction.  Uncommited transactions 
    // are automatically rolled back when they go out of scope. 
    // See https://docs.rs/sqlx/latest/sqlx/struct.Transaction.html.
    let mut tx = RUNTIME_CTX.db.begin().await?;

    // Deletion count.
    let mut deletes: u64 = 0;

    // Issue the db delete call.
    let result = sqlx::query(DELETE_USER_MFA)
        .bind(&req.tms_user_id)
        .bind(&req.tenant)
        .execute(&mut *tx)
        .await?;
    deletes += result.rows_affected();

    // Commit the transaction.
    tx.commit().await?;
    Ok(deletes)
}
