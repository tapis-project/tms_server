#![forbid(unsafe_code)]

use poem::Request;
use poem_openapi::{ OpenApi, payload::Json, Object, ApiResponse };
use anyhow::{Result, anyhow};
use sqlx::Row;
use std::cmp::min;
use uuid::Uuid;

use futures::executor::block_on;

use crate::utils::authz::{authorize, AuthzTypes, get_tenant_header, get_client_id_header};
use crate::utils::errors::HttpResult;
use crate::utils::db_types::ReservationInput;
use crate::utils::db_statements::{INSERT_RESERVATIONS, SELECT_PUBKEY_RESERVATION_INFO};
use crate::utils::db::check_pubkey_dependencies;
use crate::utils::tms_utils::{self, timestamp_utc, timestamp_utc_to_str, calc_expires_at, timestamp_str_to_datetime, RequestDebug};
use log::{error, info};

use crate::RUNTIME_CTX;

// 48 hour maximum reservation (2880 minutes).
const MAX_RESERVATION_MINUTES: i32 = 48 * 60;

// ***************************************************************************
//                          Request/Response Definiions
// ***************************************************************************
/** Create a new reservation on an active (non-expired) pubkey record.  All 
 * dependency checking required for pubkey creation is also performed here.  
 * Specifically, the user mfa, user/host mapping and client delegation records 
 * must be in order and active before a reservation can be created.
 */
pub struct CreateReservationsApi;

#[derive(Object)]
pub struct ReqCreateReservation
{
    client_user_id: String,
    host: String,
    public_key_fingerprint: String,
    ttl_minutes: i32,  // negative means i32::MAX
}

#[derive(Object, Debug)]
struct RespCreateReservation
{
    result_code: String,
    result_msg: String,
    resid: String,
    parent_resid: String,
    expires_at: String,
}

