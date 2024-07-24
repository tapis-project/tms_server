#![forbid(unsafe_code)]

use poem::Request;
use poem_openapi::{ OpenApi, payload::Json, Object, param::Path };
use anyhow::{Result, anyhow};
use futures::executor::block_on;
use sqlx::Row;

use crate::utils::authz::{authorize, AuthzTypes, get_tenant_header};
use crate::utils::db_statements::GET_USER_MFA;
use crate::utils::tms_utils::{self, RequestDebug};
use crate::utils::db_types::UserMfa;
use log::error;

use crate::RUNTIME_CTX;

// ***************************************************************************
//                          Request/Response Definiions
// ***************************************************************************
pub struct GetUserMfaApi;

// ***************************************************************************
//                          Request/Response Definiions
// ***************************************************************************
#[derive(Object)]
struct ReqGetUserMfa
{
    tms_user_id: String,
    tenant: String,
}

#[derive(Object)]
pub struct RespGetUserMfa
{
    result_code: String,
    result_msg: String,
    id: i32,
    tenant: String,
    tms_user_id: String,
    expires_at: String,
    enabled: i32,
    created: String,
    updated: String,
}

// Implement the debug record trait for logging.
impl RequestDebug for ReqGetUserMfa {   
    type Req = ReqGetUserMfa;
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
impl GetUserMfaApi {
    #[oai(path = "/tms/usermfa/:ptms_user_id", method = "get")]
    async fn get_client(&self, http_req: &Request, ptms_user_id: Path<String>) -> Json<RespGetUserMfa> {
        // -------------------- Get Tenant Header --------------------
        // Get the required tenant header value.
        let hdr_tenant = match get_tenant_header(http_req) {
            Ok(t) => t,
            Err(e) => return Json(RespGetUserMfa::new("1", e.to_string(), 0, "".to_string(), 
                                         ptms_user_id.to_string(), "".to_string(), 0, "".to_string(), "".to_string())),
        };
        
        // Package the request parameters.        
        let req = ReqGetUserMfa {tms_user_id: ptms_user_id.to_string(), tenant: hdr_tenant};
        
        // -------------------- Authorize ----------------------------
        // Currently, only the tenant admin can create a user mfa record.
        // When user authentication is implemented, we'll add user-own 
        // authorization and any additional validation.
        let allowed = [AuthzTypes::TenantAdmin];
        let authz_result = authorize(http_req, &allowed);
        if !authz_result.is_authorized() {
            let msg = format!("NOT AUTHORIZED to view mfa information for user {} in tenant {}", req.tms_user_id, req.tenant);
            error!("{}", msg);
            return Json(RespGetUserMfa::new("1", msg, 0, req.tenant.clone(), req.tms_user_id.clone(), "".to_string(),
                                             0, "".to_string(), "".to_string()));
        }

        // -------------------- Process Request ----------------------
        // Process the request.
        let resp = match RespGetUserMfa::process(http_req, &req) {
            Ok(r) => r,
            Err(e) => {
                let msg = "ERROR: ".to_owned() + e.to_string().as_str();
                error!("{}", msg);
                RespGetUserMfa::new("1", msg, 0, req.tenant.clone(), req.tms_user_id.clone(), "".to_string(),
                                    0, "".to_string(), "".to_string())},
        };

        Json(resp)
    }
}

// ***************************************************************************
//                          Request/Response Methods
// ***************************************************************************
impl RespGetUserMfa {
    /// Create a new response.
    #[allow(clippy::too_many_arguments)]
    fn new(result_code: &str, result_msg: String, id: i32, tenant: String, tms_user_id: String, 
            expires_at: String, enabled: i32, created: String, updated: String) 
    -> Self {
            Self {result_code: result_code.to_string(), result_msg, 
                  id, tenant, tms_user_id, expires_at, enabled, created, updated}
        }

    /// Process the request.
    fn process(http_req: &Request, req: &ReqGetUserMfa) -> Result<RespGetUserMfa, anyhow::Error> {
        // Conditional logging depending on log level.
        tms_utils::debug_request(http_req, req);

        // Search for the tenant/client id in the database.  Not found was already 
        // The client_secret is never part of the response.
        let u = block_on(get_user_mfa(req))?;
        Ok(Self::new("0", "success".to_string(), u.id, u.tenant, 
                     u.tms_user_id, u.expires_at, u.enabled, u.created, u.updated))
    }
}

// ***************************************************************************
//                          Private Functions
// ***************************************************************************
// ---------------------------------------------------------------------------
// get_user_mfa:
// ---------------------------------------------------------------------------
async fn get_user_mfa(req: &ReqGetUserMfa) -> Result<UserMfa> {
    // Get a connection to the db and start a transaction.
    let mut tx = RUNTIME_CTX.db.begin().await?;
    
    // Create the select statement.
    let result = sqlx::query(GET_USER_MFA)
        .bind(req.tms_user_id.clone())
        .bind(req.tenant.clone())
        .fetch_optional(&mut *tx)
        .await?;

    // Commit the transaction.
    tx.commit().await?;

    // We may have found the user mfa.
    match result {
        Some(row) => {
            Ok(UserMfa::new(row.get(0), row.get(1), row.get(2), row.get(3), 
                           row.get(4), row.get(5), row.get(6)))
        },
        None => {
            Err(anyhow!("NOT_FOUND"))
        },
    }
}
