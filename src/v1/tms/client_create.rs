#![forbid(unsafe_code)]

use poem::Request;
use poem_openapi::{ OpenApi, payload::Json, Object };
use anyhow::{anyhow, Result};
use futures::executor::block_on;

use crate::utils::db_statements::INSERT_CLIENTS;
use crate::utils::db_types::ClientInput; 
use crate::utils::tms_utils::{self, create_hex_secret, hash_hex_secret, timestamp_utc, timestamp_utc_to_str, 
                              RequestDebug, validate_semver};
use log::{error, info};

use crate::RUNTIME_CTX;

// ***************************************************************************
//                          Request/Response Definiions
// ***************************************************************************
pub struct CreateClientApi;

// ***************************************************************************
//                          Request/Response Definiions
// ***************************************************************************
#[derive(Object)]
pub struct ReqCreateClient
{
    client_id: String,
    tenant: String,
    app_name: String,
    app_version: String,
}

#[derive(Object)]
pub struct RespCreateClient
{
    result_code: String,
    result_msg: String,
    client_id: String,
    client_secret: String,
}

// Implement the debug record trait for logging.
impl RequestDebug for ReqCreateClient {   
    type Req = ReqCreateClient;
    fn get_request_info(&self) -> String {
        let mut s = String::with_capacity(255);
        s.push_str("  Request body:");
        s.push_str("\n    client_id: ");
        s.push_str(&self.client_id);
        s.push_str("\n    tenant: ");
        s.push_str(&self.tenant);
        s.push_str("\n    app_name: ");
        s.push_str(&self.app_name);
        s.push_str("\n    app_version: ");
        s.push_str(&self.app_version);
        s
    }
}

// ***************************************************************************
//                             OpenAPI Endpoint
// ***************************************************************************
#[OpenApi]
impl CreateClientApi {
    #[oai(path = "/tms/client", method = "post")]
    async fn create_client(&self, http_req: &Request, req: Json<ReqCreateClient>) -> Json<RespCreateClient> {
        let resp = match RespCreateClient::process(http_req, &req) {
            Ok(r) => r,
            Err(e) => {
                let msg = "ERROR: ".to_owned() + e.to_string().as_str();
                error!("{}", msg);
                RespCreateClient::new("1", msg.as_str(), req.client_id.clone(), "NO SECRET CREATED".to_string() )},
        };

        Json(resp)
    }
}

// ***************************************************************************
//                          Request/Response Methods
// ***************************************************************************
impl RespCreateClient {
    /// Create a new response.
    fn new(result_code: &str, result_msg: &str, client_id: String, client_secret: String,) -> Self {
        Self {result_code: result_code.to_string(), 
              result_msg: result_msg.to_string(), 
              client_id,
              client_secret,
            }
    }

    /// Process the request.
    fn process(http_req: &Request, req: &ReqCreateClient) -> Result<RespCreateClient, anyhow::Error> {
        // Conditional logging depending on log level.
        tms_utils::debug_request(http_req, req);

        // ------------------------ Validate Version -------------------
        // Only valid semantic versions are accepted.
        match validate_semver(req.app_version.as_str()) {
            Ok(_) => (),
            Err(e) => {
                let msg = format!("Invalid app_version value ({}): {}", req.app_version, e);
                error!("{}", msg);
                return Err(anyhow!(msg));
            }
        };

        // ------------------------ Generate Secret --------------------  
        let client_secret_str  = create_hex_secret();
        let client_secret_hash = hash_hex_secret(&client_secret_str);

        // ------------------------ Update Database --------------------
        let now = timestamp_utc();
        let current_ts = timestamp_utc_to_str(now);

        // Create the input record.  Note that we save the hash of
        // the hex secret, but never the secret itself.  
        let input_record = ClientInput::new(
            req.tenant.clone(),
            req.app_name.clone(),
            req.app_version.clone(),
            req.client_id.clone(),
            client_secret_hash, 
            1,
            current_ts.clone(), 
            current_ts,
        );

        // Insert the new key record.
        block_on(insert_new_client(input_record))?;
        info!("Client '{}' created for application '{}:{}' in tenant '{}'.", 
              req.client_id, req.app_name, req.app_version, req.tenant);
        
        // Return the secret represented in hex.
        Ok(Self::new("0", "success", req.client_id.clone(), client_secret_str))
    }
}

// ***************************************************************************
//                          Private Functions
// ***************************************************************************
// ---------------------------------------------------------------------------
// insert_new_client:
// ---------------------------------------------------------------------------
async fn insert_new_client(rec: ClientInput) -> Result<u64> {
    // Get a connection to the db and start a transaction.
    let mut tx = RUNTIME_CTX.db.begin().await?;
    
    // Create the insert statement.
    let result = sqlx::query(INSERT_CLIENTS)
        .bind(rec.tenant)
        .bind(rec.app_name)
        .bind(rec.app_version)
        .bind(rec.client_id)
        .bind(rec.client_secret)
        .bind(rec.enabled)
        .bind(rec.created)
        .bind(rec.updated)
        .execute(&mut *tx)
        .await?;

    // Commit the transaction.
    tx.commit().await?;

    Ok(result.rows_affected())
}