// Implement the debug record trait for logging.
impl RequestDebug for ReqCreateReservation {   
    type Req = ReqCreateReservation;
    fn get_request_info(&self) -> String {
        let mut s = String::with_capacity(255);
        s.push_str("  Request body:");
        s.push_str("\n    client_user_id: ");
        s.push_str(&self.client_user_id);
        s.push_str("\n    host: ");
        s.push_str(&self.host);
        s.push_str("\n    public_key_fingerprint: ");
        s.push_str(&self.public_key_fingerprint);
        s.push_str("\n    ttl_minutes: ");
        s.push_str(&self.ttl_minutes.to_string());
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

// Public key fields used in processing a new reservation.
struct PubkeyInfo {
    remaining_uses: i32,
    expires_at: String,
    host_account: String,
}

// ------------------- HTTP Status Codes -------------------
#[derive(Debug, ApiResponse)]
enum TmsResponse {
    #[oai(status = 201)]
    Http201(Json<RespCreateReservation>),
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

fn make_http_201(resp: RespCreateReservation) -> TmsResponse {
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
impl CreateReservationsApi {
    #[oai(path = "/tms/reservations", method = "post")]
    async fn create_reservation_api(&self, http_req: &Request, req: Json<ReqCreateReservation>) -> TmsResponse {
        match RespCreateReservation::process(http_req, &req) {
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
impl RespCreateReservation {
    #[allow(clippy::too_many_arguments)]
    fn new(result_code: &str, result_msg: &str, resid: String, parent_resid: String, expires_at: String) -> Self {
        Self {result_code: result_code.to_string(), result_msg: result_msg.to_string(), 
              resid, parent_resid, expires_at,
        }
    }

    fn process(http_req: &Request, req: &ReqCreateReservation) -> Result<TmsResponse, anyhow::Error> {
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

        // --------------------- Check Pubkey Info -----------------------
        // Get the remaining uses and expires_at time of the public key.
        let pubkey_info = 
            match block_on(get_pubkey_status(&req_ext.client_id, &req_ext.tenant, &req.host, &req.public_key_fingerprint)) {
                Ok(info) => info,
                Err(e) => {
                    if e.to_string().contains("NOT_FOUND") {
                        let msg = format!("NOT FOUND: No pubkey for client {}@{} on host {} with fingerprint {}.", 
                                    req_ext.client_id, req_ext.tenant, req.host, req.public_key_fingerprint);
                        error!("{}", msg);
                        return Ok(make_http_404(msg));
                    } else {
                        let msg = format!("ERROR: Unable to access pubkey for client {}@{} on host {} with fingerprint {}.", 
                                    req_ext.client_id, req_ext.tenant, req.host, req.public_key_fingerprint);
                        error!("{}", msg);
                        return Ok(make_http_500(msg));
                    }
                }
            };

        // Determine if the pubkey is active.
        if pubkey_info.remaining_uses < 1 {
            let msg = format!("Pubkey for client {}@{} on host {} with fingerprint {} has no remaining uses.",
                                        req_ext.client_id, req_ext.tenant, req.host, req.public_key_fingerprint);
            error!("{}", msg);
            return Ok(make_http_403(msg));
        }

        // Parse the user host mapping's expires_at timestamp.
        let expires_at_utc= match timestamp_str_to_datetime(&pubkey_info.expires_at) {
            Ok(utc) => utc,
            Err(e) => {
                // This should not happen since we are the only ones that write the database.
                let msg = format!("INTERNAL ERROR: Unable to parse pubkeys expires_at value '{}' \
                                           for client {}@{} on host {} with fingerprint {}: {}", 
                                           pubkey_info.expires_at, req_ext.client_id, req_ext.tenant, 
                                           req.host, req.public_key_fingerprint, e);
                error!("{}", msg);
                return Ok(make_http_500(msg));
            }
        };
    
        // Check whether the user host mapping has expired.
        if expires_at_utc < timestamp_utc() {
            let msg = format!("Pubkey for client {}@{} on host {} with fingerprint {} expired at {}.",
                                        req_ext.client_id, req_ext.tenant, req.host, 
                                        req.public_key_fingerprint, pubkey_info.expires_at);
            error!("{}", msg);
            return Ok(make_http_403(msg));
        }

        // --------------------- Check Expirations -----------------------
        // The 3 tables whose expiration times need to be checked before we create this key are:
        //
        //  user_mfa - use tenant and client_user_id to target unique record
        //  delegations - use tenant, client_id and client_user_id to target unique record
        //  user_hosts - use tenant, client_user_id, host and host_account to target unique record
        //
        // Each of the above tables is queried using values that define a unique index on that
        // target table.  This guarantees that either 0 or 1 records will be returned.  In the 
        // former case, the pubkey key cannot be created because one of its foriegn keys doesn't
        // exist.  In the latter case, we have to check that the retrieved record has not 
        // expired.
        //
        // This method returns an detailed error message that indicates which table did not
        // contain the required values and whether the error resulted from a missing or 
        // expired record.  
        match block_on(check_pubkey_dependencies(&req_ext.tenant, &req_ext.client_id, 
                                        &req.client_user_id, &req.host, &pubkey_info.host_account))
        {
            Ok(_) => (),
            Err(e) => {
                let msg = format!("Missing or expired dependency: {}", e);
                error!("{}", msg);
                if msg.contains("INTERNAL ERROR:") {return Ok(make_http_500(msg));}
                    else {return Ok(make_http_403(msg));}
            } 
        }

        // ------------------------ Update Database --------------------
        // Assign a uuid to the reservation id.
        let resid = Uuid::new_v4().as_hyphenated().to_string();

        // Interpret numeric input.
        let ttl_minutes = if req.ttl_minutes < 0 {MAX_RESERVATION_MINUTES} 
                                else {min(req.ttl_minutes, MAX_RESERVATION_MINUTES)};

        // Use the same current UTC timestamp in all related time caculations.
        // We also use the original requested ttl_minutes to calculate expires_at
        // so that we get a uniform maximum uniform datetime rather then one that
        // changes with current time when req.ttl_minutes = -1.
        let now  = timestamp_utc();
        let current_ts  = timestamp_utc_to_str(now);
        let expires_at  = calc_expires_at(now, ttl_minutes); 

        // Create the input record.
        let input_record: ReservationInput = ReservationInput::new(
            resid.clone(),
            resid.clone(),
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
        block_on(create_reservation(input_record))?;
        info!("Reservation '{}' created for '{}@{}' for host '{}' expires at {}.", 
              resid, req.client_user_id, req_ext.tenant, req.host, expires_at);

        // Success! 
        Ok(make_http_201(Self::new("0", "success", 
                         resid.clone(), resid, expires_at,)))
    }
}

// ***************************************************************************
//                          Private Functions
// ***************************************************************************
// ---------------------------------------------------------------------------
// create_reservation:
// ---------------------------------------------------------------------------
async fn create_reservation(rec: ReservationInput) -> Result<u64> {
    // Get a connection to the db and start a transaction.  Uncommited transactions 
    // are automatically rolled back when they go out of scope. 
    // See https://docs.rs/sqlx/latest/sqlx/struct.Transaction.html.
    let mut tx = RUNTIME_CTX.db.begin().await?;
    
    // Create the insert statement.  On new reservations, the resid and parent_resid
    // have the same value.  Only on reservation created by extending an existing
    // reservation are the resid and parent_resid different.
    let result = sqlx::query(INSERT_RESERVATIONS)
        .bind(&rec.resid)
        .bind(&rec.resid)
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
// get_pubkey_status:
// ---------------------------------------------------------------------------
async fn get_pubkey_status(client_id: &String, tenant: &String, host: &String, 
                                public_key_fingerprint: &String) -> Result<PubkeyInfo> {
    // DB connection and transaction start.
    let mut tx = RUNTIME_CTX.db.begin().await?;

    // Create the select statement.
    let result = sqlx::query(SELECT_PUBKEY_RESERVATION_INFO)
        .bind(client_id)
        .bind(tenant)
        .bind(host)
        .bind(public_key_fingerprint)
        .fetch_optional(&mut *tx)
        .await?;

    // Commit the transaction.
    tx.commit().await?;

    // Parse the result.
    match result {
        Some(row) => {
            // Get the remaining uses and expires_at values from the pubkey.
            let info = PubkeyInfo {remaining_uses: row.get(0), 
                                               expires_at: row.get(1), host_account: row.get(2)};
            Ok(info)
        },
        None => {
            Err(anyhow!("NOT_FOUND"))
        }
    }
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
