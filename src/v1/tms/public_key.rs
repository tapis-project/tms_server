use poem_openapi::{ OpenApi, payload::Json, Object };
use poem::Error;

// ***************************************************************************
//                          Request/Response Definiions
// ***************************************************************************
pub struct PublicKeyApi;

#[derive(Object)]
struct ReqPublicKey
{
    client_addr: String,
    client_port: i32,
    server_addr: String,
    server_port: i32,
    user: String,
}

#[derive(Object)]
struct RespPublicKey
{
    public_key: String,
}

// ***************************************************************************
//                             OpenAPI Endpoint
// ***************************************************************************
#[OpenApi]
impl PublicKeyApi {
    #[oai(path = "/tms/PublicKey", method = "post")]
    async fn get_new_ssh_keys(&self, keys: Json<ReqPublicKey>) -> Json<RespPublicKey> {
        let resp = match RespPublicKey::process(&keys) {
            Ok(r) => r,
            Err(e) => RespPublicKey::new("ERROR"),
        };

        Json(resp)
    }
}

// ***************************************************************************
//                          Request/Response Methods
// ***************************************************************************
impl RespPublicKey {
    fn new(key: &str) -> Self {
        Self {public_key: key.to_string()}
    }

    fn process(req: &ReqPublicKey) -> Result<RespPublicKey, Error> {
        Ok(Self::new("PUBLIC_KEY"))
    }
}