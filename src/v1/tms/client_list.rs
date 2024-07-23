#![forbid(unsafe_code)]

use poem::Request;
use poem_openapi::{ OpenApi, payload::Json, Object, param::Path };
use anyhow::Result;
use futures::executor::block_on;
use sqlx::Row;

use crate::utils::authz::{authorize, AuthzTypes};
use crate::utils::db_statements::LIST_CLIENTS;
use crate::utils::tms_utils::{self, RequestDebug};
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

#[derive(Object)]
pub struct RespListClient
{
    result_code: String,
    result_msg: String,
    num_clients: i32,
    clients: Vec<ClientListElement>,
}

#[derive(Object)]
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

// ***************************************************************************
//                             OpenAPI Endpoint
// ***************************************************************************
#[OpenApi]
impl ListClientApi {
    #[oai(path = "/tms/client/list/:ptenant", method = "get")]
    async fn get_client(&self, http_req: &Request, ptenant: Path<String>) -> Json<RespListClient> {
        // Package the path parameters.        
        let req = ReqListClient {tenant: ptenant.to_string()};
        
        // Only the client and tenant admin can query a client record.
        let allowed = [AuthzTypes::TenantAdmin];
        let authz_result = authorize(http_req, &allowed);
        if !authz_result.is_authorized() {
            let msg = format!("NOT AUTHORIZED to list clients in tenant {}.", req.tenant);
            error!("{}", msg);
            let resp = RespListClient::new("1", msg.as_str(), 0, vec!());
            return Json(resp);
        }

        // Make sure the path parms conform to the header values used for authorization.
        // Since we only all TenantAdmins to issue this call, we know the first parameter
        // is ignored by the following check function.
        if !authz_result.check_request_parms(&req.tenant, &req.tenant) {
            let msg = format!("NOT AUTHORIZED: The tenant path parameter ({}) differs from the tenant in the request header.", 
                                      req.tenant);
            error!("{}", msg);
            let resp = RespListClient::new("1", msg.as_str(), 0, vec!());
            return Json(resp);
        }

        // Process the request.
        let resp = match RespListClient::process(http_req, &req) {
            Ok(r) => r,
            Err(e) => {
                let msg = "ERROR: ".to_owned() + e.to_string().as_str();
                error!("{}", msg);
                RespListClient::new("1", msg.as_str(), 0, vec!())},
        };

        Json(resp)
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
    fn new(result_code: &str, result_msg: &str, num_clients: i32, clients: Vec<ClientListElement>) 
    -> Self {
        Self {result_code: result_code.to_string(), result_msg: result_msg.to_string(), 
              num_clients, clients}
        }

    /// Process the request.
    fn process(http_req: &Request, req: &ReqListClient) -> Result<RespListClient, anyhow::Error> {
        // Conditional logging depending on log level.
        tms_utils::debug_request(http_req, req);

        // Search for the tenant/client id in the database.  Not found was already 
        // The client_secret is never part of the response.
        let clients = block_on(list_clients(req))?;
        Ok(Self::new("0", "success", clients.len() as i32, clients))
    }
}

// ***************************************************************************
//                          Private Functions
// ***************************************************************************
// ---------------------------------------------------------------------------
// list_clients:
// ---------------------------------------------------------------------------
async fn list_clients(req: &ReqListClient) -> Result<Vec<ClientListElement>> {
    // Get a connection to the db and start a transaction.
    let mut tx = RUNTIME_CTX.db.begin().await?;
    
    // Create the select statement.
    let rows = sqlx::query(LIST_CLIENTS)
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

    // "SELECT id, tenant, app_name, app_version, client_id, enabled, created, updated ",
    // "FROM clients WHERE tenant = ?",


    // We found the client! Index 5 is the hashed secret, which the caller will never return.
    // Index 4 is the client_id, which gets set to NOT_FOUND if not client record was returned.
    // match result {
    //     Some(row) => {
    //         Ok(Client::new(row.get(0), row.get(1), row.get(2), row.get(3), row.get(4),
    //                        row.get(5), row.get(6), row.get(7), row.get(8)))
    //     },
    //     None => {
    //         Err(anyhow!("NOT_FOUND"))
    //     },
    // }
}
