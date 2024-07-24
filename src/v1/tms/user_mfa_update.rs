#![forbid(unsafe_code)]

use poem::Request;
use poem_openapi::{ OpenApi, payload::Json, Object, param::Path, param::Query };
use anyhow::Result;
use futures::executor::block_on;

use crate::utils::db_statements::UPDATE_USER_MFA_ENABLED;
use crate::utils::tms_utils::{self, RequestDebug, timestamp_utc, timestamp_utc_to_str};
use crate::utils::authz::{authorize, AuthzTypes, get_tenant_header};
use log::{error, info};

use crate::RUNTIME_CTX;

// ***************************************************************************
//                          Request/Response Definiions
// ***************************************************************************
pub struct UpdateUserMfaApi;

// ***************************************************************************
//                          Request/Response Definiions
// ***************************************************************************
#[derive(Object)]
pub struct ReqUpdateUserMfa
{
    tms_user_id: String,
    tenant: String,
    enabled: bool,
}

#[derive(Object)]
pub struct RespUpdateUserMfa
{
    result_code: String,
    result_msg: String,
    fields_updated: i32,
}

// Implement the debug record trait for logging.
impl RequestDebug for ReqUpdateUserMfa {   
    type Req = ReqUpdateUserMfa;
    fn get_request_info(&self) -> String {
        // Get optional values in displayable form. 
        let enabled = format!("{:#?}", &self.enabled);

        let mut s = String::with_capacity(255);
        s.push_str("  Request body:");
        s.push_str("\n    tms_user_id: ");
        s.push_str(&self.tms_user_id);
        s.push_str("\n    tenant: ");
        s.push_str(&self.tenant);
        s.push_str("\n    enabled: ");
        s.push_str(enabled.as_str());
        s
    }
}

// ***************************************************************************
//                             OpenAPI Endpoint
// ***************************************************************************
#[OpenApi]
impl UpdateUserMfaApi {
    #[oai(path = "/tms/usermfa/:ptms_user_id", method = "patch")]
async fn update_client(&self, http_req: &Request, ptms_user_id: Path<String>, enabled: Query<bool>) -> Json<RespUpdateUserMfa> {
        // -------------------- Get Tenant Header --------------------
        // Get the required tenant header value.
        let hdr_tenant = match get_tenant_header(http_req) {
            Ok(t) => t,
            Err(e) => return Json(RespUpdateUserMfa::new("1", e.to_string(), 0)),
        };

        // Package the request parameters.        
        let req = 
            ReqUpdateUserMfa {tms_user_id: ptms_user_id.to_string(), tenant: hdr_tenant, enabled: *enabled};

        // -------------------- Authorize ----------------------------
        // Currently, only the tenant admin can create a user mfa record.
        // When user authentication is implemented, we'll add user-own 
        // authorization and any additional validation.
        let allowed = [AuthzTypes::TenantAdmin];
        let authz_result = authorize(http_req, &allowed);
        if !authz_result.is_authorized() {
            let msg = format!("NOT AUTHORIZED to update MFA for {} in tenant {}.", req.tms_user_id, req.tenant);
            error!("{}", msg);
            return Json(RespUpdateUserMfa::new("1", msg, 0));
        }

        // -------------------- Process Request ----------------------
        // Process the request.
        let resp = match RespUpdateUserMfa::process(http_req, &req) {
            Ok(r) => r,
            Err(e) => {
                let msg = "ERROR: ".to_owned() + e.to_string().as_str();
                error!("{}", msg);
                RespUpdateUserMfa::new("1", msg, 0)},
        };

        Json(resp)
    }
}

// ***************************************************************************
//                          Request/Response Methods
// ***************************************************************************
impl RespUpdateUserMfa {
    /// Create a new response.
    fn new(result_code: &str, result_msg: String, num_updates: i32,) -> Self {
        Self {result_code: result_code.to_string(), result_msg, fields_updated: num_updates}}

    /// Process the request.
    fn process(http_req: &Request, req: &ReqUpdateUserMfa) -> Result<RespUpdateUserMfa, anyhow::Error> {
        // Conditional logging depending on log level.
        tms_utils::debug_request(http_req, req);

        // Insert the new key record.
        let updates = block_on(update_client(req))?;
        
        // Log result and return response.
        let msg = format!("{} update(s) to tms_user_id {} completed", updates, req.tms_user_id);
        info!("{}", msg);
        Ok(RespUpdateUserMfa::new("0", msg, updates as i32))
    }
}

// ***************************************************************************
//                          Private Functions
// ***************************************************************************
// ---------------------------------------------------------------------------
// update_client:
// ---------------------------------------------------------------------------
async fn update_client(req: &ReqUpdateUserMfa) -> Result<u64> {
    // Get timestamp.
    let now = timestamp_utc();
    let current_ts = timestamp_utc_to_str(now);

    // Get a connection to the db and start a transaction.
    let mut tx = RUNTIME_CTX.db.begin().await?;

    // Update count.
    let mut updates: u64 = 0;

    // Issue the db update call.
    let result = sqlx::query(UPDATE_USER_MFA_ENABLED)
        .bind(req.enabled)
        .bind(current_ts)
        .bind(&req.tms_user_id)
        .bind(&req.tenant)
        .execute(&mut *tx)
        .await?;
    updates += result.rows_affected();

    // Commit the transaction.
    tx.commit().await?;
    Ok(updates)
}
