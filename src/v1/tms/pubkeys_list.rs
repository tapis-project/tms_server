#![forbid(unsafe_code)]

use poem::Request;
use poem_openapi::{ OpenApi, payload::Json, Object, ApiResponse };
use anyhow::Result;
use futures::executor::block_on;
use sqlx::Row;

use crate::utils::errors::HttpResult;
use crate::utils::authz::{authorize, AuthzTypes, get_tenant_header, AuthzResult};
use crate::utils::db_statements::LIST_PUBKEYS_TEMPLATE;
use crate::utils::tms_utils::{self, RequestDebug, sql_substitute_client_constraint, check_tenant_enabled};
use log::error;

use crate::RUNTIME_CTX;

// ***************************************************************************
//                          Request/Response Definiions
// ***************************************************************************
pub struct ListPubkeysApi;

// ***************************************************************************
//                          Request/Response Definiions
// ***************************************************************************
#[derive(Object)]
struct ReqListPubkeys
{
    tenant: String,
}

#[derive(Object, Debug)]
pub struct RespListPubkeys
{
    result_code: String,
    result_msg: String,
    num_pubkeys: i32,
    pubkeys: Vec<PubkeysListElement>,
}

#[derive(Object, Debug)]
pub struct PubkeysListElement
{
    id: i32,
    tenant: String,
    client_id: String,
    client_user_id: String,
    host: String,
    host_account: String,
    public_key_fingerprint: String,
    public_key: String,
    key_type: String,
    key_bits: i32,
    max_uses: i32,
    remaining_uses: i32,
    initial_ttl_minutes: i32,
    expires_at: String,
    created: String,
    updated: String,
}

// Implement the debug record trait for logging.
impl RequestDebug for ReqListPubkeys {   
    type Req = ReqListPubkeys;
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
    Http200(Json<RespListPubkeys>),
    #[oai(status = 400)]
    Http400(Json<HttpResult>),
    #[oai(status = 401)]
    Http401(Json<HttpResult>),
    #[oai(status = 500)]
    Http500(Json<HttpResult>),
}

fn make_http_200(resp: RespListPubkeys) -> TmsResponse {
    TmsResponse::Http200(Json(resp))
}
fn make_http_400(msg: String) -> TmsResponse {
    TmsResponse::Http400(Json(HttpResult::new(400.to_string(), msg)))
}
fn make_http_401(msg: String) -> TmsResponse {
    TmsResponse::Http401(Json(HttpResult::new(401.to_string(), msg)))
}
fn make_http_500(msg: String) -> TmsResponse {
    TmsResponse::Http500(Json(HttpResult::new(500.to_string(), msg)))    
}

// ***************************************************************************
//                             OpenAPI Endpoint
// ***************************************************************************
#[OpenApi]
impl ListPubkeysApi {
    #[oai(path = "/tms/pubkeys/list", method = "get")]
    async fn get_pubkeys(&self, http_req: &Request) -> TmsResponse {
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
        let req = ReqListPubkeys {tenant: hdr_tenant};
        
        // -------------------- Authorize ----------------------------
        // Only the tenant admin can query all client records; 
        // a client can query their own records.
        let allowed = [AuthzTypes::ClientOwn, AuthzTypes::TenantAdmin];
        let authz_result = authorize(http_req, &allowed);
        if !authz_result.is_authorized() {
            let msg = format!("ERROR: NOT AUTHORIZED to list clients in tenant {}.", req.tenant);
            error!("{}", msg);
            return make_http_401(msg);
        }

        // -------------------- Process Request ----------------------
        // Process the request.
        match RespListPubkeys::process(http_req, &req, &authz_result) {
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
impl PubkeysListElement {
    /// Create response elements.
    #[allow(clippy::too_many_arguments)]
    fn new(id: i32, tenant: String, client_id: String, client_user_id: String, 
           host: String, host_account: String, public_key_fingerprint: String, 
           public_key: String, key_type: String, key_bits: i32, max_uses: i32,
           remaining_uses: i32, initial_ttl_minutes: i32, expires_at: String, 
           created: String, updated: String) -> Self {
        Self {id, tenant, client_id, client_user_id, host, host_account, public_key_fingerprint,
              public_key, key_type, key_bits, max_uses, remaining_uses, initial_ttl_minutes,
              expires_at, created, updated}
    }
}

impl RespListPubkeys {
    /// Create a new response.
    #[allow(clippy::too_many_arguments)]
    fn new(result_code: &str, result_msg: String, num_keys: i32, keys: Vec<PubkeysListElement>) 
    -> Self {
        Self {result_code: result_code.to_string(), result_msg, num_pubkeys: num_keys, pubkeys: keys}
        }

    /// Process the request.
    fn process(http_req: &Request, req: &ReqListPubkeys, authz_result: &AuthzResult) -> Result<TmsResponse, anyhow::Error> {
        // Conditional logging depending on log level.
        tms_utils::debug_request(http_req, req);

        // Search for the tenant/client ids in the database.  
        let keys = block_on(list_pubkeys(authz_result, req))?;
        Ok(make_http_200(Self::new("0", "success".to_string(), keys.len() as i32, keys)))
    }
}

// ***************************************************************************
//                          Private Functions
// ***************************************************************************
// ---------------------------------------------------------------------------
// list_pubkeys:
// ---------------------------------------------------------------------------
async fn list_pubkeys(authz_result: &AuthzResult, req: &ReqListPubkeys) -> Result<Vec<PubkeysListElement>> {
    // Substitute the placeholder in the query template.
    let sql_query = sql_substitute_client_constraint(LIST_PUBKEYS_TEMPLATE, authz_result); 
    
    // Get a connection to the db and start a transaction.  Uncommited transactions 
    // are automatically rolled back when they go out of scope. 
    // See https://docs.rs/sqlx/latest/sqlx/struct.Transaction.html.
    let mut tx = RUNTIME_CTX.db.begin().await?;
    
    // Create the select statement.
    let rows = sqlx::query(&sql_query)
        .bind(req.tenant.clone())
        .fetch_all(&mut *tx)
        .await?;

    // Commit the transaction.
    tx.commit().await?;

    // Collect the row data into element objects.
    let mut element_list: Vec<PubkeysListElement> = vec!();
    for row in rows {
        let elem = PubkeysListElement::new(
            row.get(0), row.get(1), row.get(2), 
            row.get(3), row.get(4), row.get(5),
            row.get(6), row.get(7), 
            row.get(8), row.get(9),
            row.get(10), row.get(11), row.get(12), 
            row.get(13), row.get(14), row.get(15));
        element_list.push(elem);
    }

    Ok(element_list)
}
