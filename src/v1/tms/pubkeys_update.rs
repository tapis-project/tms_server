#![forbid(unsafe_code)]

use std::cmp::max;

use poem::Request;
use poem_openapi::{ OpenApi, payload::Json, Object, };
use anyhow::{anyhow, Result};
use futures::executor::block_on;
use sqlx::Row;

use crate::utils::db_statements::{SELECT_PUBKEY_FOR_UPDATE, UPDATE_MAX_USES, UPDATE_EXPIRES_AT};
use crate::utils::tms_utils::{self, RequestDebug, timestamp_utc, timestamp_utc_to_str, calc_expires_at};
use crate::utils::authz::{authorize, AuthzTypes};
use log::{error, info};

use crate::RUNTIME_CTX;

// ***************************************************************************
//                          Request/Response Definiions
// ***************************************************************************
pub struct UpdatePubkeyApi;

// ***************************************************************************
//                          Request/Response Definiions
// ***************************************************************************
#[derive(Object)]
pub struct ReqUpdatePubkey
{
    client_id: String,
    tenant: String,
    host: String,
    public_key_fingerprint: String,
    max_uses: Option<u32>,     // 0 disables usage
    ttl_minutes: Option<u32>,  // 0 disables usage
}

#[derive(Object)]
pub struct RespUpdatePubkey
{
    result_code: String,
    result_msg: String,
    fields_updated: i32,
}

// Implement the debug record trait for logging.
impl RequestDebug for ReqUpdatePubkey {   
    type Req = ReqUpdatePubkey;
    fn get_request_info(&self) -> String {
        // Get optional values in displayable form. 
        let max_uses = format!("{:#?}", &self.max_uses);
        let ttl_minutes = format!("{:#?}", &self.ttl_minutes);

        let mut s = String::with_capacity(255);
        s.push_str("  Request body:");
        s.push_str("\n    client_id: ");
        s.push_str(&self.client_id);
        s.push_str("\n    tenant: ");
        s.push_str(&self.tenant);
        s.push_str("\n    host: ");
        s.push_str(&self.host);
        s.push_str("\n    public_key_fingerprint: ");
        s.push_str(&self.public_key_fingerprint);
        s.push_str("\n    max_uses: ");
        s.push_str(&max_uses);
        s.push_str("\n    ttl_minutes: ");
        s.push_str(&ttl_minutes);
        s
    }
}

struct SelectForUpdateResult {
    max_uses: i32,
    remaining_uses: i32,
}

impl SelectForUpdateResult {
    fn new(max_uses: i32, remaining_uses: i32) -> Self {
        Self {max_uses, remaining_uses}
    }
}

// ***************************************************************************
//                             OpenAPI Endpoint
// ***************************************************************************
#[OpenApi]
impl UpdatePubkeyApi {
    #[oai(path = "/tms/pubkeys", method = "patch")]
    async fn update_client(&self, http_req: &Request, req: Json<ReqUpdatePubkey>) 
        -> Json<RespUpdatePubkey> {
        // -------------------- Authorize ----------------------------
        // Only the client and tenant admin can query a client record.
        let allowed = [AuthzTypes::ClientOwn, AuthzTypes::TenantAdmin];
        let authz_result = authorize(http_req, &allowed);
        if !authz_result.is_authorized() {
            let msg = format!("ERROR NOT AUTHORIZED to update client {} in tenant {}.", req.client_id, req.tenant);
            error!("{}", msg);
            return Json(RespUpdatePubkey::new("1", msg, 0));
        }

        // Make sure the request parms conform to the header values used for authorization.
        if !authz_result.check_hdr_id(&req.client_id) || !authz_result.check_hdr_tenant(&req.tenant) {
            let msg = format!("ERROR: NOT AUTHORIZED - Payload parameters ({}@{}) differ from those in the request header.", 
                                      req.client_id, req.tenant);
            error!("{}", msg);
            return Json(RespUpdatePubkey::new("1", msg, 0));
        }

        // -------------------- Process Request ----------------------
        // Process the request.
        let resp = match RespUpdatePubkey::process(http_req, &req) {
            Ok(r) => r,
            Err(e) => {
                let msg = "ERROR: ".to_owned() + e.to_string().as_str();
                error!("{}", msg);
                RespUpdatePubkey::new("1", msg, 0)},
        };

        Json(resp)
    }
}

// ***************************************************************************
//                          Request/Response Methods
// ***************************************************************************
impl RespUpdatePubkey {
    /// Create a new response.
    fn new(result_code: &str, result_msg: String, num_updates: i32,) -> Self {
        Self {result_code: result_code.to_string(), result_msg, fields_updated: num_updates}}

