#![forbid(unsafe_code)]

use poem_openapi::{  OpenApi, payload::Json, Object };
use poem::Error;

// From cargo.toml.
const TMS_VERSION: Option<&str> = option_env!("CARGO_PKG_VERSION");

// ***************************************************************************
//                          Request/Response Definiions
// ***************************************************************************
pub struct VersionApi;

#[derive(Object)]
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

// ***************************************************************************
//                             OpenAPI Endpoint
// ***************************************************************************
#[OpenApi]
impl VersionApi {
    #[oai(path = "/tms/version", method = "get")]
    async fn get_version(&self) -> Json<RespVersion> {
        let resp = match RespVersion::process() {
            Ok(r) => r,
            Err(e) => {
                let msg = "ERROR: ".to_owned() + e.to_string().as_str();
                RespVersion::new("1", msg.as_str(), "", "", "", "", "", "",)},
        };

        Json(resp)
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

    fn process() -> Result<RespVersion, Error> {
        Ok(Self::new("0", 
                    "success",
                    TMS_VERSION.unwrap_or("unknown"),
                    env!("GIT_BRANCH"),
                    env!("GIT_COMMIT_SHORT"),
                    env!("GIT_DIRTY"),
                    env!("SOURCE_TIMESTAMP"),
                    env!("RUSTC_VERSION")),
        )
    }
}
