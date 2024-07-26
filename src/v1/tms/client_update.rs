#![forbid(unsafe_code)]

use poem::Request;
use poem_openapi::{ OpenApi, payload::Json, Object, param::Path, param::Query };
use anyhow::Result;
use futures::executor::block_on;

use crate::utils::db_statements::{UPDATE_CLIENT_APP_VERSION, UPDATE_CLIENT_ENABLED};
use crate::utils::tms_utils::{self, RequestDebug, timestamp_utc, timestamp_utc_to_str, validate_semver};
use crate::utils::authz::{authorize, AuthzTypes, get_tenant_header};
use log::{error, info};

use crate::RUNTIME_CTX;

// ***************************************************************************
//                          Request/Response Definiions
// ***************************************************************************
pub struct UpdateClientApi;

// ***************************************************************************
//                          Request/Response Definiions
// ***************************************************************************
#[derive(Object)]
pub struct ReqUpdateClient
{
    client_id: String,
    tenant: String,
    app_version: Option<String>,
    enabled: Option<bool>,
}

#[derive(Object)]
pub struct RespUpdateClient
{
    result_code: String,
    result_msg: String,
    fields_updated: i32,
}

// Implement the debug record trait for logging.
impl RequestDebug for ReqUpdateClient {   
    type Req = ReqUpdateClient;
    fn get_request_info(&self) -> String {
        // Get optional values in displayable form. 
        let app_version = format!("{:#?}", &self.app_version);
        let enabled = format!("{:#?}", &self.enabled);

        let mut s = String::with_capacity(255);
        s.push_str("  Request body:");
        s.push_str("\n    client_id: ");
        s.push_str(&self.client_id);
        s.push_str("\n    tenant: ");
        s.push_str(&self.tenant);
        s.push_str("\n    app_version: ");
        s.push_str(app_version.as_str());
        s.push_str("\n    enabled: ");
        s.push_str(enabled.as_str());
        s
    }
}

// ***************************************************************************
//                             OpenAPI Endpoint
// ***************************************************************************
#[OpenApi]
impl UpdateClientApi {
    #[oai(path = "/tms/client/:client_id", method = "patch")]
    async fn update_client(&self, http_req: &Request, client_id: Path<String>, 
                           app_version: Query<Option<String>>, enabled: Query<Option<bool>>) 
            -> Json<RespUpdateClient> {
        // -------------------- Get Tenant Header --------------------
        // Get the required tenant header value.
        let hdr_tenant = match get_tenant_header(http_req) {
            Ok(t) => t,
            Err(e) => return Json(RespUpdateClient::new("1", e.to_string(), 0)),
        };

        // Package the request parameters. The difference in query parameter processing 
        // is because Option<String> does not implement the copy trait, but Option<bool> does.
        let req = 
            ReqUpdateClient {client_id: client_id.to_string(), tenant: hdr_tenant, 
                             app_version: app_version.clone(), enabled: *enabled};

        // -------------------- Authorize ----------------------------
        // Only the client and tenant admin can query a client record.
        let allowed = [AuthzTypes::ClientOwn, AuthzTypes::TenantAdmin];
        let authz_result = authorize(http_req, &allowed);
        if !authz_result.is_authorized() {
            let msg = format!("NOT AUTHORIZED to update client {} in tenant {}.", req.client_id, req.tenant);
            error!("{}", msg);
            return Json(RespUpdateClient::new("1", msg, 0));
        }

        // Make sure the request parms conform to the header values used for authorization.
        if !authz_result.check_hdr_id(&req.client_id) {
            let msg = format!("NOT AUTHORIZED: Payload parameters ({}@{}) differ from those in the request header.", 
                                      req.client_id, req.tenant);
            error!("{}", msg);
            return Json(RespUpdateClient::new("1", msg, 0));
        }

        // -------------------- Process Request ----------------------
        // Process the request.
        let resp = match RespUpdateClient::process(http_req, &req) {
            Ok(r) => r,
            Err(e) => {
                let msg = "ERROR: ".to_owned() + e.to_string().as_str();
                error!("{}", msg);
                RespUpdateClient::new("1", msg, 0)},
        };

        Json(resp)
    }
}

// ***************************************************************************
//                          Request/Response Methods
// ***************************************************************************
impl RespUpdateClient {
    /// Create a new response.
    fn new(result_code: &str, result_msg: String, num_updates: i32,) -> Self {
        Self {result_code: result_code.to_string(), result_msg, fields_updated: num_updates}}

    /// Process the request.
    fn process(http_req: &Request, req: &ReqUpdateClient) -> Result<RespUpdateClient, anyhow::Error> {
        // Conditional logging depending on log level.
        tms_utils::debug_request(http_req, req);

        // Determine if any updates are required.
        if req.app_version.is_none() && req.enabled.is_none() {
            return Ok(RespUpdateClient::new("0", "No updates specified".to_string(), 0));
        } 

        // Insert the new key record.
        let updates = block_on(update_client(req))?;
        
        // Log result and return response.
        let msg = format!("{} update(s) to client {} completed", updates, req.client_id);
        info!("{}", msg);
        Ok(RespUpdateClient::new("0", msg, updates as i32))
    }
}

// ***************************************************************************
//                          Private Functions
// ***************************************************************************
// ---------------------------------------------------------------------------
// update_client:
// ---------------------------------------------------------------------------
async fn update_client(req: &ReqUpdateClient) -> Result<u64> {
    // Get timestamp.
    let now = timestamp_utc();
    let current_ts = timestamp_utc_to_str(now);

    // Get a connection to the db and start a transaction.
    let mut tx = RUNTIME_CTX.db.begin().await?;

    // Update count.
    let mut updates: u64 = 0;

    // Conditionally update the app version.
    if let Some(app_version) = &req.app_version {
        // Validate that the app version conforms cargo's implemenation of semantic versioning.
        validate_semver(app_version)?;

        // Issue the db update call.
        let result = sqlx::query(UPDATE_CLIENT_APP_VERSION)
            .bind(app_version)
            .bind(&current_ts)
            .bind(&req.client_id)
            .bind(&req.tenant)
            .execute(&mut *tx)
            .await?;
        updates += result.rows_affected();
    }

    // Conditionally update the app version.
    if let Some(enabled) = &req.enabled {
        // Issue the db update call.
        let result = sqlx::query(UPDATE_CLIENT_ENABLED)
            .bind(enabled)
            .bind(&current_ts)
            .bind(&req.client_id)
            .bind(&req.tenant)
            .execute(&mut *tx)
            .await?;
        updates += result.rows_affected();
    }

    // Commit the transaction.
    tx.commit().await?;
    Ok(updates)
}
