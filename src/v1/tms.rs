use poem_openapi::{ OpenApi, payload::Json, Object };
use poem::Error;

pub struct TMSApi;

#[derive(Object)]
struct ReqNewSSHKeys
{
    client_id: String,
    client_secret: String,
    host: String,
    user: String,
}

#[derive(Object)]
struct RespNewSSHKeys
{
    private_key: String,
}

#[OpenApi]
impl TMSApi {
    
    #[oai(path = "/tms/new_ssh_keys", method = "post")]
    async fn get_new_ssh_keys(&self, keys: Json<ReqNewSSHKeys>) -> Json<RespNewSSHKeys> {
        // keys.0.client_id = "CLIENT_ID".to_string();
        // keys.0.client_secret = "CLIENT_SECRET".to_string();
        // keys.0.user = "Bud".to_string();
        let resp = match RespNewSSHKeys::process(&keys) {
            Ok(r) => r,
            Err(e) => RespNewSSHKeys::new("ERROR"),
        };

        Json(resp)
    }
}

impl RespNewSSHKeys {
    fn new(key: &str) -> Self {
        Self {private_key: key.to_string()}
    }

    fn process(req: &ReqNewSSHKeys) -> Result<RespNewSSHKeys, Error> {
        Ok(Self::new("PRIVATE_KEY"))
    }
}