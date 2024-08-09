#![forbid(unsafe_code)]

use poem::Request;
use poem_openapi::{ OpenApi, payload::Json, Object, ApiResponse };
use anyhow::Result;
use futures::executor::block_on;

use crate::utils::errors::HttpResult;
use crate::utils::db_statements::DELETE_PUBKEY;
use crate::utils::tms_utils::{self, RequestDebug};
use crate::utils::authz::{authorize, AuthzTypes};
use log::{error, info};

use crate::RUNTIME_CTX;

// ***************************************************************************
//                          Request/Response Definiions
// ***************************************************************************
pub struct DeletePubkeysApi;

// ***************************************************************************
//                          Request/Response Definiions
// ***************************************************************************
#[derive(Object)]
pub struct ReqDeletePubkey
{
    client_id: String,
    tenant: String,
    host: String,
    public_key_fingerprint: String,
}

#[derive(Object, Debug)]
pub struct RespDeletePubkey
{
    result_code: String,
    result_msg: String,
    num_deleted: u32,
}

// Implement the debug record trait for logging.
impl RequestDebug for ReqDeletePubkey {   
    type Req = ReqDeletePubkey;
    fn get_request_info(&self) -> String {
        let mut s = String::with_capacity(255);
        s.push_str("  Request body:");
        s.push_str("\n    client_id: ");
        s.push_str(&self.client_id);
        s.push_str("\n    tenant: ");
        s.push_str(&self.tenant);
        s.push_str("\n    host: ");
        s.push_str(&self.host);
        s.push_str("\n    public_key_fingerprint: ");
        s.push_str(&self.public_key_fingerprint);
        s
    }
}

// ------------------- HTTP Status Codes -------------------
#[derive(Debug, ApiResponse)]
enum TmsResponse {
    #[oai(status = 200)]
    Http200(Json<RespDeletePubkey>),
    #[oai(status = 401)]
    Http401(Json<HttpResult>),
    #[oai(status = 403)]
    Http403(Json<HttpResult>),
    #[oai(status = 500)]
    Http500(Json<HttpResult>),
}

fn make_http_200(resp: RespDeletePubkey) -> TmsResponse {
    TmsResponse::Http200(Json(resp))
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
impl DeletePubkeysApi {
    #[oai(path = "/tms/pubkeys", method = "delete")]
    async fn delete_client(&self, http_req: &Request, req: Json<ReqDeletePubkey>) -> TmsResponse {
        // -------------------- Authorize ----------------------------
        // Only the client and tenant admin can access a client record.
        let allowed = [AuthzTypes::ClientOwn, AuthzTypes::TenantAdmin];
        let authz_result = authorize(http_req, &allowed);
        if !authz_result.is_authorized() {
            let msg = format!("ERROR: NOT AUTHORIZED to delete public key {} in tenant {}.", req.client_id, req.tenant);
            error!("{}", msg);
            return make_http_401(msg);
        }

        // Make sure the request parms conform to the header values used for authorization.
        if !authz_result.check_hdr_id(&req.client_id) || !authz_result.check_hdr_tenant(&req.tenant) {
            let msg = format!("ERROR: NOT AUTHORIZED - Payload parameters ({}@{}) differ from those in the request header.", 
                                      req.client_id, req.tenant);
            error!("{}", msg);
            return make_http_403(msg);
        }

        // -------------------- Process Request ----------------------
        // Process the request.
        match RespDeletePubkey::process(http_req, &req) {
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
impl RespDeletePubkey {
    /// Create a new response.
    fn new(result_code: &str, result_msg: String, num_deleted: u32) -> Self {
        Self {result_code: result_code.to_string(), result_msg, num_deleted}}

    /// Process the request.
    fn process(http_req: &Request, req: &ReqDeletePubkey) -> Result<TmsResponse, anyhow::Error> {
        // Conditional logging depending on log level.
        tms_utils::debug_request(http_req, req);

        // Insert the new key record.
        let deletes = block_on(delete_pubkey(req))?;
        
        // Log result and return response.
        let msg = 
            if deletes < 1 {format!("Pubkey {} NOT FOUND for host {} and client {} - Nothing deleted", 
                                    req.public_key_fingerprint, req.host, req.client_id)}
            else {format!("Pubkey {} for host {} deleted", req.public_key_fingerprint, req.host)};
        info!("{}", msg);
        Ok(make_http_200(RespDeletePubkey::new("0", msg, deletes as u32)))
    }
}

// ***************************************************************************
//                          Private Functions
// ***************************************************************************
// ---------------------------------------------------------------------------
// delete_pubkey:
// ---------------------------------------------------------------------------
async fn delete_pubkey(req: &ReqDeletePubkey) -> Result<u64> {
    // Get a connection to the db and start a transaction.  Uncommited transactions 
    // are automatically rolled back when they go out of scope. 
    // See https://docs.rs/sqlx/latest/sqlx/struct.Transaction.html.
    let mut tx = RUNTIME_CTX.db.begin().await?;

    // Deletion count.
    let mut deletes: u64 = 0;

    // Issue the db delete call.
    let result = sqlx::query(DELETE_PUBKEY)
        .bind(&req.client_id)
        .bind(&req.tenant)
        .bind(&req.host)
        .bind(&req.public_key_fingerprint)
        .execute(&mut *tx)
        .await?;
    deletes += result.rows_affected();

    // Commit the transaction.
    tx.commit().await?;
    Ok(deletes)
}
