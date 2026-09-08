#![forbid(unsafe_code)]

use poem::Request;
use poem_openapi::{ OpenApi, payload::Json, Object, param::Path, ApiResponse };
use anyhow::Result;

use crate::utils::errors::HttpResult;
use crate::utils::db_statements::DELETE_RP_LOGIN;
use crate::utils::tms_utils::{self, RequestDebug};
use crate::utils::authz::{authorize, AuthzTypes};
use log::{error, info};

use crate::RUNTIME_CTX;

// ***************************************************************************
//                          Request/Response Definiions
// ***************************************************************************
pub struct DeleteRPLoginApi;

// ***************************************************************************
//                          Request/Response Definiions
// ***************************************************************************
#[derive(Object)]
pub struct ReqDeleteRPLogin
{
    tms_identity: String,
    rp_id: String,
    rp_account: String
}

#[derive(Object, Debug)]
pub struct RespDeleteRPLogin
{
    result_code: String,
    result_msg: String,
    num_deleted: u32,
}

// Implement the debug record trait for logging.
impl RequestDebug for ReqDeleteRPLogin {   
    type Req = ReqDeleteRPLogin;
    fn get_request_info(&self) -> String {
        let mut s = String::with_capacity(255);
        s.push_str("  Request body:");
        s.push_str("\n    tms_identity: ");
        s.push_str(&self.tms_identity);
        s.push_str("\n    rp_id: ");
        s.push_str(&self.rp_id);
        s.push_str("\n    rp_account: ");
        s.push_str(&self.rp_account);
        s
    }
}

// ------------------- HTTP Status Codes -------------------
#[derive(Debug, ApiResponse)]
enum TmsResponse {
    #[oai(status = 200)]
    Http200(Json<RespDeleteRPLogin>),
    #[oai(status = 400)]
    Http400(Json<HttpResult>),
    #[oai(status = 401)]
    Http401(Json<HttpResult>),
    #[oai(status = 500)]
    Http500(Json<HttpResult>),
}

fn make_http_200(resp: RespDeleteRPLogin) -> TmsResponse {
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
impl DeleteRPLoginApi {
    #[oai(path = "/tms/rplogin/del/:tms_identity/:rp_id/:rp_acct", method = "delete")]
    async fn delete_rp_login_api(&self, http_req: &Request, tms_identity: Path<String>, rp_id: Path<String>,
                                 rp_account: Path<String>) -> TmsResponse {
        // Package the request parameters.
        let req = ReqDeleteRPLogin { tms_identity: tms_identity.to_string(),
                                     rp_id: rp_id.to_string(), rp_account: rp_account.to_string() };

        // -------------------- Authorize ----------------------------
        // Currently, only the admin can delete a user rp_login record.
        // When user authentication is implemented, we'll add user-own 
        // authorization and any additional validation.
        let allowed = [AuthzTypes::TmsAdmin];
        let authz_result = authorize(http_req, &allowed).await;
        if !authz_result.is_authorized() {
            let msg = format!("ERROR: NOT AUTHORIZED to delete resource provider login. TmsId: {} RpId: {} RpAcct: {}.",
                                     req.tms_identity, req.rp_id, req.rp_account);
            error!("{}", msg);
            return make_http_401(msg);
        }

        // -------------------- Process Request ----------------------
        // Process the request.
        match RespDeleteRPLogin::process(http_req, &req).await {
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
impl RespDeleteRPLogin {
    /// Create a new response.
    fn new(result_code: &str, result_msg: String, num_deleted: u32) -> Self {
        Self {result_code: result_code.to_string(), result_msg, num_deleted}}

    /// Process the request.
    async fn process(http_req: &Request, req: &ReqDeleteRPLogin) -> Result<TmsResponse, anyhow::Error> {
        // Conditional logging depending on log level.
        tms_utils::debug_request(http_req, req);

        // Insert the new key record.
        let deletes = delete_rp_login(req).await?;
        
        // Log result and return response.
        let msg = 
            if deletes < 1 {format!("RP_LOGIN NOT FOUND - Nothing deleted. TmsId: {} RpId: {} RpAcct: {}",
                                    req.tms_identity, req.rp_id, req.rp_account)}
            else {format!("RP_LOGIN deleted. TmsId: {} RpId: {} RpAcct: {}",
                          req.tms_identity, req.rp_id, req.rp_account)};
        info!("{}", msg);
        Ok(make_http_200(RespDeleteRPLogin::new("0", msg, deletes as u32)))
    }
}

// ***************************************************************************
//                          Private Functions
// ***************************************************************************
// ---------------------------------------------------------------------------
// delete_rp_login:
// ---------------------------------------------------------------------------
async fn delete_rp_login(req: &ReqDeleteRPLogin) -> Result<u64> {
    // Get a connection to the db and start a transaction.  Uncommited transactions 
    // are automatically rolled back when they go out of scope. 
    // See https://docs.rs/sqlx/latest/sqlx/struct.Transaction.html.
    let mut tx = RUNTIME_CTX.db.begin().await?;

    // Deletion count.
    let mut deletes: u64 = 0;

    // Issue the db delete call.
    let result = sqlx::query(DELETE_RP_LOGIN)
        .bind(&req.tms_identity)
        .bind(&req.rp_id)
        .bind(&req.rp_account)
        .execute(&mut *tx)
        .await?;
    deletes += result.rows_affected();

    // Commit the transaction.
    tx.commit().await?;
    Ok(deletes)
}
