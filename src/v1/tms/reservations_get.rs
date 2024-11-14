#![forbid(unsafe_code)]

use poem::Request;
use poem_openapi::{ OpenApi, payload::Json, Object, param::Path, ApiResponse };
use anyhow::{Result, anyhow};
use futures::executor::block_on;
use sqlx::Row;

use crate::utils::errors::HttpResult;
use crate::utils::authz::{authorize, AuthzTypes, get_tenant_header};
use crate::utils::db_statements::GET_RESERVATION;
use crate::utils::tms_utils::{self, RequestDebug, check_tenant_enabled};
use crate::utils::db_types::Reservation;
use log::error;

use crate::RUNTIME_CTX;

// ***************************************************************************
//                          Request/Response Definiions
// ***************************************************************************
pub struct GetReservationApi;

// ***************************************************************************
//                          Request/Response Definiions
// ***************************************************************************
#[derive(Object)]
struct ReqGetReservation
{
    resid: String,
    client_id: String,
    tenant: String,
}

#[derive(Object, Debug)]
pub struct RespGetReservation
{
    result_code: String,
    result_msg: String,
    id: i32,
    resid: String,
    parent_resid: String,
    tenant: String,
    client_id: String,
    client_user_id: String,
    host: String,
    public_key_fingerprint: String, 
    expires_at: String,
    created: String,
    updated: String,
}

// Implement the debug record trait for logging.
impl RequestDebug for ReqGetReservation
{
    type Req = ReqGetReservation;
    fn get_request_info(&self) -> String {
        let mut s = String::with_capacity(255);
        s.push_str("  Request body:");
        s.push_str("\n    resid: ");
        s.push_str(&self.resid);
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
    Http200(Json<RespGetReservation>),
    #[oai(status = 400)]
    Http400(Json<HttpResult>),
    #[oai(status = 401)]
    Http401(Json<HttpResult>),
    #[oai(status = 403)]
    Http403(Json<HttpResult>),
    #[oai(status = 404)]
    Http404(Json<HttpResult>),
    #[oai(status = 500)]
    Http500(Json<HttpResult>),
}

fn make_http_200(resp: RespGetReservation) -> TmsResponse {
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
impl GetReservationApi {
    #[oai(path = "/tms/reservations/:client_id/:resid", method = "get")]
    async fn get_reservation_api(&self, http_req: &Request, client_id: Path<String>, resid: Path<String>) -> TmsResponse {
        // -------------------- Get Tenant Header --------------------
        // Get the required tenant header value.
        let hdr_tenant = match get_tenant_header(http_req) {
            Ok(t) => t,
            Err(e) => return make_http_400(e.to_string()),
        };
        
        // Check tenant.
        if !check_tenant_enabled(&hdr_tenant) {
            return make_http_400("Tenant not enabled.".to_string());
        }

        // Package the request parameters.        
        let req = 
            ReqGetReservation {resid: resid.to_string(), client_id: client_id.to_string(), tenant: hdr_tenant};
        
        // -------------------- Authorize ----------------------------
        // Only the client and tenant admin can query a reservation record.
        let allowed = [AuthzTypes::ClientOwn, AuthzTypes::TenantAdmin];
        let authz_result = authorize(http_req, &allowed);
        if !authz_result.is_authorized() {
            let msg = format!("ERROR: NOT AUTHORIZED to view reservation {} in tenant {}.", req.resid, req.tenant);
            error!("{}", msg);
            return make_http_401(msg);
        }

        // Make sure the path parms conform to the header values used for authorization.
        // The req.tenant is assigned the hdr_tenant value, so there's no point in checking
        // for matching tenant values.  
        if !authz_result.check_hdr_id(&req.client_id) {
            let msg = format!("ERROR: FORBIDDEN - Path client_id parameter ({}) differs from the client_id in the request header.", 
                                      req.client_id);
            error!("{}", msg);
            return make_http_403(msg);        
        }

        // -------------------- Process Request ----------------------
        // Process the request.
        match RespGetReservation::process(http_req, &req) {
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
impl RespGetReservation {
    // Create a new response.
    #[allow(clippy::too_many_arguments)]
    fn new(result_code: &str, result_msg: String, id: i32, resid: String, parent_resid: String, 
            tenant: String, client_id: String, client_user_id: String, host: String, 
            public_key_fingerprint: String, expires_at: String, created: String, updated: String) 
    -> Self {
            Self {result_code: result_code.to_string(), result_msg, 
              id, resid, parent_resid, tenant, client_id, client_user_id, host, public_key_fingerprint, 
              expires_at, created, updated}
        }

    // Process the request.
    fn process(http_req: &Request, req: &ReqGetReservation) -> Result<TmsResponse, anyhow::Error> {
        // Conditional logging depending on log level.
        tms_utils::debug_request(http_req, req);

        // Search for the tenant/resid in the database.  Not found was already 
        let db_result = block_on(get_reservation(req));
        match db_result {
            Ok(res) => Ok(make_http_200(Self::new("0", "success".to_string(), 
                                    res.id, res.resid, res.parent_resid, res.tenant, res.client_id, 
                                    res.client_user_id, res.host, res.public_key_fingerprint, 
                                    res.expires_at, res.created, res.updated))),
            Err(e) => {
                // Determine if this is a real db error or just record not found.
                let msg = e.to_string();
                if msg.contains("NOT_FOUND") {Ok(make_http_404(msg))} 
                  else {Err(e)}
            },
        }
    }
}

// ***************************************************************************
//                          Private Functions
// ***************************************************************************
// ---------------------------------------------------------------------------
// get_reservation:
// ---------------------------------------------------------------------------
async fn get_reservation(req: &ReqGetReservation) -> Result<Reservation> {
    // Get a connection to the db and start a transaction.  Uncommited transactions 
    // are automatically rolled back when they go out of scope. 
    // See https://docs.rs/sqlx/latest/sqlx/struct.Transaction.html.
    let mut tx = RUNTIME_CTX.db.begin().await?;
    
    // Create the select statement.
    let result = sqlx::query(GET_RESERVATION)
        .bind(req.resid.clone())
        .bind(req.tenant.clone())
        .fetch_optional(&mut *tx)
        .await?;

    // Commit the transaction.
    tx.commit().await?;

    // We may have found the reservation. 
    match result {
        Some(row) => {
            Ok(Reservation::new(row.get(0), row.get(1), row.get(2), 
                row.get(3), row.get(4), row.get(5), row.get(6), 
                row.get(7), row.get(8), row.get(9), row.get(10)))
        },
        None => {
            Err(anyhow!("NOT_FOUND"))
        },
    }
}
