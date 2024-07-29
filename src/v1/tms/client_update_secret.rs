#![forbid(unsafe_code)]

use poem::Request;
use poem_openapi::{ OpenApi, payload::Json, Object, param::Path };
use anyhow::Result;
use futures::executor::block_on;

use crate::utils::db_statements::UPDATE_CLIENT_SECRET;
use crate::utils::tms_utils::{self, RequestDebug, create_hex_secret, hash_hex_secret, timestamp_utc, timestamp_utc_to_str};
use crate::utils::authz::{authorize, AuthzTypes, get_tenant_header};
use log::{error, info};

use crate::RUNTIME_CTX;

// ***************************************************************************
//                          Request/Response Definiions
// ***************************************************************************
pub struct UpdateClientSecretApi;

// ***************************************************************************
//                          Request/Response Definiions
// ***************************************************************************
#[derive(Object)]
pub struct ReqUpdateClientSecret
{
    client_id: String,
    tenant: String,
}

#[derive(Object)]
pub struct RespUpdateClientSecret
{
    result_code: String,
    result_msg: String,
    client_id: String,
    tenant: String,
    client_secret: String,
}

// Implement the debug record trait for logging.
impl RequestDebug for ReqUpdateClientSecret {   
    type Req = ReqUpdateClientSecret;
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
impl UpdateClientSecretApi {
    #[oai(path = "/tms/client/secret/:client_id", method = "patch")]
    async fn update_client(&self, http_req: &Request, client_id: Path<String>) -> Json<RespUpdateClientSecret> {
        // -------------------- Get Tenant Header --------------------
        // Get the required tenant header value.
        let hdr_tenant = match get_tenant_header(http_req) {
            Ok(t) => t,
            Err(e) => return Json(RespUpdateClientSecret::new("1", e.to_string(), client_id.to_string(), 
                                                                     "".to_string(), "".to_string())),
        };

        // Package the request parameters.
        let req = ReqUpdateClientSecret {client_id: client_id.to_string(), tenant: hdr_tenant};

        // -------------------- Authorize ----------------------------
        // Only the client and tenant admin can query a client record.
        let allowed = [AuthzTypes::ClientOwn, AuthzTypes::TenantAdmin];
        let authz_result = authorize(http_req, &allowed);
        if !authz_result.is_authorized() {
            let msg = format!("NOT AUTHORIZED to update client {} in tenant {}.", req.client_id, req.tenant);
            error!("{}", msg);
            return Json(RespUpdateClientSecret::new("1", msg, req.client_id, req.tenant, "".to_string()));
        }

        // Make sure the request parms conform to the header values used for authorization.
        if !authz_result.check_hdr_id(&req.client_id) {
            let msg = format!("NOT AUTHORIZED: Payload parameters ({}@{}) differ from those in the request header.", 
                                      req.client_id, req.tenant);
            error!("{}", msg);
            return Json(RespUpdateClientSecret::new("1", msg, req.client_id, req.tenant, "".to_string()));
        }

        // -------------------- Process Request ----------------------
        // Process the request.
        let resp = match RespUpdateClientSecret::process(http_req, &req) {
            Ok(r) => r,
            Err(e) => {
                let msg = "ERROR: ".to_owned() + e.to_string().as_str();
                error!("{}", msg);
                RespUpdateClientSecret::new("1", msg, req.client_id, req.tenant, "".to_string())},
        };

        Json(resp)
    }
}

// ***************************************************************************
//                          Request/Response Methods
// ***************************************************************************
impl RespUpdateClientSecret {
    /// Create a new response.
    fn new(result_code: &str, result_msg: String, client_id: String, tenant: String, client_secret: String,) -> Self {
        Self {result_code: result_code.to_string(), result_msg, client_id, tenant, client_secret,}
    }

    /// Process the request.
    fn process(http_req: &Request, req: &ReqUpdateClientSecret) -> Result<RespUpdateClientSecret, anyhow::Error> {
        // Conditional logging depending on log level.
        tms_utils::debug_request(http_req, req);

        // ------------------------ Generate Secret --------------------  
        let client_secret_str  = create_hex_secret();
        let client_secret_hash = hash_hex_secret(&client_secret_str);

        // Insert the new key record.
        block_on(update_client_secret(req, client_secret_hash))?;
        
        // Log result and return response.
        let msg = format!("Secret updated for client {}", req.client_id);
        info!("{}", msg);
        Ok(RespUpdateClientSecret::new("0", msg, req.client_id.clone(), 
                                       req.tenant.clone(), client_secret_str))
    }
}

// ***************************************************************************
//                          Private Functions
// ***************************************************************************
// ---------------------------------------------------------------------------
// update_client_secret:
// ---------------------------------------------------------------------------
async fn update_client_secret(req: &ReqUpdateClientSecret, client_secret_hash: String) -> Result<u64> {
    // Get timestamp.
    let now = timestamp_utc();
    let current_ts = timestamp_utc_to_str(now);

    // Get a connection to the db and start a transaction.  Uncommited transactions 
    // are automatically rolled back when they go out of scope. 
    // See https://docs.rs/sqlx/latest/sqlx/struct.Transaction.html.
    let mut tx = RUNTIME_CTX.db.begin().await?;

    // Update count.
    let mut updates: u64 = 0;

    // Issue the db update call.
    let result = sqlx::query(UPDATE_CLIENT_SECRET)
        .bind(client_secret_hash)
        .bind(current_ts)
        .bind(&req.client_id)
        .bind(&req.tenant)
        .execute(&mut *tx)
        .await?;
    updates += result.rows_affected();

    // Commit the transaction.
    tx.commit().await?;
    Ok(updates)
}
