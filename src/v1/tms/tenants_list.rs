#![forbid(unsafe_code)]

use poem::Request;
use poem_openapi::{ OpenApi, payload::Json, Object, ApiResponse };
use anyhow::Result;
use futures::executor::block_on;
use sqlx::Row;

use crate::utils::errors::HttpResult;

use crate::utils::db_statements::LIST_TENANTS;
use crate::utils::tms_utils::{self, RequestDebug};
use log::error;

use crate::RUNTIME_CTX;

// ***************************************************************************
//                          Request/Response Definiions
// ***************************************************************************
pub struct ListTenantsApi;

// ***************************************************************************
//                          Request/Response Definiions
// ***************************************************************************
#[derive(Object)]
struct ReqListTenants
{
    // Empty for now, but kept as a placeholder for the eventual implementation
    // of query parameters that will filter this endpoint's output.
}

#[derive(Object, Debug)]
pub struct RespListTenants
{
    result_code: String,
    result_msg: String,
    num_tenants: i32,
    tenants: Vec<TenantsListElement>,
}

#[derive(Object, Debug)]
pub struct TenantsListElement
{
    id: i32,
    tenant: String,
    enabled: i32,
    created: String,
    updated: String,
}

// Implement the debug record trait for logging.
impl RequestDebug for ReqListTenants {   
    type Req = ReqListTenants;
    fn get_request_info(&self) -> String {
        let mut s = String::with_capacity(255);
        s.push_str("  Request body:");
        s
    }
}

// ------------------- HTTP Status Codes -------------------
#[derive(Debug, ApiResponse)]
enum TmsResponse {
    #[oai(status = 200)]
    Http200(Json<RespListTenants>),
    #[oai(status = 500)]
    Http500(Json<HttpResult>),
}

fn make_http_200(resp: RespListTenants) -> TmsResponse {
    TmsResponse::Http200(Json(resp))
}
fn make_http_500(msg: String) -> TmsResponse {
    TmsResponse::Http500(Json(HttpResult::new(500.to_string(), msg)))    
}

// ***************************************************************************
//                             OpenAPI Endpoint
// ***************************************************************************
#[OpenApi]
impl ListTenantsApi {
    #[oai(path = "/tms/tenants/list", method = "get")]
    async fn get_tenants(&self, http_req: &Request) -> TmsResponse {
        // Package the request parameters.        
        let req = ReqListTenants {};
        
        // -------------------- Process Request ----------------------
        // Process the request.
        match RespListTenants::process(http_req, &req) {
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
impl TenantsListElement {
    /// Create response elements.
    #[allow(clippy::too_many_arguments)]
    fn new(id: i32, tenant: String, enabled: i32, created: String, updated: String) -> Self {
        Self {id, tenant, enabled, created, updated}
    }
}

impl RespListTenants {
    /// Create a new response.
    #[allow(clippy::too_many_arguments)]
    fn new(result_code: &str, result_msg: String, num_tenants: i32, tenants: Vec<TenantsListElement>) 
    -> Self {
        Self {result_code: result_code.to_string(), result_msg, num_tenants, tenants}
        }

    /// Process the request.
    fn process(http_req: &Request, req: &ReqListTenants) -> Result<TmsResponse, anyhow::Error> {
        // Conditional logging depending on log level.
        tms_utils::debug_request(http_req, req);

        // Search for the tenant/users in the database.  
        let users = block_on(list_tenants(req))?;
        Ok(make_http_200(Self::new("0", "success".to_string(), 
                                        users.len() as i32, users)))
    }
}

// ***************************************************************************
//                          Private Functions
// ***************************************************************************
// ---------------------------------------------------------------------------
// list_tenants:
// ---------------------------------------------------------------------------
#[allow(unused_variables)]
async fn list_tenants(req: &ReqListTenants) -> Result<Vec<TenantsListElement>> {
    // Get a connection to the db and start a transaction.  Uncommited transactions 
    // are automatically rolled back when they go out of scope. 
    // See https://docs.rs/sqlx/latest/sqlx/struct.Transaction.html.
    let mut tx = RUNTIME_CTX.db.begin().await?;
    
    // Create the select statement.
    let rows = sqlx::query(LIST_TENANTS)
        .fetch_all(&mut *tx)
        .await?;

    // Commit the transaction.
    tx.commit().await?;

    // Collect the row data into element objects.
    let mut element_list: Vec<TenantsListElement> = vec!();
    for row in rows {
        let elem = TenantsListElement::new(
                    row.get(0), row.get(1), row.get(2), 
                    row.get(3), row.get(4));
        element_list.push(elem);
    }

    Ok(element_list)
}
