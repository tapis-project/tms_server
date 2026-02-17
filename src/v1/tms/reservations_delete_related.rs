#![forbid(unsafe_code)]

use poem::Request;
use poem_openapi::{ OpenApi, payload::Json, Object, param::Path, ApiResponse };
use anyhow::Result;

use crate::utils::errors::HttpResult;
use crate::utils::authz::{authorize, AuthzTypes, get_tenant_header};
use crate::utils::db_statements::DELETE_RELATED_RESERVATIONS;
use crate::utils::tms_utils::{self, RequestDebug, check_tenant_enabled};
use log::{info, error};

use crate::RUNTIME_CTX;

/** Delete a reservation and all its children. 
 */

// ***************************************************************************
//                          Request/Response Definiions
// ***************************************************************************
pub struct DeleteRelatedReservationsApi;

// ***************************************************************************
//                          Request/Response Definiions
// ***************************************************************************
#[derive(Object)]
struct ReqDeleteRelatedReservations
{
    resid: String,
    client_id: String,
    tenant: String,
}

#[derive(Object, Debug)]
pub struct RespDeleteRelatedReservations
{
    result_code: String,
    result_msg: String,
    num_deleted: u32,
}

// Implement the debug record trait for logging.
impl RequestDebug for ReqDeleteRelatedReservations
{
    type Req = ReqDeleteRelatedReservations;
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
    Http200(Json<RespDeleteRelatedReservations>),
    #[oai(status = 400)]
    Http400(Json<HttpResult>),
    #[oai(status = 401)]
    Http401(Json<HttpResult>),
    #[oai(status = 403)]
    Http403(Json<HttpResult>),
    #[oai(status = 500)]
    Http500(Json<HttpResult>),
}

fn make_http_200(resp: RespDeleteRelatedReservations) -> TmsResponse {
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
impl DeleteRelatedReservationsApi {
    #[oai(path = "/tms/reservations/del/related/:client_id/:resid", method = "delete")]
    async fn delete_reservations_api(&self, http_req: &Request, client_id: Path<String>, resid: Path<String>) -> TmsResponse {
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
        let req = 
            ReqDeleteRelatedReservations {resid: resid.to_string(), client_id: client_id.to_string(), tenant: hdr_tenant};
        
        // -------------------- Authorize ----------------------------
        // Only the client and tenant admin can delete a reservation record.
        let allowed = [AuthzTypes::ClientOwn, AuthzTypes::TenantAdmin];
        let authz_result = authorize(http_req, &allowed).await;
        if !authz_result.is_authorized() {
            let msg = format!("ERROR: NOT AUTHORIZED to delete reservation {} in tenant {}.", req.resid, req.tenant);
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
        match RespDeleteRelatedReservations::process(http_req, &req).await {
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
impl RespDeleteRelatedReservations {
    // Create a new response.
    fn new(result_code: &str, result_msg: String, num_deleted: u32) -> Self {
        Self {result_code: result_code.to_string(), result_msg, num_deleted,}}

    // Process the request.
    async fn process(http_req: &Request, req: &ReqDeleteRelatedReservations) -> Result<TmsResponse, anyhow::Error> {
        // Conditional logging depending on log level.
        tms_utils::debug_request(http_req, req);

        // Insert the new key record.
        let deletes = delete_reservations(req).await?;
        
        // Log result and return response.
        let msg = 
            if deletes < 1 {format!("Reservation {} for {}@{} NOT FOUND - Nothing deleted", req.resid, req.client_id, req.tenant)}
            else {format!("Reservation {} deleted along with all its children.", req.resid)};
        info!("{}", msg);
        Ok(make_http_200(RespDeleteRelatedReservations::new("0", msg, deletes as u32)))
    }
}

// ***************************************************************************
//                          Private Functions
// ***************************************************************************
// ---------------------------------------------------------------------------
// delete_reservations:
// ---------------------------------------------------------------------------
async fn delete_reservations(req: &ReqDeleteRelatedReservations) -> Result<u64> {
    // Get a connection to the db and start a transaction.  Uncommited transactions 
    // are automatically rolled back when they go out of scope. 
    // See https://docs.rs/sqlx/latest/sqlx/struct.Transaction.html.
    let mut tx = RUNTIME_CTX.db.begin().await?;

    // Deletion count.
    let mut deletes: u64 = 0;

    // Issue the db delete call.
    let result = sqlx::query(DELETE_RELATED_RESERVATIONS)
        .bind(&req.resid)
        .bind(&req.resid)
        .bind(&req.client_id)
        .bind(&req.tenant)
        .execute(&mut *tx)
        .await?;
    deletes += result.rows_affected();

    // Commit the transaction.
    tx.commit().await?;
    Ok(deletes)
}