    /// Process the request.
    fn process(http_req: &Request, req: &ReqUpdatePubkey) -> Result<RespUpdatePubkey, anyhow::Error> {
        // Conditional logging depending on log level.
        tms_utils::debug_request(http_req, req);

        // Determine if any updates are required.
        if req.max_uses.is_none() && req.ttl_minutes.is_none() {
            return Ok(RespUpdatePubkey::new("0", "No updates specified".to_string(), 0));
        } 

        // Insert the new key record.
        let updates = block_on(update_pubkey(req))?;
        
        // Log result and return response.
        let msg = format!("{} update(s) to client {} completed", updates, req.client_id);
        info!("{}", msg);
        Ok(RespUpdatePubkey::new("0", msg, updates as i32))
    }
}

// ***************************************************************************
//                          Private Functions
// ***************************************************************************
// ---------------------------------------------------------------------------
// update_pubkey:
// ---------------------------------------------------------------------------
/** Ths function issues a SQL SELECT and then 1 or 2 UPDATE calls in the same
 * tranaction.  Sqlite documentation indicates that upgrading from a read lock
 * to a r/w lock during a transaction can fail with a SQLITE_BUSY_SNAPSHOT error
 * if another transaction changed the database.  Other databases use 
 * SELECT FOR UPDATE to avoid these types of concurrency problems.  See the
 * Sqlite documentaton for details:  https://www.sqlite.org/isolation.html 
 * 
 * Currently, this code will fail the user's request when such a concurrency 
 * conflict occurs.  If this becomes a problem for users, we can institute 
 * automatic retries or take other remedial measures.
 */
async fn update_pubkey(req: &ReqUpdatePubkey) -> Result<u64> {
    // Safely convert u32s to i32s.  The result is always either 
    // (1) zero or a positive number or (2) -1 which indicates no user 
    // input.  This function won't be called if the user doesn't 
    // specify at least one update.  Zero max_uses forces no remaining 
    // uses; zero ttl_minutes forces immediate expiration.
    const I32MAX: u32 = i32::MAX as u32;
    let max_uses: i32 = match req.max_uses{
        Some(num) => if num > I32MAX {i32::MAX} else {num as i32},
        None => -1, // no user input
    };
    let ttl_minutes: i32 = match req.ttl_minutes {
        Some(num) => if num > I32MAX {i32::MAX} else {num as i32},
        None => -1, // no user input
    };
    
    // Get timestamp.
    let now = timestamp_utc();
    let current_ts = timestamp_utc_to_str(now);

    // Get a connection to the db and start a transaction.  Uncommited transactions 
    // are automatically rolled back when they go out of scope. 
    // See https://docs.rs/sqlx/latest/sqlx/struct.Transaction.html.
    let mut tx = RUNTIME_CTX.db.begin().await?;

    // -------------------- Get Updateable Field Values --------------------
    // Create the select statement.
    let result = sqlx::query(SELECT_PUBKEY_FOR_UPDATE)
        .bind(&req.client_id)
        .bind(&req.tenant)
        .bind(&req.host)
        .bind(&req.public_key_fingerprint)
        .fetch_optional(&mut *tx)
        .await?;

    // Update count.
    let mut updates: u64 = 0;

    // ------------------------------- max_uses ----------------------------
    // Conditionally update the max uses and remaining uses.
    if max_uses > -1 {
        // Retrieve the current db values.
        let select_result = match result {
            Some(row) => {
                SelectForUpdateResult::new(row.get(0), row.get(1))
            },
            None => {
                return Err(anyhow!("NOT_FOUND"))
            },
        };

        // Calculate the new number of remaining uses to never be less than 0.
        let already_used = select_result.max_uses - select_result.remaining_uses;
        let remaining_uses = max(max_uses - already_used, 0);

        // Issue the db update call.
        let result = sqlx::query(UPDATE_MAX_USES)
            .bind(max_uses)
            .bind(remaining_uses)
            .bind(&current_ts)
            .bind(&req.client_id)
            .bind(&req.tenant)
            .bind(&req.host)
            .bind(&req.public_key_fingerprint)
            .execute(&mut *tx)
            .await?;
        updates += result.rows_affected();
    }

    // Conditionally update the expiration time.
    if ttl_minutes > -1 {
        // Calculate the new expiration date/time, which will
        // be the current time if the ttl_minutes is zero.
        let expires_at = calc_expires_at(now, ttl_minutes);

        // Issue the db update call.
        let result = sqlx::query(UPDATE_EXPIRES_AT)
            .bind(expires_at)
            .bind(&current_ts)
            .bind(&req.client_id)
            .bind(&req.tenant)
            .bind(&req.host)
            .bind(&req.public_key_fingerprint)
            .execute(&mut *tx)
            .await?;
        updates += result.rows_affected();
    }

    // Commit the transaction.
    tx.commit().await?;
    Ok(updates)
}

