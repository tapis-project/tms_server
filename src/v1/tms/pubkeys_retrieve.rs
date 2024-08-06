#![forbid(unsafe_code)]

use poem::Request;
use poem_openapi::{  OpenApi, payload::Json, Object };
use anyhow::Result;
use futures::executor::block_on;
use sqlx::Row;

use crate::utils::db_statements::SELECT_PUBKEY;
use crate::utils::db_types::PubkeyRetrieval;
use crate::utils::{tms_utils, tms_utils::RequestDebug};
use crate::RUNTIME_CTX;

// ***************************************************************************
//                          Request/Response Definiions
// ***************************************************************************
pub struct PublicKeyApi;

#[derive(Object)]
struct ReqPublicKey
{
    user: String,
    user_uid: Option<String>,
    host: String,
    public_key_fingerprint: String, // protocol:base64hash format
    key_type: Option<String>,       // RSA, ECDSA, ED25519
}

#[derive(Object)]
struct RespPublicKey
{
    result_code: String,
    result_msg: String,
    public_key: String,
}

// Implement the debug record trait for logging.
impl RequestDebug for ReqPublicKey {   
    type Req = ReqPublicKey;
    fn get_request_info(&self) -> String {
        let mut s = String::with_capacity(255);
        s.push_str("  Request body:");
        s.push_str("\n    user: ");
        s.push_str(&self.user);
        s.push_str("\n    user_uid: ");
        let uid = match &self.user_uid {
            Some(k) => k,
            None => "None",
        };
        s.push_str(uid);
        s.push_str("\n    host: ");
        s.push_str(&self.host);
        s.push_str("\n    public_key_fingerprint: ");
        s.push_str(&self.public_key_fingerprint);
        s.push_str("\n    key_type: ");
        let kt = match &self.key_type {
            Some(k) => k,
            None => "None",
        };
        s.push_str(kt);
        s.push('\n');
        s
    }
}

// ***************************************************************************
//                             OpenAPI Endpoint
// ***************************************************************************
#[OpenApi]
impl PublicKeyApi {
    #[oai(path = "/tms/pubkeys/creds/retrieve", method = "post")]
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
        tms_utils::debug_request(http_req, req);

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
    // Get a connection to the db and start a transaction.  Uncommited transactions 
    // are automatically rolled back when they go out of scope. 
    // See https://docs.rs/sqlx/latest/sqlx/struct.Transaction.html.
    let mut tx = RUNTIME_CTX.db.begin().await?;
    
    // Create the insert statement.
    let result = sqlx::query(SELECT_PUBKEY)
        .bind(&req.user)
        .bind(&req.host)
        .bind(&req.public_key_fingerprint)
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
