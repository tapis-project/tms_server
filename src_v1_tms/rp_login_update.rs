#![forbid(unsafe_code)]

use poem::Request;
use poem_openapi::{ OpenApi, payload::Json, Object, ApiResponse };
use anyhow::Result;

use crate::utils::errors::HttpResult;
use crate::utils::db_statements::UPDATE_RP_LOGIN_ENABLED;
use crate::utils::tms_utils::{self, RequestDebug, timestamp_utc, timestamp_utc_to_str};
use crate::utils::authz::{authorize, AuthzTypes};
use log::{error, info};

use crate::RUNTIME_CTX;

// ***************************************************************************
//                          Request/Response Definitions
// ***************************************************************************
pub struct UpdateRPLoginApi;

// ***************************************************************************
//                          Request/Response Definitions
// ***************************************************************************
#[derive(Object)]
pub struct ReqUpdateRPLogin
{
    tms_identity: String,
    rp_id: String,
    rp_account: String,
    enabled: bool,
    // TODO need to add last_login? expires_at?
}

#[derive(Object, Debug)]
pub struct RespUpdateRPLogin
{
    result_code: String,
    result_msg: String,
    fields_updated: i32,
}

// Implement the debug record trait for logging.
impl RequestDebug for ReqUpdateRPLogin {
    type Req = ReqUpdateRPLogin;
    fn get_request_info(&self) -> String {
        // Get optional values in displayable form. 
        let enabled = format!("{:#?}", &self.enabled);

        let mut s = String::with_capacity(255);
        s.push_str("  Request body:");
        s.push_str("\n    tms_identity: ");
        s.push_str(&self.tms_identity);
        s.push_str("\n    rp_id: ");
        s.push_str(&self.rp_id);
        s.push_str("\n    rp_acount: ");
        s.push_str(&self.rp_account);
        s.push_str("\n    enabled: ");
        s.push_str(enabled.as_str());
        s
    }
}

// ------------------- HTTP Status Codes -------------------
#[derive(Debug, ApiResponse)]
enum TmsResponse {
    #[oai(status = 200)]
    Http200(Json<RespUpdateRPLogin>),
    #[oai(status = 400)]
    Http400(Json<HttpResult>),
    #[oai(status = 401)]
    Http401(Json<HttpResult>),
    #[oai(status = 403)]
    Http403(Json<HttpResult>),
    #[oai(status = 500)]
    Http500(Json<HttpResult>),
}

fn make_http_200(resp: RespUpdateRPLogin) -> TmsResponse {
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
impl UpdateRPLoginApi {
    #[oai(path = "/tms/rplogin/upd", method = "patch")]
async fn update_rp_login(&self, http_req: &Request, req: Json<ReqUpdateRPLogin>) -> TmsResponse {
        // -------------------- Authorize ----------------------------
        // Currently, only the admin can create a user rp_login record.
        // When user authentication is implemented, we'll add user-own 
        // authorization and any additional validation.
        let allowed = [AuthzTypes::TmsAdmin];
        let authz_result = authorize(http_req, &allowed).await;
        if !authz_result.is_authorized() {
            let msg = format!("ERROR: NOT AUTHORIZED to update resource provider login. TmsId: {} RpId: {} RpAcct: {}",
                                     req.tms_identity, req.rp_id, req.rp_account);
            error!("{}", msg);
            return make_http_401(msg);
        }

        // -------------------- Process Request ----------------------
        // Process the request.
        match RespUpdateRPLogin::process(http_req, &req).await {
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
impl RespUpdateRPLogin {
    /// Create a new response.
    fn new(result_code: &str, result_msg: String, num_updates: i32,) -> Self {
        Self {result_code: result_code.to_string(), result_msg, fields_updated: num_updates}}

    /// Process the request.
    async fn process(http_req: &Request, req: &ReqUpdateRPLogin) -> Result<TmsResponse, anyhow::Error> {
        // Conditional logging depending on log level.
        tms_utils::debug_request(http_req, req);

        // Insert the new key record.
        let updates = update_rp_login(req).await?;
        
        // Log result and return response.
        let msg = format!("{} update(s) completed. TmsId: {} RpId: {} RpAcct: {} Enabled: {}",
                                 updates, req.tms_identity, req.rp_id, req.rp_account, req.enabled);
        info!("{}", msg);
        Ok(make_http_200(RespUpdateRPLogin::new("0", msg, updates as i32)))
    }
}

// ***************************************************************************
//                          Private Functions
// ***************************************************************************
// ---------------------------------------------------------------------------
// update_rp_login:
// ---------------------------------------------------------------------------
async fn update_rp_login(req: &ReqUpdateRPLogin) -> Result<u64> {
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
    let result = sqlx::query(UPDATE_RP_LOGIN_ENABLED)
        .bind(req.enabled)
        .bind(current_ts)
        .bind(&req.tms_identity)
        .bind(&req.rp_id)
        .bind(&req.rp_account)
        .execute(&mut *tx)
        .await?;
    updates += result.rows_affected();

    // Commit the transaction.
    tx.commit().await?;
    Ok(updates)
}
