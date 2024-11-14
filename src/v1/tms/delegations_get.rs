#![forbid(unsafe_code)]

use poem::Request;
use poem_openapi::{ OpenApi, payload::Json, Object, param::Path, ApiResponse };
use anyhow::{Result, anyhow};
use futures::executor::block_on;
use sqlx::Row;

use crate::utils::errors::HttpResult;
use crate::utils::authz::{authorize, AuthzTypes, get_tenant_header};
use crate::utils::db_statements::GET_DELEGATION;
use crate::utils::tms_utils::{self, RequestDebug, check_tenant_enabled};
use crate::utils::db_types::Delegation;
use log::error;

use crate::RUNTIME_CTX;

// ***************************************************************************
//                          Request/Response Definiions
// ***************************************************************************
pub struct GetDelegationsApi;

// ***************************************************************************
//                          Request/Response Definiions
// ***************************************************************************
#[derive(Object)]
struct ReqGetDelegations
{
    id: i32,
    tenant: String,
}

#[derive(Object, Debug)]
pub struct RespGetDelegations
{
    result_code: String,
    result_msg: String,
    id: i32,
    tenant: String,
    client_id: String,
    client_user_id: String,
    expires_at: String,
    created: String,
    updated: String,
}

// Implement the debug record trait for logging.
impl RequestDebug for ReqGetDelegations {   
    type Req = ReqGetDelegations;
    fn get_request_info(&self) -> String {
        let mut s = String::with_capacity(255);
        s.push_str("  Request body:");
        s.push_str("\n    id: ");
        s.push_str(&self.id.to_string());
        s.push_str("\n    tenant: ");
        s.push_str(&self.tenant);
        s
    }
}

// ------------------- HTTP Status Codes -------------------
#[derive(Debug, ApiResponse)]
enum TmsResponse {
    #[oai(status = 200)]
    Http200(Json<RespGetDelegations>),
    #[oai(status = 400)]
    Http400(Json<HttpResult>),
    #[oai(status = 401)]
    Http401(Json<HttpResult>),
    #[oai(status = 404)]
    Http404(Json<HttpResult>),
    #[oai(status = 500)]
    Http500(Json<HttpResult>),
}

fn make_http_200(resp: RespGetDelegations) -> TmsResponse {
    TmsResponse::Http200(Json(resp))
}
fn make_http_400(msg: String) -> TmsResponse {
    TmsResponse::Http400(Json(HttpResult::new(400.to_string(), msg)))
}
fn make_http_401(msg: String) -> TmsResponse {
    TmsResponse::Http401(Json(HttpResult::new(401.to_string(), msg)))
}
fn make_http_404(msg: String) -> TmsResponse {
    TmsResponse::Http404(Json(HttpResult::new(404.to_string(), msg)))
}
fn make_http_500(msg: String) -> TmsResponse {
    TmsResponse::Http500(Json(HttpResult::new(500.to_string(), msg)))    
}

// ***************************************************************************
//                             OpenAPI Endpoint
// ***************************************************************************
#[OpenApi]
impl GetDelegationsApi {
    #[oai(path = "/tms/delegations/:id", method = "get")]
    async fn get_delegation_api(&self, http_req: &Request, id: Path<i32>) -> TmsResponse {
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
        let req = ReqGetDelegations {id: *id, tenant: hdr_tenant};
        
        // -------------------- Authorize ----------------------------
        // Currently, only the tenant admin can create a user host record.
        // When user authentication is implemented, we'll add user-own 
        // authorization and any additional validation.
        let allowed = [AuthzTypes::TenantAdmin];
        let authz_result = authorize(http_req, &allowed);
        if !authz_result.is_authorized() {
            let msg = format!("ERROR: NOT AUTHORIZED to view client delegation information for record #{} in tenant {}", 
                                      req.id, req.tenant);
            error!("{}", msg);
            return make_http_401(msg);
        }

        // -------------------- Process Request ----------------------
        // Process the request.
        match RespGetDelegations::process(http_req, &req) {
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
impl RespGetDelegations {
    /// Create a new response.
    #[allow(clippy::too_many_arguments)]
    fn new(result_code: &str, result_msg: String, id: i32, tenant: String, client_id: String, 
           client_user_id: String, expires_at: String, created: String, updated: String) 
    -> Self {
            Self {result_code: result_code.to_string(), result_msg, 
                  id, tenant, client_id, client_user_id, expires_at, created, updated}
        }

    /// Process the request.
    fn process(http_req: &Request, req: &ReqGetDelegations) -> Result<TmsResponse, anyhow::Error> {
        // Conditional logging depending on log level.
        tms_utils::debug_request(http_req, req);

        // Search for the tenant/client id in the database.  Not found was already 
        // The client_secret is never part of the response.
        let db_result = block_on(get_delegation(req));
        match db_result {
            Ok(u) => Ok(make_http_200(Self::new("0", "success".to_string(), u.id, u.tenant, 
                                        u.client_id, u.client_user_id, u.expires_at, u.created, u.updated))),
            Err(e) => {
                // Determine if this is a real db error or just record not found.
                let msg = e.to_string();
                if msg.contains("NOT_FOUND") {Ok(make_http_404(msg))} 
                  else {Err(e)}
            }
        }
    }
}

// ***************************************************************************
//                          Private Functions
// ***************************************************************************
// ---------------------------------------------------------------------------
// get_delegation:
// ---------------------------------------------------------------------------
async fn get_delegation(req: &ReqGetDelegations) -> Result<Delegation> {
    // Get a connection to the db and start a transaction.  Uncommited transactions 
    // are automatically rolled back when they go out of scope. 
    // See https://docs.rs/sqlx/latest/sqlx/struct.Transaction.html.
    let mut tx = RUNTIME_CTX.db.begin().await?;
    
    // Create the select statement.
    let result = sqlx::query(GET_DELEGATION)
        .bind(req.id)
        .bind(req.tenant.clone())
        .fetch_optional(&mut *tx)
        .await?;

    // Commit the transaction.
    tx.commit().await?;

    // We may have found the user host.
    match result {
        Some(row) => {
            Ok(Delegation::new(row.get(0), row.get(1), row.get(2), 
                               row.get(3),row.get(4), row.get(5), row.get(6)))
        },
        None => {
            Err(anyhow!("NOT_FOUND"))
        },
    }
}
