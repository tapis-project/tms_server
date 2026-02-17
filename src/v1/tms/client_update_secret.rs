#![forbid(unsafe_code)]

use poem::Request;
use poem_openapi::{ OpenApi, payload::Json, Object, param::Path, ApiResponse };
use anyhow::Result;

use crate::utils::errors::HttpResult;
use crate::utils::db_statements::UPDATE_CLIENT_SECRET;
use crate::utils::tms_utils::{self, RequestDebug, create_hex_secret, hash_hex_secret, 
                              timestamp_utc, timestamp_utc_to_str, check_tenant_enabled};
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

#[derive(Object, Debug)]
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

// ------------------- HTTP Status Codes -------------------
#[derive(Debug, ApiResponse)]
enum TmsResponse {
    #[oai(status = 200)]
    Http200(Json<RespUpdateClientSecret>),
    #[oai(status = 400)]
    Http400(Json<HttpResult>),
    #[oai(status = 401)]
    Http401(Json<HttpResult>),
    #[oai(status = 403)]
    Http403(Json<HttpResult>),
    #[oai(status = 500)]
    Http500(Json<HttpResult>),
}

fn make_http_200(resp: RespUpdateClientSecret) -> TmsResponse {
    TmsResponse::Http200(Json(resp))
}
fn make_http_400(msg: String) -> TmsResponse {
    TmsResponse::Http400(Json(HttpResult::new(400.to_string(), msg)))
}
fn make_http_401(msg: String) -> TmsResponse {
    TmsResponse::Http401(Json(HttpResult::new(401.to_string(), msg)))
}
fn make_http_403(msg: String) -> TmsResponse {
    TmsResponse::Http403(Json(HttpResult::new(403.to_string(), msg)))
}
fn make_http_500(msg: String) -> TmsResponse {
    TmsResponse::Http500(Json(HttpResult::new(500.to_string(), msg)))    
}

// ***************************************************************************
//                             OpenAPI Endpoint
// ***************************************************************************
#[OpenApi]
impl UpdateClientSecretApi {
    #[oai(path = "/tms/client/secret/:client_id", method = "patch")]
    async fn update_client(&self, http_req: &Request, client_id: Path<String>) -> TmsResponse {
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
        let req = ReqUpdateClientSecret {client_id: client_id.to_string(), tenant: hdr_tenant};

        // -------------------- Authorize ----------------------------
        // Only the client and tenant admin can query a client record.
        let allowed = [AuthzTypes::ClientOwn, AuthzTypes::TenantAdmin];
        let authz_result = authorize(http_req, &allowed).await;
        if !authz_result.is_authorized() {
            let msg = format!("ERROR: NOT AUTHORIZED to update client {} in tenant {}.", req.client_id, req.tenant);
            error!("{}", msg);
            return make_http_401(msg);
        }

        // Make sure the request parms conform to the header values used for authorization.
        if !authz_result.check_hdr_id(&req.client_id) {
            let msg = format!("ERROR: FORBIDDEN - Payload parameters ({}@{}) differ from those in the request header.", 
                                      req.client_id, req.tenant);
            error!("{}", msg);
            return make_http_403(msg);
        }

        // -------------------- Process Request ----------------------
        // Process the request.
        match RespUpdateClientSecret::process(http_req, &req).await {
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
impl RespUpdateClientSecret {
    /// Create a new response.
    fn new(result_code: &str, result_msg: String, client_id: String, tenant: String, client_secret: String,) -> Self {
        Self {result_code: result_code.to_string(), result_msg, client_id, tenant, client_secret,}
    }

    /// Process the request.
    async fn process(http_req: &Request, req: &ReqUpdateClientSecret) -> Result<TmsResponse, anyhow::Error> {
        // Conditional logging depending on log level.
        tms_utils::debug_request(http_req, req);

        // ------------------------ Generate Secret --------------------  
        let client_secret_str  = create_hex_secret();
        let client_secret_hash = hash_hex_secret(&client_secret_str);

        // Insert the new key record.
        update_client_secret(req, client_secret_hash).await?;
        
        // Log result and return response.
        let msg = format!("Secret updated for client {}", req.client_id);
        info!("{}", msg);
        Ok(make_http_200(RespUpdateClientSecret::new("0", msg, req.client_id.clone(), 
                                       req.tenant.clone(), client_secret_str)))
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
