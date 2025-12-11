#![forbid(unsafe_code)]

use poem::Request;
use poem_openapi::{ OpenApi, payload::Json, Object, ApiResponse };
use anyhow::Result;
use futures::executor::block_on;

use crate::utils::errors::HttpResult;
use crate::utils::db_statements::INSERT_HOSTS;
use crate::utils::db_types::HostInput;
use crate::utils::authz::{authorize, AuthzTypes, get_tenant_header, X_TMS_TENANT}; 
use crate::utils::tms_utils::{self, timestamp_utc, timestamp_utc_to_str, RequestDebug, check_tenant_enabled};
use log::{error, info};

use crate::RUNTIME_CTX;

// ***************************************************************************
//                          Request/Response Definiions
// ***************************************************************************
pub struct CreateHostsApi;

// ***************************************************************************
//                          Request/Response Definiions
// ***************************************************************************
#[derive(Object)]
pub struct ReqCreateHost
{
    tenant: String,
    host: String,
    addr: String,
}

#[derive(Object, Debug)]
pub struct RespCreateHost
{
    result_code: String,
    result_msg: String,
    tenant: String,
    host: String,
    addr: String,
}

// Implement the debug record trait for logging.
impl RequestDebug for ReqCreateHost {   
    type Req = ReqCreateHost;
    fn get_request_info(&self) -> String {
        let mut s = String::with_capacity(255);
        s.push_str("  Request body:");
        s.push_str("\n    tenant: ");
        s.push_str(&self.tenant);
        s.push_str("\n    host: ");
        s.push_str(&self.host);
        s.push_str("\n    addr: ");
        s.push_str(&self.addr);
        s
    }
}

// ------------------- HTTP Status Codes -------------------
#[derive(Debug, ApiResponse)]
enum TmsResponse {
    #[oai(status = 201)]
    Http201(Json<RespCreateHost>),
    #[oai(status = 400)]
    Http400(Json<HttpResult>),
    #[oai(status = 401)]
    Http401(Json<HttpResult>),
    #[oai(status = 403)]
    Http403(Json<HttpResult>),
    #[oai(status = 500)]
    Http500(Json<HttpResult>),
}

fn make_http_201(resp: RespCreateHost) -> TmsResponse {
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
fn make_http_500(msg: String) -> TmsResponse {
    TmsResponse::Http500(Json(HttpResult::new(500.to_string(), msg)))    
}

// ***************************************************************************
//                             OpenAPI Endpoint
// ***************************************************************************
#[OpenApi]
impl CreateHostsApi {
    #[oai(path = "/tms/hosts", method = "post")]
    async fn create_host_mapping(&self, http_req: &Request, req: Json<ReqCreateHost>) -> TmsResponse {
        // -------------------- Get Tenant Header --------------------
        // Get the required tenant header value.
        let hdr_tenant = match get_tenant_header(http_req) {
            Ok(t) => t,
            Err(e) => return make_http_400(e.to_string()),
        };

        // Check that the tenant specified in the header is the default tenant.
        if hdr_tenant != req.tenant {
            let msg = format!("ERROR: FORBIDDEN - The tenant in the {} header ({}) does not match the tenant in the request body ({})", 
                                      X_TMS_TENANT, hdr_tenant, req.tenant);
            error!("{}", msg);
            return make_http_403(msg);  
        }

        // Check tenant.
        if check_tenant_enabled(&hdr_tenant).await {
            return make_http_400("Tenant not enabled.".to_string());
        }

        // -------------------- Authorize ----------------------------
        // Currently, only the tenant admin can create a user mfa record.
        // When user authentication is implemented, we'll add user-own 
        // authorization and any additional validation.
        let allowed = [AuthzTypes::TenantAdmin];
        let authz_result = authorize(http_req, &allowed);
        if !authz_result.is_authorized() {
            let msg = "ERROR: NOT AUTHORIZED to add a new host mapping.".to_string();
            error!("{}", msg);
            return make_http_401(msg);
        }

        // -------------------- Process Request ----------------------
        match RespCreateHost::process(http_req, &req) {
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
impl RespCreateHost {
    /// Create a new response.
    fn new(result_code: &str, result_msg: String, tenant: String, host: String, addr: String) 
        -> Self {Self {result_code: result_code.to_string(), result_msg, tenant, host, addr}}

    /// Process the request.
    fn process(http_req: &Request, req: &ReqCreateHost) -> Result<TmsResponse, anyhow::Error> {
        // Conditional logging depending on log level.
        tms_utils::debug_request(http_req, req);

        // Use the same current UTC timestamp in all related time caculations..
        let now = timestamp_utc();
        let current_ts = timestamp_utc_to_str(now);
    
        // Create the input record.  Note that we save the hash of
        // the hex secret, but never the secret itself.  
        let input_record: HostInput = HostInput::new(
            req.tenant.clone(),
            req.host.clone(),
            req.addr.clone(),
            current_ts.clone(), 
            current_ts,
        );

        // Insert the new key record.
        block_on(insert_tenant(input_record))?;
        info!("New host mapping in tenant '{}' created, {} -> {}.", &req.tenant, &req.host, &req.addr);
        
        // Return the secret represented in hex.
        Ok(make_http_201(Self::new("0", "success".to_string(), req.tenant.clone(), 
                         req.host.clone(), req.addr.clone())))
    }
}

// ***************************************************************************
//                          Private Functions
// ***************************************************************************
// ---------------------------------------------------------------------------
// insert_tenant:
// ---------------------------------------------------------------------------
async fn insert_tenant(rec: HostInput) -> Result<u64> {
    // Get a connection to the db and start a transaction.  Uncommited transactions 
    // are automatically rolled back when they go out of scope. 
    // See https://docs.rs/sqlx/latest/sqlx/struct.Transaction.html.
    let mut tx = RUNTIME_CTX.db.begin().await?;
    
    // Create the tenant insert statement.
    let result = sqlx::query(INSERT_HOSTS)
        .bind(&rec.tenant)
        .bind(&rec.host)
        .bind(&rec.addr)
        .bind(&rec.created)
        .bind(&rec.updated)
        .execute(&mut *tx)
        .await?;

    // Commit the transaction.
    tx.commit().await?;

    Ok(result.rows_affected())
}
