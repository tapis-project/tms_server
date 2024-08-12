#![forbid(unsafe_code)]

use poem_openapi::{  OpenApi, payload::Json, Object, ApiResponse };
use poem::Error;

use crate::utils::errors::HttpResult;
use log::error;

// From cargo.toml.
const TMS_VERSION: Option<&str> = option_env!("CARGO_PKG_VERSION");

// ***************************************************************************
//                          Request/Response Definiions
// ***************************************************************************
pub struct VersionApi;

#[derive(Object, Debug)]
struct RespVersion
{
    result_code: String,
    result_msg: String,
    tms_version: String,
    git_branch: String,
    git_commit: String,
    git_dirty: String,
    source_ts: String,
    rustc_version: String,
}

// ------------------- HTTP Status Codes -------------------
#[derive(Debug, ApiResponse)]
enum TmsResponse {
    #[oai(status = 200)]
    Http200(Json<RespVersion>),
    #[oai(status = 500)]
    Http500(Json<HttpResult>),
}

fn make_http_200(resp: RespVersion) -> TmsResponse {
    TmsResponse::Http200(Json(resp))
}
fn make_http_500(msg: String) -> TmsResponse {
    TmsResponse::Http500(Json(HttpResult::new(500.to_string(), msg)))    
}

// ***************************************************************************
//                             OpenAPI Endpoint
// ***************************************************************************
#[OpenApi]
impl VersionApi {
    #[oai(path = "/tms/version", method = "get")]
    async fn get_version(&self) -> TmsResponse {
        match RespVersion::process() {
            Ok(r) => r,
            Err(e) => {
                let msg = "ERROR: ".to_owned() + e.to_string().as_str();
                error!("{}", msg);
                make_http_500(msg)
            }
        }
    }
}

// ***************************************************************************
//                          Request/Response Methods
// ***************************************************************************
impl RespVersion {
    #[allow(clippy::too_many_arguments)]
    fn new(result_code: &str, result_msg: &str, tms: &str, branch: &str, commit: &str, dirty: &str, ts: &str, rustc: &str)
    -> Self {
        Self {result_code: result_code.to_string(), 
              result_msg: result_msg.to_string(),
              tms_version: tms.to_string(),
              git_branch: branch.to_string(),
              git_commit: commit.to_string(),
              git_dirty:  dirty.to_string(),
              source_ts: ts.to_string(),
              rustc_version: rustc.to_string(),  
        }
    }

    fn process() -> Result<TmsResponse, Error> {
        Ok(make_http_200(Self::new("0", 
                    "success",
                    TMS_VERSION.unwrap_or("unknown"),
                    env!("GIT_BRANCH"),
                    env!("GIT_COMMIT_SHORT"),
                    env!("GIT_DIRTY"),
                    env!("SOURCE_TIMESTAMP"),
                    env!("RUSTC_VERSION"))),
        )
    }
}
