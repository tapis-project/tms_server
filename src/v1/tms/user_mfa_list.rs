#![forbid(unsafe_code)]

use poem::Request;
use poem_openapi::{ OpenApi, payload::Json, Object, ApiResponse };
use anyhow::Result;
use futures::executor::block_on;
use sqlx::Row;

use crate::utils::errors::HttpResult;

use crate::utils::authz::{authorize, AuthzTypes, get_tenant_header};
use crate::utils::db_statements::LIST_USER_MFA;
use crate::utils::tms_utils::{self, RequestDebug, check_tenant_enabled};
use log::error;

use crate::RUNTIME_CTX;

// ***************************************************************************
//                          Request/Response Definiions
// ***************************************************************************
pub struct ListUserMfaApi;

// ***************************************************************************
//                          Request/Response Definiions
// ***************************************************************************
#[derive(Object)]
struct ReqListUserMfa
{
    tenant: String,
}

#[derive(Object, Debug)]
pub struct RespListUserMfa
{
    result_code: String,
    result_msg: String,
    num_users: i32,
    users: Vec<UserMfaListElement>,
}

#[derive(Object, Debug)]
pub struct UserMfaListElement
{
    id: i32,
    tenant: String,
    tms_user_id: String,
    expires_at: String,
    enabled: i32,
    created: String,
    updated: String,
}

// Implement the debug record trait for logging.
impl RequestDebug for ReqListUserMfa {   
    type Req = ReqListUserMfa;
    fn get_request_info(&self) -> String {
        let mut s = String::with_capacity(255);
        s.push_str("  Request body:");
        s.push_str("\n    tenant: ");
        s.push_str(&self.tenant);
        s
    }
}

// ------------------- HTTP Status Codes -------------------
#[derive(Debug, ApiResponse)]
enum TmsResponse {
    #[oai(status = 200)]
    Http200(Json<RespListUserMfa>),
    #[oai(status = 400)]
    Http400(Json<HttpResult>),
    #[oai(status = 401)]
    Http401(Json<HttpResult>),
    #[oai(status = 500)]
    Http500(Json<HttpResult>),
}

fn make_http_200(resp: RespListUserMfa) -> TmsResponse {
    TmsResponse::Http200(Json(resp))
}
fn make_http_400(msg: String) -> TmsResponse {
    TmsResponse::Http400(Json(HttpResult::new(400.to_string(), msg)))
}
fn make_http_401(msg: String) -> TmsResponse {
    TmsResponse::Http401(Json(HttpResult::new(401.to_string(), msg)))
}
fn make_http_500(msg: String) -> TmsResponse {
    TmsResponse::Http500(Json(HttpResult::new(500.to_string(), msg)))    
}

// ***************************************************************************
//                             OpenAPI Endpoint
// ***************************************************************************
#[OpenApi]
impl ListUserMfaApi {
    #[oai(path = "/tms/usermfa/list", method = "get")]
    async fn get_users(&self, http_req: &Request) -> TmsResponse {
        // -------------------- Get Tenant Header --------------------
        // Get the required tenant header value.
        let hdr_tenant = match get_tenant_header(http_req) {
            Ok(t) => t,
            Err(e) => return make_http_400(e.to_string()),
        };
        
        // Check tenant.
        if !check_tenant_enabled(&hdr_tenant) {
            return make_http_400("Tenant not enabled.".to_string());
        }

        // Package the request parameters.        
        let req = ReqListUserMfa {tenant: hdr_tenant};
        
        // -------------------- Authorize ----------------------------
        // Only the tenant admin can query user records.
        let allowed = [AuthzTypes::TenantAdmin];
        let authz_result = authorize(http_req, &allowed);
        if !authz_result.is_authorized() {
            let msg = format!("ERROR: NOT AUTHORIZED to list user MFA information in tenant {}.", req.tenant);
            error!("{}", msg);
            return make_http_401(msg);
        }

        // -------------------- Process Request ----------------------
        // Process the request.
        match RespListUserMfa::process(http_req, &req) {
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
impl UserMfaListElement {
    /// Create response elements.
    #[allow(clippy::too_many_arguments)]
    fn new(id: i32, tenant: String, tms_user_id: String, expires_at: String, 
           enabled: i32, created: String, updated: String) -> Self {
        Self {id, tenant, tms_user_id, expires_at, enabled, created, updated}
    }
}

impl RespListUserMfa {
    /// Create a new response.
    #[allow(clippy::too_many_arguments)]
    fn new(result_code: &str, result_msg: String, num_users: i32, users: Vec<UserMfaListElement>) 
    -> Self {
        Self {result_code: result_code.to_string(), result_msg, num_users, users}
        }

    /// Process the request.
    fn process(http_req: &Request, req: &ReqListUserMfa) -> Result<TmsResponse, anyhow::Error> {
        // Conditional logging depending on log level.
        tms_utils::debug_request(http_req, req);

        // Search for the tenant/users in the database.  
        let users = block_on(list_mfa_users(req))?;
        Ok(make_http_200(Self::new("0", "success".to_string(), 
                                        users.len() as i32, users)))
    }
}

// ***************************************************************************
//                          Private Functions
// ***************************************************************************
// ---------------------------------------------------------------------------
// list_mfa_users:
// ---------------------------------------------------------------------------
async fn list_mfa_users(req: &ReqListUserMfa) -> Result<Vec<UserMfaListElement>> {
    // Get a connection to the db and start a transaction.  Uncommited transactions 
    // are automatically rolled back when they go out of scope. 
    // See https://docs.rs/sqlx/latest/sqlx/struct.Transaction.html.
    let mut tx = RUNTIME_CTX.db.begin().await?;
    
    // Create the select statement.
    let rows = sqlx::query(LIST_USER_MFA)
        .bind(&req.tenant)
        .fetch_all(&mut *tx)
        .await?;

    // Commit the transaction.
    tx.commit().await?;

    // Collect the row data into element objects.
    let mut element_list: Vec<UserMfaListElement> = vec!();
    for row in rows {
        let elem = UserMfaListElement::new(
                 row.get(0), row.get(1), row.get(2), 
        row.get(3), row.get(4), row.get(5), 
            row.get(6));
        element_list.push(elem);
    }

    Ok(element_list)
}
