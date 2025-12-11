#![forbid(unsafe_code)]

use poem::Request;
use poem_openapi::{ OpenApi, payload::Json, Object, param::Path, ApiResponse };
use anyhow::Result;
use futures::executor::block_on;

use crate::utils::errors::HttpResult;
use crate::utils::db_statements::{INSERT_TENANT, INSERT_ADMIN};
use crate::utils::db_types::TenantInput;
use crate::utils::authz::{authorize, AuthzTypes, get_tenant_header}; 
use crate::utils::tms_utils::{self, timestamp_utc, timestamp_utc_to_str, create_hex_secret, hash_hex_secret, 
                              RequestDebug, check_tenant_enabled};
use crate::utils::config::{DEFAULT_TENANT, DEFAULT_ADMIN_ID, PERM_ADMIN};
use log::{error, info};

use crate::RUNTIME_CTX;

// ***************************************************************************
//                          Request/Response Definiions
// ***************************************************************************
pub struct CreateTenantsApi;

// ***************************************************************************
//                          Request/Response Definiions
// ***************************************************************************
#[derive(Object)]
pub struct ReqCreateTenants
{
    tenant: String,
}

#[derive(Object, Debug)]
pub struct RespCreateTenants
{
    result_code: String,
    result_msg: String,
    enabled: bool,
    tenant: String,
    admin_id: String,
    admin_secret: String,
}

// Implement the debug record trait for logging.
impl RequestDebug for ReqCreateTenants {   
    type Req = ReqCreateTenants;
    fn get_request_info(&self) -> String {
        let mut s = String::with_capacity(255);
        s.push_str("  Request body:");
        s.push_str("\n    tenant: ");
        s.push_str(&self.tenant);
        s
    }
}

// ------------------- HTTP Status Codes -------------------
#[derive(Debug, ApiResponse)]
enum TmsResponse {
    #[oai(status = 201)]
    Http201(Json<RespCreateTenants>),
    #[oai(status = 400)]
    Http400(Json<HttpResult>),
    #[oai(status = 401)]
    Http401(Json<HttpResult>),
    #[oai(status = 403)]
    Http403(Json<HttpResult>),
    #[oai(status = 500)]
    Http500(Json<HttpResult>),
}

fn make_http_201(resp: RespCreateTenants) -> TmsResponse {
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
impl CreateTenantsApi {
    #[oai(path = "/tms/tenants/:tenant", method = "post")]
    async fn create_tenant(&self, http_req: &Request, tenant: Path<String>) -> TmsResponse {
        // -------------------- Get Tenant Header --------------------
        // Get the required tenant header value.
        let hdr_tenant = match get_tenant_header(http_req) {
            Ok(t) => t,
            Err(e) => return make_http_400(e.to_string()),
        };

        // Check that the tenant specified in the header is the default tenant.
        if hdr_tenant != DEFAULT_TENANT {
            let msg = "ERROR: FORBIDDEN - Only admin users in the 'default' tenant can create new tenants.".to_string();
            error!("{}", msg);
            return make_http_403(msg);  
        }

        // Check tenant.
        if check_tenant_enabled(&hdr_tenant).await {
            return make_http_400("Tenant not enabled.".to_string());
        }

        // Create a request object.
        let req = ReqCreateTenants {tenant: tenant.to_string()};

        // -------------------- Authorize ----------------------------
        // Currently, only the tenant admin can create a user mfa record.
        // When user authentication is implemented, we'll add user-own 
        // authorization and any additional validation.
        let allowed = [AuthzTypes::TenantAdmin];
        let authz_result = authorize(http_req, &allowed);
        if !authz_result.is_authorized() {
            let msg = "ERROR: NOT AUTHORIZED to add a new tenant.".to_string();
            error!("{}", msg);
            return make_http_401(msg);
        }

        // -------------------- Process Request ----------------------
        match RespCreateTenants::process(http_req, &req) {
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
impl RespCreateTenants {
    /// Create a new response.
    fn new(result_code: &str, result_msg: String, enabled: bool, tenant: String,
           admin_id: String, admin_secret: String) -> Self {
        Self {result_code: result_code.to_string(), result_msg, enabled, tenant, admin_id, admin_secret}}

    /// Process the request.
    fn process(http_req: &Request, req: &ReqCreateTenants) -> Result<TmsResponse, anyhow::Error> {
        // Conditional logging depending on log level.
        tms_utils::debug_request(http_req, req);

        // Use the same current UTC timestamp in all related time caculations..
        let now = timestamp_utc();
        let current_ts = timestamp_utc_to_str(now);

        // Generate the admin user's secret.
        let key_str = create_hex_secret();
        let key_hash = hash_hex_secret(&key_str);
    
        // Create the input record.  Note that we save the hash of
        // the hex secret, but never the secret itself.  
        let input_record: TenantInput = TenantInput::new(
            req.tenant.clone(),
            1,
            key_hash,
            current_ts.clone(), 
            current_ts,
        );

        // Insert the new key record.
        block_on(insert_tenant(input_record))?;
        info!("New tenant '{}' created and enabled.", &req.tenant);
        
        // Return the secret represented in hex.
        Ok(make_http_201(Self::new("0", "success".to_string(), true, 
                         req.tenant.clone(), DEFAULT_ADMIN_ID.to_string(), key_str)))
    }
}

// ***************************************************************************
//                          Private Functions
// ***************************************************************************
// ---------------------------------------------------------------------------
// insert_tenant:
// ---------------------------------------------------------------------------
async fn insert_tenant(rec: TenantInput) -> Result<u64> {
    // Get a connection to the db and start a transaction.  Uncommited transactions 
    // are automatically rolled back when they go out of scope. 
    // See https://docs.rs/sqlx/latest/sqlx/struct.Transaction.html.
    let mut tx = RUNTIME_CTX.db.begin().await?;
    
    // Create the tenant insert statement.
    let result = sqlx::query(INSERT_TENANT)
        .bind(&rec.tenant)
        .bind(rec.enabled)
        .bind(&rec.created)
        .bind(&rec.updated)
        .execute(&mut *tx)
        .await?;

    // Create the new tenant's user id.
    let _dft_admin_result = sqlx::query(INSERT_ADMIN)
        .bind(&rec.tenant)
        .bind(DEFAULT_ADMIN_ID)
        .bind(&rec.key_hash)
        .bind(PERM_ADMIN)
        .bind(&rec.created)
        .bind(&rec.updated)
        .execute(&mut *tx)
        .await?;

    // Commit the transaction.
    tx.commit().await?;

    Ok(result.rows_affected())
}
