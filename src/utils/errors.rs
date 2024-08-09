#![forbid(unsafe_code)]

use thiserror::Error;
use poem_openapi::Object;

/// Error enumerates the errors returned by this application.
#[derive(Error, Debug)]
pub enum Errors {
    /// Input parameter logging.
    #[error("tms_server input parameters:\n{}", .0)]
    InputParms(String),

    /// Represents all other cases of `std::io::Error`.
    #[error(transparent)]
    IOError(#[from] std::io::Error),

    /// Inaccessible logger configuration file.
    #[error("Unable to access the Log4rs configuration file: {}", .0)]
    Log4rsInitialization(String),

    #[error("Reading application configuration file: {}", .0)]
    ReadingConfigFile(String),

    #[error("Unable to parse TOML file: {}", .0)]
    TOMLParseError(String),

    #[error("TMS Error: {}", .0)]
    TMSError(String),
}

#[derive(PartialEq, Clone, Debug, Object)]
pub struct HttpResult {
    pub result_code: String,
    pub result_msg: String,
}
impl HttpResult {
    pub fn new(result_code: String, result_msg: String) -> Self {
        HttpResult {result_code, result_msg}
    }
}
