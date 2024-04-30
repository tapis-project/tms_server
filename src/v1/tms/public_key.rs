#![forbid(unsafe_code)]

use poem_openapi::{  OpenApi, payload::Json, Object };
use poem::Error;

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
    public_key_fingerprint: String,
    key_type: String,
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
    async fn get_public_key(&self, req: Json<ReqPublicKey>) -> Json<RespPublicKey> {
        let resp = match RespPublicKey::process(&req) {
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
// impl ReqPublicKey {
//     fn new(client_addr: String, client_port: i32, server_addr: String, 
//            server_port: i32, user: String) -> ReqPublicKey {
//                 ReqPublicKey {client_addr, client_port, server_addr, server_port, user}
//            }
// }

impl RespPublicKey {
    fn new(result_code: &str, result_msg: &str, key: &str) -> Self {
        Self {result_code: result_code.to_string(), 
              result_msg: result_msg.to_string(), 
              public_key: key.to_string()}
    }

    fn process(_req: &ReqPublicKey) -> Result<RespPublicKey, Error> {
        Ok(Self::new("0", "success", "PUBLIC_KEY"))
    }
}