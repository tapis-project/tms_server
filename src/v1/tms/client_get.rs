#![forbid(unsafe_code)]

use poem::Request;
use poem_openapi::{ OpenApi, payload::Json, Object, param::Path };
use anyhow::{Result, anyhow};
use futures::executor::block_on;
use sqlx::Row;

use crate::utils::authz::{authorize, AuthzTypes, get_tenant_header};
use crate::utils::db_statements::GET_CLIENT;
use crate::utils::tms_utils::{self, RequestDebug};
use crate::utils::db_types::Client;
use log::error;

use crate::RUNTIME_CTX;

// ***************************************************************************
//                          Request/Response Definiions
// ***************************************************************************
pub struct GetClientApi;

// ***************************************************************************
//                          Request/Response Definiions
// ***************************************************************************
#[derive(Object)]
struct ReqGetClient
{
    client_id: String,
    tenant: String,
}

#[derive(Object)]
pub struct RespGetClient
{
    result_code: String,
    result_msg: String,
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
impl RequestDebug for ReqGetClient {   
    type Req = ReqGetClient;
    fn get_request_info(&self) -> String {
        let mut s = String::with_capacity(255);
        s.push_str("  Request body:");
        s.push_str("\n    client_id: ");
        s.push_str(&self.client_id);
        s.push_str("\n    tenant: ");
        s.push_str(&self.tenant);
        s
    }
}

// ***************************************************************************
//                             OpenAPI Endpoint
// ***************************************************************************
#[OpenApi]
impl GetClientApi {
    #[oai(path = "/tms/client/:pclient_id", method = "get")]
    async fn get_client(&self, http_req: &Request, pclient_id: Path<String>) -> Json<RespGetClient> {
        // -------------------- Get Tenant Header --------------------
        // Get the required tenant header value.
        let hdr_tenant = match get_tenant_header(http_req) {
            Ok(t) => t,
            Err(e) => return Json(RespGetClient::new("1", e.to_string(), 0, "".to_string(), 
                                         "".to_string(), "".to_string(), pclient_id.to_string(), 0,  
                                         "".to_string(), "".to_string())),
        };
        
        // Package the request parameters.        
        let req = ReqGetClient {client_id: pclient_id.to_string(), tenant: hdr_tenant};
        
        // -------------------- Authorize ----------------------------
        // Only the client and tenant admin can query a client record.
        let allowed = [AuthzTypes::ClientOwn, AuthzTypes::TenantAdmin];
        let authz_result = authorize(http_req, &allowed);
        if !authz_result.is_authorized() {
            let msg = format!("NOT AUTHORIZED to view client {} in tenant {}.", req.client_id, req.tenant);
            error!("{}", msg);
            let resp = RespGetClient::new("1", msg, 0, req.tenant.clone(), "".to_string(), 
                                        "".to_string(), req.client_id.clone(), 0,  "".to_string(), "".to_string());
            return Json(resp);
        }

        // Make sure the path parms conform to the header values used for authorization.
        if !authz_result.check_hdr_id(&req.client_id) {
            let msg = format!("NOT AUTHORIZED: Path parameters ({}@{}) differ from those in the request header.", 
                                      req.client_id, req.tenant);
            error!("{}", msg);
            let resp = RespGetClient::new("1", msg, 0, req.tenant.clone(), "".to_string(), 
                                        "".to_string(), req.client_id.clone(), 0,  "".to_string(), "".to_string());
            return Json(resp);
        }

        // -------------------- Process Request ----------------------
        // Process the request.
        let resp = match RespGetClient::process(http_req, &req) {
            Ok(r) => r,
            Err(e) => {
                let msg = "ERROR: ".to_owned() + e.to_string().as_str();
                error!("{}", msg);
                RespGetClient::new("1", msg, 0, req.tenant.clone(), "".to_string(), "".to_string(),
                                   req.client_id.clone(), 0,  "".to_string(), "".to_string())},
        };

        Json(resp)
    }
}

// ***************************************************************************
//                          Request/Response Methods
// ***************************************************************************
impl RespGetClient {
    /// Create a new response.
    #[allow(clippy::too_many_arguments)]
    fn new(result_code: &str, result_msg: String, id: i32, tenant: String, app_name: String, 
            app_version: String, client_id: String, enabled: i32, created: String, updated: String) 
    -> Self {
            Self {result_code: result_code.to_string(), result_msg, 
              id, tenant, app_name, app_version, client_id, enabled, created, updated}
        }

    /// Process the request.
    fn process(http_req: &Request, req: &ReqGetClient) -> Result<RespGetClient, anyhow::Error> {
        // Conditional logging depending on log level.
        tms_utils::debug_request(http_req, req);

        // Search for the tenant/client id in the database.  Not found was already 
        // The client_secret is never part of the response.
        let client = block_on(get_client(req))?;
        Ok(Self::new("0", "success".to_string(), client.id, client.tenant, client.app_name, 
                     client.app_version, client.client_id, client.enabled, client.created, client.updated))
    }
}

// ***************************************************************************
//                          Private Functions
// ***************************************************************************
// ---------------------------------------------------------------------------
// get_client:
// ---------------------------------------------------------------------------
async fn get_client(req: &ReqGetClient) -> Result<Client> {
    // Get a connection to the db and start a transaction.
    let mut tx = RUNTIME_CTX.db.begin().await?;
    
    // Create the select statement.
    let result = sqlx::query(GET_CLIENT)
        .bind(req.client_id.clone())
        .bind(req.tenant.clone())
        .fetch_optional(&mut *tx)
        .await?;

    // Commit the transaction.
    tx.commit().await?;

    // We found the client! Index 5 is the hashed secret, which the caller will never return.
    // Index 4 is the client_id, which gets set to NOT_FOUND if not client record was returned.
    match result {
        Some(row) => {
            Ok(Client::new(row.get(0), row.get(1), row.get(2), row.get(3), row.get(4),
                           row.get(5), row.get(6), row.get(7), row.get(8)))
        },
        None => {
            Err(anyhow!("NOT_FOUND"))
        },
    }
}
