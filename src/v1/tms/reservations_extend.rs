#![forbid(unsafe_code)]

use poem::Request;
use poem_openapi::{ OpenApi, payload::Json, Object, ApiResponse };
use anyhow::Result;
use uuid::Uuid;

use futures::executor::block_on;

use crate::utils::authz::{authorize, AuthzTypes, get_tenant_header, get_client_id_header};
use crate::utils::errors::HttpResult;
use crate::utils::db_types::ReservationInput;
use crate::utils::db_statements::INSERT_RESERVATIONS;
use crate::utils::tms_utils::{self, timestamp_utc, timestamp_utc_to_str, RequestDebug, check_tenant_enabled};
use crate::utils::db::check_parent_reservation;
use log::{error, info};

use crate::RUNTIME_CTX;

// ***************************************************************************
//                          Request/Response Definiions
// ***************************************************************************
/** Extend an existing, active reservation.  The only required dependency checking
 * involved the state of the existing reservation--the underlying public key and
 * its dependencies do not have to be checked. 
 */
pub struct ExtendReservationsApi;

#[derive(Object)]
pub struct ReqExtendReservation
{
    client_user_id: String,
    host: String,
    public_key_fingerprint: String,
    parent_resid: String,
}

#[derive(Object, Debug)]
struct RespExtendReservation
{
    result_code: String,
    result_msg: String,
    resid: String,
    parent_resid: String,
    expires_at: String,
}

// Implement the debug record trait for logging.
impl RequestDebug for ReqExtendReservation {   
    type Req = ReqExtendReservation;
    fn get_request_info(&self) -> String {
        let mut s = String::with_capacity(255);
        s.push_str("  Request body:");
        s.push_str("\n    client_user_id: ");
        s.push_str(&self.client_user_id);
        s.push_str("\n    host: ");
        s.push_str(&self.host);
        s.push_str("\n    public_key_fingerprint: ");
        s.push_str(&self.public_key_fingerprint);
        s.push_str("\n    parent_resid: ");
        s.push_str(&self.parent_resid);
        s.push('\n');
        s
    }
}

// Extracted header values to complete request input
#[derive(Debug)]
struct ReqReservationExtension
{
    client_id: String,
    tenant: String,
}

impl ReqReservationExtension {
    fn new(client_id: String, tenant: String,) -> Self 
    { Self {client_id, tenant} }  
}

