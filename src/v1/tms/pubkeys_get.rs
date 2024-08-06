#![forbid(unsafe_code)]

use poem::Request;
use poem_openapi::{ OpenApi, payload::Json, Object, param::Path };
use anyhow::{Result, anyhow};
use futures::executor::block_on;
use sqlx::Row;

use crate::utils::authz::{authorize, get_tenant_header, AuthzResult, AuthzTypes};
use crate::utils::tms_utils::{self, sql_substitute_client_constraint, RequestDebug};
use crate::utils::db_statements::GET_PUBKEY_TEMPLATE;
use crate::utils::db_types::Pubkey;
use log::error;

use crate::RUNTIME_CTX;

// ***************************************************************************
//                          Request/Response Definiions
// ***************************************************************************
pub struct GetPubkeysApi;

// ***************************************************************************
//                          Request/Response Definiions
// ***************************************************************************
#[derive(Object)]
struct ReqGetPubkeys
{
    seqno: i32,
    tenant: String,
}

#[derive(Object)]
pub struct RespGetPubkeys
{
    result_code: String,
    result_msg: String,
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
impl RequestDebug for ReqGetPubkeys {   
    type Req = ReqGetPubkeys;
    fn get_request_info(&self) -> String {
        let mut s = String::with_capacity(255);
        s.push_str("  Request body:");
        s.push_str("\n    row_id: ");
        s.push_str(&self.seqno.to_string());
        s.push_str("\n    tenant: ");
        s.push_str(&self.tenant);
        s
    }
}

// ***************************************************************************
//                             OpenAPI Endpoint
// ***************************************************************************
#[OpenApi]
impl GetPubkeysApi {
    #[oai(path = "/tms/pubkeys/id/:seqno", method = "get")]
    async fn get_client(&self, http_req: &Request, seqno: Path<i32>) -> Json<RespGetPubkeys> {
        // -------------------- Get Tenant Header --------------------
        // Get the required tenant header value.
        let hdr_tenant = match get_tenant_header(http_req) {
            Ok(t) => t,
            Err(e) => return Json(RespGetPubkeys::new_error("1", e.to_string(), *seqno, "".to_string())),
        };
        
        // Package the request parameters.        
        let req = ReqGetPubkeys {seqno: *seqno, tenant: hdr_tenant};
        
        // -------------------- Authorize ----------------------------
        // Only the tenant admin can query all client records; 
        // a client can query their own records.
        let allowed = [AuthzTypes::ClientOwn, AuthzTypes::TenantAdmin];
        let authz_result = authorize(http_req, &allowed);
        if !authz_result.is_authorized() {
            let msg = format!("ERROR: NOT AUTHORIZED to view pubkey #{} in tenant {}.", req.seqno, req.tenant);
            error!("{}", msg);
            return Json(RespGetPubkeys::new_error("1", msg, req.seqno, req.tenant));
        }

        // -------------------- Process Request ----------------------
        // Process the request.
        let resp = match RespGetPubkeys::process(http_req, &req, &authz_result) {
            Ok(r) => r,
            Err(e) => {
                let msg = "ERROR: ".to_owned() + e.to_string().as_str();
                error!("{}", msg);
                RespGetPubkeys::new_error("1", msg, req.seqno, req.tenant)},
        };

        Json(resp)
    }
}

// ***************************************************************************
//                          Request/Response Methods
// ***************************************************************************
impl RespGetPubkeys {
    /// Create a new response.
    #[allow(clippy::too_many_arguments)]
    fn new(    
        result_code: &str,
        result_msg: String,
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
    ) 
    -> Self {
            Self {result_code: result_code.to_string(), result_msg, 
                  id, tenant, client_id, client_user_id, host, host_account, public_key_fingerprint, 
                  public_key, key_type, key_bits, max_uses, remaining_uses, initial_ttl_minutes, 
                  expires_at, created, updated}
        }

    // Create an error response object with most fields defaulted.
    fn new_error(
        result_code: &str,
        result_msg: String,
        id: i32,
        tenant: String,
    )
    -> Self {
            Self {result_code: result_code.to_string(), result_msg, 
                id, tenant, client_id:"".to_string(), client_user_id:"".to_string(), host:"".to_string(), 
                host_account:"".to_string(), public_key_fingerprint:"".to_string(), public_key:"".to_string(), 
                key_type:"".to_string(), key_bits:0, max_uses:0, remaining_uses:0, initial_ttl_minutes:0, 
                expires_at:"".to_string(), created:"".to_string(), updated:"".to_string()}
    }
    /// Process the request.
    fn process(http_req: &Request, req: &ReqGetPubkeys, authz_result: &AuthzResult) -> Result<RespGetPubkeys, anyhow::Error> {
        // Conditional logging depending on log level.
        tms_utils::debug_request(http_req, req);

        // Search for the tenant/client id in the database.  Not found was already 
        // The client_secret is never part of the response.
        let pubkey = block_on(get_pubkey(authz_result, req))?;
        Ok(Self::new("0", "success".to_string(), pubkey.id, pubkey.tenant, pubkey.client_id, 
                     pubkey.client_user_id, 
                     pubkey.host, pubkey.host_account, pubkey.public_key_fingerprint, pubkey.public_key,
                     pubkey.key_type, pubkey.key_bits, pubkey.max_uses, pubkey.remaining_uses, pubkey.initial_ttl_minutes,
                     pubkey.expires_at, pubkey.created, pubkey.updated))
    }
}

// ***************************************************************************
//                          Private Functions
// ***************************************************************************
// ---------------------------------------------------------------------------
// get_pubkey:
// ---------------------------------------------------------------------------
async fn get_pubkey(authz_result: &AuthzResult, req: &ReqGetPubkeys) -> Result<Pubkey> {
    // Substitute the placeholder in the query template.  Uncommited transactions 
    // are automatically rolled back when they go out of scope. 
    // See https://docs.rs/sqlx/latest/sqlx/struct.Transaction.html.
    let sql_query = sql_substitute_client_constraint(GET_PUBKEY_TEMPLATE, authz_result); 

    // Get a connection to the db and start a transaction.
    let mut tx = RUNTIME_CTX.db.begin().await?;
    
    // Create the select statement.
    let result = sqlx::query(&sql_query)
        .bind(req.seqno)
        .bind(req.tenant.clone())
        .fetch_optional(&mut *tx)
        .await?;

    // Commit the transaction.
    tx.commit().await?;

    // We may have found the client. 
    match result {
        Some(row) => {
            Ok(Pubkey::new(row.get(0), row.get(1), row.get(2), 
                           row.get(3), row.get(4),
                           row.get(5), row.get(6), 
                           row.get(7), row.get(8), row.get(9), 
                           row.get(10), row.get(11), 
                           row.get(12), row.get(13), 
                           row.get(14), row.get(15)))
        },
        None => {
            Err(anyhow!("NOT_FOUND"))
        },
    }
}
