#![forbid(unsafe_code)]

use poem_openapi::{ OpenApi, payload::Json, Object };
use poem::Error;

// ***************************************************************************
//                          Request/Response Definiions
// ***************************************************************************
pub struct NewSshKeysApi;

#[derive(Object)]
struct ReqNewSshKeys
{
    client_id: String,
    client_secret: String,
    host: String,
    user: String,
    num_uses: u32,     // 0 means unlimited
    ttl_minutes: u32,  // 0 means unlimited
}

#[derive(Object)]
struct RespNewSshKeys
{
    result_code: String,
    result_msg: String,
    private_key: String,
    public_key: String,
}

// ***************************************************************************
//                             OpenAPI Endpoint
// ***************************************************************************
#[OpenApi]
impl NewSshKeysApi {
    #[oai(path = "/tms/creds/sshkeys", method = "post")]
    async fn get_new_ssh_keys(&self, req: Json<ReqNewSshKeys>) -> Json<RespNewSshKeys> {
        let resp = match RespNewSshKeys::process(&req) {
            Ok(r) => r,
            Err(e) => {
                let msg = "ERROR: ".to_owned() + e.to_string().as_str();
                RespNewSshKeys::new("1", msg.as_str(), "", "")},
        };

        Json(resp)
    }
}

// ***************************************************************************
//                          Request/Response Methods
// ***************************************************************************
impl RespNewSshKeys {
    fn new(result_code: &str, result_msg: &str, private_key: &str, public_key: &str) -> Self {
        Self {result_code: result_code.to_string(), 
              result_msg: result_msg.to_string(), 
              private_key: private_key.to_string(), 
              public_key: public_key.to_string()}
    }

    fn process(req: &ReqNewSshKeys) -> Result<RespNewSshKeys, Error> {
        Ok(Self::new("0", "success", "PRIVATE_KEY", "PUBLIC_KEY"))
    }
}