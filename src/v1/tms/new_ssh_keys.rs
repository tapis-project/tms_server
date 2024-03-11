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
    private_key: String,
}

// ***************************************************************************
//                             OpenAPI Endpoint
// ***************************************************************************
#[OpenApi]
impl NewSshKeysApi {
    #[oai(path = "/tms/NewSshKeys", method = "post")]
    async fn get_new_ssh_keys(&self, keys: Json<ReqNewSshKeys>) -> Json<RespNewSshKeys> {
        let resp = match RespNewSshKeys::process(&keys) {
            Ok(r) => r,
            Err(e) => RespNewSshKeys::new("ERROR"),
        };

        Json(resp)
    }
}

// ***************************************************************************
//                          Request/Response Methods
// ***************************************************************************
impl RespNewSshKeys {
    fn new(key: &str) -> Self {
        Self {private_key: key.to_string()}
    }

    fn process(req: &ReqNewSshKeys) -> Result<RespNewSshKeys, Error> {
        Ok(Self::new("PRIVATE_KEY"))
    }
}