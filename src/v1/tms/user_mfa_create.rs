#![forbid(unsafe_code)]

use poem::Request;
use poem_openapi::{ OpenApi, payload::Json, Object };
use anyhow::Result;
use futures::executor::block_on;

use crate::utils::db_statements::INSERT_USER_MFA;
use crate::utils::db_types::UserMfaInput;
use crate::utils::authz::{authorize, AuthzTypes, get_tenant_header, X_TMS_TENANT}; 
use crate::utils::tms_utils::{self, timestamp_utc, timestamp_utc_to_str, calc_expires_at, RequestDebug};
use log::{error, info};

use crate::RUNTIME_CTX;

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
    ttl_minutes: u32,  // 0 means unlimited
}

#[derive(Object)]
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

// ***************************************************************************
//                             OpenAPI Endpoint
// ***************************************************************************
#[OpenApi]
impl CreateUserMfaApi {
    #[oai(path = "/tms/usermfa", method = "post")]
    async fn create_client(&self, http_req: &Request, req: Json<ReqCreateUserMfa>) -> Json<RespCreateUserMfa> {
        // -------------------- Get Tenant Header --------------------
        // Get the required tenant header value.
        let hdr_tenant = match get_tenant_header(http_req) {
            Ok(t) => t,
            Err(e) => return Json(RespCreateUserMfa::new("1", e.to_string(), req.tms_user_id.clone(), "".to_string(), false)),
        };

        // Check that the tenant specified in the header is the same as the one in the request body.
        if hdr_tenant != req.tenant {
            let msg = format!("The tenant in the {} header ({}) does not match the tenent in the request body ({})", 
                                        X_TMS_TENANT, hdr_tenant, req.tenant);
            error!("{}", msg);
            return Json(RespCreateUserMfa::new("1", msg, req.tms_user_id.clone(), "".to_string(), false));
        }

        // -------------------- Authorize ----------------------------
        // Currently, only the tenant admin can create a user mfa record.
        // When user authentication is implemented, we'll add user-own 
        // authorization and any additional validation.
        let allowed = [AuthzTypes::TenantAdmin];
        let authz_result = authorize(http_req, &allowed);
        if !authz_result.is_authorized() {
            let msg = format!("NOT AUTHORIZED to add a user MFA record in tenant {}.", req.tenant);
            error!("{}", msg);
            return Json(RespCreateUserMfa::new("1", msg, req.tms_user_id.clone(), "".to_string(), false));
        }

        // -------------------- Process Request ----------------------
        let resp = match RespCreateUserMfa::process(http_req, &req) {
            Ok(r) => r,
            Err(e) => {
                let msg = "ERROR: ".to_owned() + e.to_string().as_str();
                error!("{}", msg);
                RespCreateUserMfa::new("1", msg, req.tms_user_id.clone(), "".to_string(), false)},
        };

        Json(resp)
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
    fn process(http_req: &Request, req: &ReqCreateUserMfa) -> Result<RespCreateUserMfa, anyhow::Error> {
        // Conditional logging depending on log level.
        tms_utils::debug_request(http_req, req);

        // ------------------------ Time Values ------------------------  
        let ttl_minutes: i32 = match req.ttl_minutes.try_into(){
            Ok(num) => num,
            Err(_) => i32::MAX,
        };

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
        block_on(insert_new_client(input_record))?;
        info!("MFA for user '{}' created in tenant '{}' with experation at {}.", 
              req.tms_user_id, req.tenant, expires_at.clone());
        
        // Return the secret represented in hex.
        Ok(Self::new("0", "success".to_string(), req.tms_user_id.clone(), expires_at, true))
    }
}

// ***************************************************************************
//                          Private Functions
// ***************************************************************************
// ---------------------------------------------------------------------------
// insert_new_client:
// ---------------------------------------------------------------------------
async fn insert_new_client(rec: UserMfaInput) -> Result<u64> {
    // Get a connection to the db and start a transaction.  Uncommited transactions 
    // are automatically rolled back when they go out of scope. 
    // See https://docs.rs/sqlx/latest/sqlx/struct.Transaction.html.
    let mut tx = RUNTIME_CTX.db.begin().await?;
    
    // Create the insert statement.
    let result = sqlx::query(INSERT_USER_MFA)
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
