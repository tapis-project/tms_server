use poem_openapi::{ param::Query, OpenApi, payload::Json, Object };
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
    #[oai(path = "/tms/creds/publickey", method = "get")]
    async fn get_new_ssh_keys(&self, client_addr: Query<String>,
                              client_port: Query<i32>,
                              server_addr: Query<String>,
                              server_port: Query<i32>,
                              user: Query<String>,
                             ) -> Json<RespPublicKey> {
        let req = ReqPublicKey::new(client_addr.0, client_port.0, server_addr.0, server_port.0, user.0);                      
        let resp = match RespPublicKey::process(&req) {
            Ok(r) => r,
            Err(e) => {
                let msg = "ERROR: ".to_owned() + e.to_string().as_str();
                RespPublicKey::new(msg.as_str())},
        };

        Json(resp)
    }
}

// ***************************************************************************
//                          Request/Response Methods
// ***************************************************************************
impl ReqPublicKey {
    fn new(client_addr: String, client_port: i32, server_addr: String, 
           server_port: i32, user: String) -> ReqPublicKey {
                ReqPublicKey {client_addr, client_port, server_addr, server_port, user}
           }
}

impl RespPublicKey {
    fn new(key: &str) -> Self {
        Self {public_key: key.to_string()}
    }

    fn process(req: &ReqPublicKey) -> Result<RespPublicKey, Error> {
        Ok(Self::new("PUBLIC_KEY"))
    }
}