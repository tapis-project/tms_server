#![forbid(unsafe_code)]

//use ssh_key::private::{ KeypairData, PrivateKey, RsaKeypair };
use poem_openapi::{ OpenApi, payload::Json, Object };
use poem::Error;
use anyhow::anyhow;

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
    public_key_fingerprint: String,
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
                RespNewSshKeys::new("1", msg.as_str(), "".to_string(), "".to_string(), "".to_string())},
        };

        Json(resp)
    }
}

// ***************************************************************************
//                          Request/Response Methods
// ***************************************************************************
impl RespNewSshKeys {
    fn new(result_code: &str, result_msg: &str, private_key: String, public_key: String, 
           public_key_fingerprint: String) -> Self {
        Self {result_code: result_code.to_string(), 
              result_msg: result_msg.to_string(), 
              private_key, public_key, public_key_fingerprint,
            }
    }

    fn process(req: &ReqNewSshKeys) -> Result<RespNewSshKeys, anyhow::Error> {
        // Generate the new key pair.
        let keyinfo = match keygen::generate_key(keygen::KeyType::Rsa) {
            Ok(k) => k,
            Err(e) => {
                return Result::Err(anyhow!(e));
            }
        };
        
        // Success!
        Ok(Self::new("0", "success", 
                    keyinfo.private_key, 
                    keyinfo.public_key, 
                    keyinfo.public_key_fingerprint))
    }
}