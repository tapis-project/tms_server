#![forbid(unsafe_code)]

use poem::Request;
use poem_openapi::{ OpenApi, payload::Json, Object, ApiResponse };
use anyhow::Result;
use futures::executor::block_on;
use sqlx::Row;

use crate::utils::errors::HttpResult;
use crate::utils::authz::{authorize, get_tenant_header, AuthzResult, AuthzTypes};
use crate::utils::db_statements::LIST_CLIENTS_TEMPLATE;
use crate::utils::tms_utils::{self, RequestDebug, sql_substitute_client_constraint, check_tenant_enabled};
use log::error;

use crate::RUNTIME_CTX;

// ***************************************************************************
//                          Request/Response Definiions
// ***************************************************************************
pub struct ListClientApi;

// ***************************************************************************
//                          Request/Response Definiions
// ***************************************************************************
#[derive(Object)]
struct ReqListClient
{
    tenant: String,
}

#[derive(Object, Debug)]
pub struct RespListClient
{
    result_code: String,
    result_msg: String,
    num_clients: i32,
    clients: Vec<ClientListElement>,
}

#[derive(Object, Debug)]
pub struct ClientListElement
{
    id: i32,
    tenant: String,
    app_name: String,
    app_version: String,
    client_id: String,
    enabled: i32,
    created: String,
    updated: String,
}

// Implement the debug record trait for logging.
impl RequestDebug for ReqListClient {   
    type Req = ReqListClient;
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
    Http200(Json<RespListClient>),
    #[oai(status = 400)]
    Http400(Json<HttpResult>),
    #[oai(status = 401)]
    Http401(Json<HttpResult>),
    #[oai(status = 500)]
    Http500(Json<HttpResult>),
}

fn make_http_200(resp: RespListClient) -> TmsResponse {
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
impl ListClientApi {
    #[oai(path = "/tms/client/list", method = "get")]
    async fn get_clients(&self, http_req: &Request) -> TmsResponse {
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
        let req = ReqListClient {tenant: hdr_tenant};
        
        // -------------------- Authorize ----------------------------
        // Only the tenant admin can query all client records; 
        // a client can query their own records.
        let allowed = [AuthzTypes::ClientOwn, AuthzTypes::TenantAdmin];
        let authz_result = authorize(http_req, &allowed).await;
        if !authz_result.is_authorized() {
            let msg = format!("ERROR: NOT AUTHORIZED to list clients in tenant {}.", req.tenant);
            error!("{}", msg);
            return make_http_401(msg);
        }

        // -------------------- Process Request ----------------------
        // Process the request.
        match RespListClient::process(http_req, &req, &authz_result) {
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
impl ClientListElement {
    /// Create response elements.
    #[allow(clippy::too_many_arguments)]
    fn new(id: i32, tenant: String, app_name: String, app_version: String, 
           client_id: String, enabled: i32, created: String, updated: String) -> Self {
        Self {id, tenant, app_name, app_version, client_id, enabled, created, updated}
    }
}

impl RespListClient {
    /// Create a new response.
    #[allow(clippy::too_many_arguments)]
    fn new(result_code: &str, result_msg: String, num_clients: i32, clients: Vec<ClientListElement>) 
    -> Self {
        Self {result_code: result_code.to_string(), result_msg, num_clients, clients}
        }

    /// Process the request.
    fn process(http_req: &Request, req: &ReqListClient, authz_result: &AuthzResult) -> Result<TmsResponse, anyhow::Error> {
        // Conditional logging depending on log level.
        tms_utils::debug_request(http_req, req);

        // Search for the tenant/client ids in the database.  
        // The client_secret is never part of the response.
        match block_on(list_clients(authz_result, req)) {
            Ok(clients) =>
                Ok(make_http_200(Self::new("0", "success".to_string(), clients.len() as i32, clients))),
            Err(e) => Err(e),
        }
    }
}

// ***************************************************************************
//                          Private Functions
// ***************************************************************************
// ---------------------------------------------------------------------------
// list_clients:
// ---------------------------------------------------------------------------
async fn list_clients(authz_result: &AuthzResult, req: &ReqListClient) -> Result<Vec<ClientListElement>> {
    // Substitute the placeholder in the query template.  Uncommited transactions 
    // are automatically rolled back when they go out of scope. 
    // See https://docs.rs/sqlx/latest/sqlx/struct.Transaction.html.
    let sql_query = sql_substitute_client_constraint(LIST_CLIENTS_TEMPLATE, authz_result); 

    // Get a connection to the db and start a transaction.
    let mut tx = RUNTIME_CTX.db.begin().await?;
    
    // Create the select statement.
    let rows = sqlx::query(&sql_query)
        .bind(req.tenant.clone())
        .fetch_all(&mut *tx)
        .await?;

    // Commit the transaction.
    tx.commit().await?;

    // Collect the row data into element objects.
    let mut element_list: Vec<ClientListElement> = vec!();
    for row in rows {
        let elem = ClientListElement::new(
                 row.get(0), row.get(1), row.get(2), 
        row.get(3), row.get(4), row.get(5), 
            row.get(6), row.get(7));
        element_list.push(elem);
    }

    Ok(element_list)
}
