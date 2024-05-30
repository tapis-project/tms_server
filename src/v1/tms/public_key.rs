#![forbid(unsafe_code)]

use poem::Request;
use poem_openapi::{  OpenApi, payload::Json, Object };
use anyhow::Result;
use futures::executor::block_on;
use sqlx::Row;

use crate::utils::db_statements::SELECT_PUBKEY;
use crate::utils::db_types::PubkeyRetrieval;
use crate::utils::tms_utils;
use crate::RUNTIME_CTX;

// ***************************************************************************
//                          Request/Response Definiions
// ***************************************************************************
pub struct PublicKeyApi;

#[derive(Object)]
struct ReqPublicKey
{
    user: String,
    user_uid: String,
    user_home_dir: String,
    host: String,
    public_key_fingerprint: String, // protocol:base64hash format
    requestor_host: String,
    requestor_addr: String,
}

#[derive(Object)]
struct RespPublicKey
{
    result_code: String,
    result_msg: String,
    public_key: String,
}

// ***************************************************************************
//                             OpenAPI Endpoint
// ***************************************************************************
#[OpenApi]
impl PublicKeyApi {
    #[oai(path = "/tms/creds/publickey", method = "post")]
    async fn get_public_key(&self, http_req: &Request, req: Json<ReqPublicKey>) -> Json<RespPublicKey> {
        let resp = match RespPublicKey::process(http_req, &req) {
            Ok(r) => r,
            Err(e) => {
                let msg = "ERROR: ".to_owned() + e.to_string().as_str();
                RespPublicKey::new("1", msg.as_str(), "")},
        };

        Json(resp)
    }
}

// ***************************************************************************
//                          Request/Response Methods
// ***************************************************************************
impl RespPublicKey {
    fn new(result_code: &str, result_msg: &str, key: &str) -> Self {
        Self {result_code: result_code.to_string(), 
              result_msg: result_msg.to_string(), 
              public_key: key.to_string()}
    }

    fn process(http_req: &Request, req: &ReqPublicKey) -> Result<RespPublicKey> {
        // Conditional logging depending on log level.
        tms_utils::debug_request(http_req);

        // Look for the key in the database.
        let result = block_on(get_public_key(req))?;
        if "NOT_FOUND" == result.public_key {
            Ok(Self::new("1", "NOT_FOUND", ""))
        } else {
            Ok(Self::new("0", "success", result.public_key.as_str()))
        }
    }
}

// ***************************************************************************
//                          Private Functions
// ***************************************************************************
// ---------------------------------------------------------------------------
// get_public_key:
// ---------------------------------------------------------------------------
async fn get_public_key(req: &ReqPublicKey) -> Result<PubkeyRetrieval> {
    // Get a connection to the db and start a transaction.
    let mut tx = RUNTIME_CTX.db.begin().await?;
    
    // Create the insert statement.
    let result = sqlx::query(SELECT_PUBKEY)
        .bind(req.user.clone())
        .bind(req.host.clone())
        .bind(req.public_key_fingerprint.clone())
        .fetch_optional(&mut *tx)
        .await?;

    // Commit the transaction.
    tx.commit().await?;

    // We found the key!
    match result {
        Some(row) => {
            Ok(PubkeyRetrieval::new(row.get(0), row.get(1), row.get(2)))
        },
        None => {
            Ok(PubkeyRetrieval::new("NOT_FOUND".to_string(), 0, "".to_string()))
        },
    }
}