// ------------------- HTTP Status Codes -------------------
#[derive(Debug, ApiResponse)]
enum TmsResponse {
    #[oai(status = 201)]
    Http201(Json<RespExtendReservation>),
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

fn make_http_201(resp: RespExtendReservation) -> TmsResponse {
    TmsResponse::Http201(Json(resp))
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
impl ExtendReservationsApi {
    #[oai(path = "/tms/reservations/extend", method = "post")]
    async fn extend_reservation_api(&self, http_req: &Request, req: Json<ReqExtendReservation>) -> TmsResponse {
        match RespExtendReservation::process(http_req, &req) {
            Ok(r) => r,
            Err(e) => {
                // Assume a server fault if a raw error came through.
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
impl RespExtendReservation {
    #[allow(clippy::too_many_arguments)]
    fn new(result_code: &str, result_msg: &str, resid: String, parent_resid: String, expires_at: String) -> Self {
        Self {result_code: result_code.to_string(), result_msg: result_msg.to_string(), 
              resid, parent_resid, expires_at,
        }
    }

    fn process(http_req: &Request, req: &ReqExtendReservation) -> Result<TmsResponse, anyhow::Error> {
        // Conditional logging depending on log level.
        tms_utils::debug_request(http_req, req);

        // -------------------- Extract Headers ----------------------
        // Get the headers used in this function.
        let req_ext = match get_header_values(http_req) {
            Ok(h) => h,
            Err(e) => {
                return Ok(make_http_400(e.to_string()));
            }
        };

        // Check tenant.
        if tokio::runtime::Handle::current().block_on(check_tenant_enabled(&req_ext.tenant)) {
            return Ok(make_http_400("Tenant not enabled.".to_string()));
        }

        // -------------------- Authorize ----------------------------
        // Only the client and tenant admin can query a client record.
        let allowed = [AuthzTypes::ClientOwn];
        let authz_result = authorize(http_req, &allowed);
        if !authz_result.is_authorized() {
            let msg = format!("ERROR: NOT AUTHORIZED Credential mismatch for client {} in tenant {}.", 
                                      req_ext.client_id, req_ext.tenant);
            error!("{}", msg);
            return Ok(make_http_401(msg));
        }

        // ------------------ Get Parent Reservation -------------------
        // Check that the designated parent reservation can be extended
        // and retrieve that reservation's experation time.
        let expires_at = match block_on(check_parent_reservation(&req.parent_resid, &req_ext.tenant, 
                        &req_ext.client_id, &req.client_user_id, &req.host, &req.public_key_fingerprint)) 
        {
            Ok(expiry) => expiry,
            Err(e) => {
                let msg = format!("Missing or expired dependency: {}", e);
                error!("{}", msg);
                if msg.contains("NOT_FOUND:") {return Ok(make_http_404(msg));}
                else if msg.contains("INTERNAL ERROR:") {return Ok(make_http_500(msg));}
                else {return Ok(make_http_403(msg));}
            } 
        };

        // ------------------------ Update Database --------------------
        // Assign a new uuid to the reservation id.
        let resid = Uuid::new_v4().as_hyphenated().to_string();

        // Use the same current UTC timestamp in all related time caculations.
        // We also use the original requested ttl_minutes to calculate expires_at
        // so that we get a uniform maximum uniform datetime rather then one that
        // changes with current time when req.ttl_minutes = -1.
        let now  = timestamp_utc();
        let current_ts  = timestamp_utc_to_str(now);

        // Create the input record.
        let input_record: ReservationInput = ReservationInput::new(
            resid.clone(),
            req.parent_resid.clone(),
            req_ext.tenant.clone(),
            req_ext.client_id.clone(),
            req.client_user_id.clone(), 
            req.host.clone(), 
            req.public_key_fingerprint.clone(), 
            expires_at.clone(), 
            current_ts.clone(), 
            current_ts,
        );

        // Insert the new key record.
        block_on(extend_reservation(input_record))?;
        info!("Reservation '{}' created for '{}@{}' for host '{}' expires at {}.", 
              resid, req.client_user_id, req_ext.tenant, req.host, expires_at);

        // Success! 
        Ok(make_http_201(Self::new("0", "success", 
                         resid, req.parent_resid.clone(), expires_at,)))
    }
}

// ***************************************************************************
//                          Private Functions
// ***************************************************************************
// ---------------------------------------------------------------------------
// extend_reservation:
// ---------------------------------------------------------------------------
async fn extend_reservation(rec: ReservationInput) -> Result<u64> {
    // Get a connection to the db and start a transaction.  Uncommited transactions 
    // are automatically rolled back when they go out of scope. 
    // See https://docs.rs/sqlx/latest/sqlx/struct.Transaction.html.
    let mut tx = RUNTIME_CTX.db.begin().await?;
    
    // Create the insert statement.
    let result = sqlx::query(INSERT_RESERVATIONS)
        .bind(rec.resid)
        .bind(rec.parent_resid)
        .bind(rec.tenant)
        .bind(rec.client_id)
        .bind(rec.client_user_id)
        .bind(rec.host)
        .bind(rec.public_key_fingerprint)
        .bind(rec.expires_at)
        .bind(rec.created)
        .bind(rec.updated)
        .execute(&mut *tx)
        .await?;

    // Commit the transaction.
    tx.commit().await?;

    Ok(result.rows_affected())
}

// ---------------------------------------------------------------------------
// get_header_values:
// ---------------------------------------------------------------------------
fn get_header_values(http_req: &Request) -> Result<ReqReservationExtension> {
    // Get the required header values.
    let hdr_client_id = get_client_id_header(http_req)?;
    let hdr_tenant = get_tenant_header(http_req)?;

    Ok(ReqReservationExtension::new(hdr_client_id, hdr_tenant))
}
