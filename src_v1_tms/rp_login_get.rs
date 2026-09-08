#![forbid(unsafe_code)]

use poem::Request;
use poem_openapi::{ OpenApi, payload::Json, Object, param::Path, ApiResponse };
use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc};
use sqlx::Row;

use crate::utils::errors::HttpResult;
use crate::utils::authz::{authorize, AuthzTypes};
use crate::utils::db_statements::GET_RP_LOGIN;
use crate::utils::tms_utils::{self, RequestDebug};
use crate::utils::db_types::RPLogin;
use log::error;

use crate::RUNTIME_CTX;

// ***************************************************************************
//                          Request/Response Definiions
// ***************************************************************************
pub struct GetRPLoginApi;

// ***************************************************************************
//                          Request/Response Definiions
// ***************************************************************************
#[derive(Object)]
struct ReqGetRPLogin
{
    tms_identity: String,
    rp_id: String,
    rp_account: String
}

#[derive(Object, Debug)]
pub struct RespGetRPLogin
{
    result_code: String,
    result_msg: String,
    id: i32,
    tms_identity: String,
    rp_id: String,
    rp_account: String,
    expires_at: DateTime<Utc>,
    enabled: bool,
    created: DateTime<Utc>,
    updated: DateTime<Utc>,
}

// Implement the debug record trait for logging.
impl RequestDebug for ReqGetRPLogin {   
    type Req = ReqGetRPLogin;
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
    Http200(Json<RespGetRPLogin>),
    #[oai(status = 400)]
    Http400(Json<HttpResult>),
    #[oai(status = 401)]
    Http401(Json<HttpResult>),
    #[oai(status = 404)]
    Http404(Json<HttpResult>),
    #[oai(status = 500)]
    Http500(Json<HttpResult>),
}

fn make_http_200(resp: RespGetRPLogin) -> TmsResponse {
    TmsResponse::Http200(Json(resp))
}
fn make_http_400(msg: String) -> TmsResponse {
    TmsResponse::Http400(Json(HttpResult::new(400.to_string(), msg)))
}
fn make_http_401(msg: String) -> TmsResponse {
    TmsResponse::Http401(Json(HttpResult::new(401.to_string(), msg)))
}
fn make_http_404(msg: String) -> TmsResponse {
    TmsResponse::Http404(Json(HttpResult::new(404.to_string(), msg)))
}
fn make_http_500(msg: String) -> TmsResponse {
    TmsResponse::Http500(Json(HttpResult::new(500.to_string(), msg)))    
}

// ***************************************************************************
//                             OpenAPI Endpoint
// ***************************************************************************
#[OpenApi]
impl GetRPLoginApi {
    #[oai(path = "/tms/rplogin/:tms_identity/:rp_id/:rp_account", method = "get")]
    async fn get_rp_login_api(&self, http_req: &Request, tms_identity: Path<String>,
                              rp_id: Path<String>, rp_account: Path<String>) -> TmsResponse {
        // Package the request parameters.
        let req = ReqGetRPLogin {
            tms_identity: tms_identity.to_string(), rp_id: rp_id.to_string(),
            rp_account: rp_account.to_string()
        };
        
        // -------------------- Authorize ----------------------------
        // Currently, only the admin can create a user resource provider login record.
        // When user authentication is implemented, we'll add user-own 
        // authorization and any additional validation.
        let allowed = [AuthzTypes::TmsAdmin];
        let authz_result = authorize(http_req, &allowed).await;
        if !authz_result.is_authorized() {
            let msg = format!("ERROR: NOT AUTHORIZED to view resource provider login information for record #{}",
                                      req.tms_identity);
            error!("{}", msg);
            return make_http_401(msg);
        }

        // -------------------- Process Request ----------------------
        // Process the request.
        match RespGetRPLogin::process(http_req, &req).await {
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
impl RespGetRPLogin {
    /// Create a new response.
    #[allow(clippy::too_many_arguments)]
    fn new(result_code: &str, result_msg: String, id: i32, tms_identity: String, rp_id: String,
           rp_account: String, expires_at: DateTime<Utc>, enabled: bool,
           created: DateTime<Utc>, updated: DateTime<Utc>)
    -> Self {
            Self {result_code: result_code.to_string(), result_msg, 
                  id, tms_identity, rp_id, rp_account, expires_at, enabled, created, updated}
        }

    /// Process the request.
    async fn process(http_req: &Request, req: &ReqGetRPLogin) -> Result<TmsResponse, anyhow::Error> {
        // Conditional logging depending on log level.
        tms_utils::debug_request(http_req, req);

        // Search for the client id in the database.  Not found was already
        // The client_secret is never part of the response.
        let db_result = get_rp_login(req).await;
        match db_result {
            Ok(u) => Ok(make_http_200(Self::new("0", "success".to_string(), u.id,
                                                u.tms_identity, u.rp_id, u.rp_account, u.expires_at,
                                                u.enabled, u.created, u.updated))),
            Err(e) => {
                // Determine if this is a real db error or just record not found.
                let msg = e.to_string();
                if msg.contains("NOT_FOUND") {Ok(make_http_404(msg))} 
                  else {Err(e)}
            }
        }
    }
}

// ***************************************************************************
//                          Private Functions
// ***************************************************************************
// ---------------------------------------------------------------------------
// get_rp_login:
// ---------------------------------------------------------------------------
async fn get_rp_login(req: &ReqGetRPLogin) -> Result<RPLogin> {
    // Get a connection to the db and start a transaction.  Uncommited transactions 
    // are automatically rolled back when they go out of scope. 
    // See https://docs.rs/sqlx/latest/sqlx/struct.Transaction.html.
    let mut tx = RUNTIME_CTX.db.begin().await?;
    
    // Create the select statement.
    let result = sqlx::query(GET_RP_LOGIN)
        .bind(&req.tms_identity)
        .bind(&req.rp_id)
        .bind(&req.rp_account)
        .fetch_optional(&mut *tx)
        .await?;

    // Commit the transaction.
    tx.commit().await?;

    // We may have found the user resource provider login record.
    match result {
        Some(row) => {
            Ok(RPLogin::new(row.get(0), row.get(1), row.get(2), row.get(3), 
                            row.get(4), row.get(5), row.get(6), row.get(7),
                            row.get(8)))
        },
        None => {
            Err(anyhow!("NOT_FOUND"))
        },
    }
}
