#![forbid(unsafe_code)]

use poem::Request;
use poem_openapi::{ OpenApi, payload::Json, Object, param::Path, ApiResponse };
use anyhow::{Result, anyhow};
use sqlx::Row;

use crate::utils::errors::HttpResult;
use crate::utils::db_statements::GET_TENANT;
use crate::utils::tms_utils::{self, RequestDebug};
use crate::utils::db_types::Tenant;
use log::error;

use crate::RUNTIME_CTX;

// ***************************************************************************
//                          Request/Response Definiions
// ***************************************************************************
pub struct GetTenantsApi;

// ***************************************************************************
//                          Request/Response Definiions
// ***************************************************************************
#[derive(Object)]
struct ReqGetTenants
{
    tenant: String,
}

#[derive(Object, Debug)]
pub struct RespGetTenants
{
    result_code: String,
    result_msg: String,
    id: i32,
    tenant: String,
    enabled: i32,
    created: String,
    updated: String,
}

// Implement the debug record trait for logging.
impl RequestDebug for ReqGetTenants {   
    type Req = ReqGetTenants;
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
    #[oai(status = 200)]
    Http200(Json<RespGetTenants>),
    #[oai(status = 404)]
    Http404(Json<HttpResult>),
    #[oai(status = 500)]
    Http500(Json<HttpResult>),
}

fn make_http_200(resp: RespGetTenants) -> TmsResponse {
    TmsResponse::Http200(Json(resp))
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
impl GetTenantsApi {
    #[oai(path = "/tms/tenants/:tenant", method = "get")]
    async fn get_tenant_api(&self, http_req: &Request, tenant: Path<String>) -> TmsResponse {
        // Package the request parameters.        
        let req = ReqGetTenants {tenant: tenant.to_string()};
        
        // -------------------- Process Request ----------------------
        // Process the request.
        match RespGetTenants::process(http_req, &req).await {
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
impl RespGetTenants {
    /// Create a new response.
    #[allow(clippy::too_many_arguments)]
    fn new(result_code: &str, result_msg: String, id: i32, tenant: String, 
           enabled: i32, created: String, updated: String) 
    -> Self {
            Self {result_code: result_code.to_string(), result_msg, 
                  id, tenant, enabled, created, updated}
        }

    /// Process the request.
    async fn process(http_req: &Request, req: &ReqGetTenants) -> Result<TmsResponse, anyhow::Error> {
        // Conditional logging depending on log level.
        tms_utils::debug_request(http_req, req);

        // Search for the tenant/client id in the database. Note that even disabled
        // tenants can see tenant definitions. The client_secret is never part of the response.
        let db_result = get_tenant_by_name(req).await;
        match db_result {
            Ok(u) => Ok(make_http_200(Self::new("0", "success".to_string(), u.id, u.tenant, 
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
// get_tenant_by_name:
// ---------------------------------------------------------------------------
async fn get_tenant_by_name(req: &ReqGetTenants) -> Result<Tenant> {
    // Get a connection to the db and start a transaction.  Uncommited transactions 
    // are automatically rolled back when they go out of scope. 
    // See https://docs.rs/sqlx/latest/sqlx/struct.Transaction.html.
    let mut tx = RUNTIME_CTX.db.begin().await?;
    
    // Create the select statement.
    let result = sqlx::query(GET_TENANT)
        .bind(&req.tenant)
        .fetch_optional(&mut *tx)
        .await?;

    // Commit the transaction.
    tx.commit().await?;

    // We may have found the tenant.
    match result {
        Some(row) => {
            Ok(Tenant::new(row.get(0), row.get(1), row.get(2), 
                            row.get(3), row.get(4)))
        },
        None => {
            Err(anyhow!("NOT_FOUND"))
        },
    }
}
