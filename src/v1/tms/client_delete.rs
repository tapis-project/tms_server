#![forbid(unsafe_code)]

use poem::Request;
use poem_openapi::{ OpenApi, payload::Json, Object, param::Path };
use anyhow::Result;
use futures::executor::block_on;

use crate::utils::db_statements::DELETE_CLIENT;
use crate::utils::tms_utils::{self, RequestDebug};
use crate::utils::authz::{authorize, get_tenant_header, AuthzTypes};
use log::{error, info};

use crate::RUNTIME_CTX;

// ***************************************************************************
//                          Request/Response Definiions
// ***************************************************************************
pub struct DeleteClientApi;

// ***************************************************************************
//                          Request/Response Definiions
// ***************************************************************************
#[derive(Object)]
pub struct ReqDeleteClient
{
    client_id: String,
    tenant: String,
}

#[derive(Object)]
pub struct RespDeleteClient
{
    result_code: String,
    result_msg: String,
}

// Implement the debug record trait for logging.
impl RequestDebug for ReqDeleteClient {   
    type Req = ReqDeleteClient;
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
impl DeleteClientApi {
    #[oai(path = "/tms/client/:pclient_id", method = "delete")]
    async fn delete_client(&self, http_req: &Request, pclient_id: Path<String>) -> Json<RespDeleteClient> {
        // -------------------- Get Tenant Header --------------------
        // Get the required tenant header value.
        let hdr_tenant = match get_tenant_header(http_req) {
            Ok(t) => t,
            Err(e) => return Json(RespDeleteClient::new("1", e.to_string())),
        };
        
        // Package the request parameters.
        let req = ReqDeleteClient {client_id: pclient_id.to_string(), tenant: hdr_tenant};

        // -------------------- Authorize ----------------------------
        // Only the client and tenant admin can query a client record.
        let allowed = [AuthzTypes::ClientOwn, AuthzTypes::TenantAdmin];
        let authz_result = authorize(http_req, &allowed);
        if !authz_result.is_authorized() {
            let msg = format!("NOT AUTHORIZED to delete client {} in tenant {}.", req.client_id, req.tenant);
            error!("{}", msg);
            return Json(RespDeleteClient::new("1", msg));
        }

        // Make sure the request parms conform to the header values used for authorization.
        if !authz_result.check_hdr_id(&req.client_id) {
            let msg = format!("NOT AUTHORIZED: Payload parameters ({}@{}) differ from those in the request header.", 
                                      req.client_id, req.tenant);
            error!("{}", msg);
            return Json(RespDeleteClient::new("1", msg));
        }

        // -------------------- Process Request ----------------------
        // Process the request.
        let resp = match RespDeleteClient::process(http_req, &req) {
            Ok(r) => r,
            Err(e) => {
                let msg = "ERROR: ".to_owned() + e.to_string().as_str();
                error!("{}", msg);
                RespDeleteClient::new("1", msg)},
        };

        Json(resp)
    }
}

// ***************************************************************************
//                          Request/Response Methods
// ***************************************************************************
impl RespDeleteClient {
    /// Create a new response.
    fn new(result_code: &str, result_msg: String) -> Self {
        Self {result_code: result_code.to_string(), result_msg,}}

    /// Process the request.
    fn process(http_req: &Request, req: &ReqDeleteClient) -> Result<RespDeleteClient, anyhow::Error> {
        // Conditional logging depending on log level.
        tms_utils::debug_request(http_req, req);

        // Insert the new key record.
        let deletes = block_on(delete_client(req))?;
        
        // Log result and return response.
        let msg = 
            if deletes < 1 {format!("Client {} NOT deleted", req.client_id)}
            else {format!("Client {} deleted", req.client_id)};
        info!("{}", msg);
        Ok(RespDeleteClient::new("0", msg))
    }
}

// ***************************************************************************
//                          Private Functions
// ***************************************************************************
// ---------------------------------------------------------------------------
// delete_client:
// ---------------------------------------------------------------------------
async fn delete_client(req: &ReqDeleteClient) -> Result<u64> {
    // Get a connection to the db and start a transaction.
    let mut tx = RUNTIME_CTX.db.begin().await?;

    // Deletion count.
    let mut deletes: u64 = 0;

    // Issue the db delete call.
    let result = sqlx::query(DELETE_CLIENT)
        .bind(&req.client_id)
        .bind(&req.tenant)
        .execute(&mut *tx)
        .await?;
    deletes += result.rows_affected();

    // Commit the transaction.
    tx.commit().await?;
    Ok(deletes)
}
