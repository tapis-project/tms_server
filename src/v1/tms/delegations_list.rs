#![forbid(unsafe_code)]

use poem::Request;
use poem_openapi::{ OpenApi, payload::Json, Object, ApiResponse };
use anyhow::Result;
use sqlx::Row;

use crate::utils::errors::HttpResult;

use crate::utils::authz::{authorize, AuthzTypes, get_tenant_header};
use crate::utils::db_statements::LIST_DELEGATIONS;
use crate::utils::tms_utils::{self, RequestDebug, check_tenant_enabled};
use log::error;

use crate::RUNTIME_CTX;

// ***************************************************************************
//                          Request/Response Definiions
// ***************************************************************************
pub struct ListDelegationsApi;

// ***************************************************************************
//                          Request/Response Definiions
// ***************************************************************************
#[derive(Object)]
struct ReqListDelegations
{
    tenant: String,
}

#[derive(Object, Debug)]
pub struct RespListDelegations
{
    result_code: String,
    result_msg: String,
    num_users: i32,
    users: Vec<DelegationsListElement>,
}

#[derive(Object, Debug)]
pub struct DelegationsListElement
{
    id: i32,
    tenant: String,
    client_id: String,
    client_user_id: String,
    expires_at: String,
    created: String,
    updated: String,
}

// Implement the debug record trait for logging.
impl RequestDebug for ReqListDelegations {   
    type Req = ReqListDelegations;
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
    Http200(Json<RespListDelegations>),
    #[oai(status = 400)]
    Http400(Json<HttpResult>),
    #[oai(status = 401)]
    Http401(Json<HttpResult>),
    #[oai(status = 500)]
    Http500(Json<HttpResult>),
}

fn make_http_200(resp: RespListDelegations) -> TmsResponse {
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
impl ListDelegationsApi {
    #[oai(path = "/tms/delegations/list", method = "get")]
    async fn get_delegations(&self, http_req: &Request) -> TmsResponse {
        // -------------------- Get Tenant Header --------------------
        // Get the required tenant header value.
        let hdr_tenant = match get_tenant_header(http_req) {
            Ok(t) => t,
            Err(e) => return make_http_400(e.to_string()),
        };
        
        // Check tenant.
        if !check_tenant_enabled(&hdr_tenant).await {
            return make_http_400("Tenant not enabled.".to_string());
        }

        // Package the request parameters.        
        let req = ReqListDelegations {tenant: hdr_tenant};
        
        // -------------------- Authorize ----------------------------
        // Only the tenant admin can query a user delegation record.
        let allowed = [AuthzTypes::TenantAdmin];
        let authz_result = authorize(http_req, &allowed).await;
        if !authz_result.is_authorized() {
            let msg = format!("ERROR: NOT AUTHORIZED to list delegations in tenant {}.", req.tenant);
            error!("{}", msg);
            return make_http_401(msg);
        }

        // -------------------- Process Request ----------------------
        // Process the request.
        match RespListDelegations::process(http_req, &req).await {
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
impl DelegationsListElement {
    /// Create response elements.
    #[allow(clippy::too_many_arguments)]
    fn new(id: i32, tenant: String, client_id: String, client_user_id: String,  
           expires_at: String, created: String, updated: String) -> Self {
        Self {id, tenant, client_id, client_user_id, expires_at, created, updated}
    }
}

impl RespListDelegations {
    /// Create a new response.
    #[allow(clippy::too_many_arguments)]
    fn new(result_code: &str, result_msg: String, num_users: i32, users: Vec<DelegationsListElement>) 
    -> Self {
        Self {result_code: result_code.to_string(), result_msg, num_users, users}
        }

    /// Process the request.
    async fn process(http_req: &Request, req: &ReqListDelegations) -> Result<TmsResponse, anyhow::Error> {
        // Conditional logging depending on log level.
        tms_utils::debug_request(http_req, req);

        // Search for the tenant/client id in the database.  Not found was already 
        // The client_secret is never part of the response.
        let clients = list_delegations(req).await?;
        Ok(make_http_200(Self::new("0", "success".to_string(), 
                                        clients.len() as i32, clients)))
    }
}

// ***************************************************************************
//                          Private Functions
// ***************************************************************************
// ---------------------------------------------------------------------------
// list_delegations:
// ---------------------------------------------------------------------------
async fn list_delegations(req: &ReqListDelegations) -> Result<Vec<DelegationsListElement>> {
    // Get a connection to the db and start a transaction.  Uncommited transactions 
    // are automatically rolled back when they go out of scope. 
    // See https://docs.rs/sqlx/latest/sqlx/struct.Transaction.html.
    let mut tx = RUNTIME_CTX.db.begin().await?;
    
    // Create the select statement.
    let rows = sqlx::query(LIST_DELEGATIONS)
        .bind(req.tenant.clone())
        .fetch_all(&mut *tx)
        .await?;

    // Commit the transaction.
    tx.commit().await?;

    // Collect the row data into element objects.
    let mut element_list: Vec<DelegationsListElement> = vec!();
    for row in rows {
        let elem = DelegationsListElement::new(
                 row.get(0), row.get(1), row.get(2), 
        row.get(3), row.get(4), row.get(5), 
            row.get(6));
        element_list.push(elem);
    }

    Ok(element_list)
}
