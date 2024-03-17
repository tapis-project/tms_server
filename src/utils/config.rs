use anyhow::{Result, anyhow};
use log::{info, error};
use serde::Deserialize;
use std::{env, fs};
use toml;

// TMS Utilities
use crate::utils::{tms_utils, errors::Errors};

// ***************************************************************************
//                                Constants
// ***************************************************************************
// Constants.
const ENV_LOG4RS_FILE_KEY : &str = "TMS_LOG4RS_CONFIG_FILE";
const LOG4RS_CONFIG_FILE  : &str = "resources/log4rs.yml";
const ENV_CONFIG_FILE_KEY : &str = "TMS_CONFIG_FILE";
const DEFAULT_CONFIG_FILE : &str = "~/tms.toml";
const DEFAULT_HTTP_ADDR   : &str = "https://localhost";
const DEFAULT_HTTP_PORT   : u16  = 3000;

// ***************************************************************************
//                               Config Structs
// ***************************************************************************
// ---------------------------------------------------------------------------
// Parms:
// ---------------------------------------------------------------------------
#[derive(Debug)]
pub struct Parms {
    pub config_file: String,
    pub config: Config,
}

// ---------------------------------------------------------------------------
// RuntimeCtx:
// ---------------------------------------------------------------------------
#[derive(Debug)]
pub struct RuntimeCtx {
    pub parms: Parms,
}

// ---------------------------------------------------------------------------
// Config:
// ---------------------------------------------------------------------------
#[derive(Debug, Deserialize)]
pub struct Config {
    pub title: String,
    pub http_addr: String,
    pub http_port: u16,
}

impl Config {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Config::default()
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            title: "TMS Server".to_string(),
            http_addr: DEFAULT_HTTP_ADDR.to_string(),
            http_port: DEFAULT_HTTP_PORT,
        }
    }
}

// ***************************************************************************
//                               Log Functions
// ***************************************************************************
// ---------------------------------------------------------------------------
// init_log:
// ---------------------------------------------------------------------------
pub fn init_log() {
    // Initialize log4rs logging.
    let logconfig = init_log_config();
    match log4rs::init_file(logconfig.clone(), Default::default()) {
        Ok(_) => (),
        Err(e) => {
            println!("{}", e);
            let s = format!("{}", Errors::Log4rsInitialization(logconfig));
            panic!("{}", s);
        },
    }
    info!("{} {}", "Log4rs initialized using:", logconfig);
}

// ---------------------------------------------------------------------------
// init_log_config:
// ---------------------------------------------------------------------------
fn init_log_config() -> String {
    env::var(ENV_LOG4RS_FILE_KEY).unwrap_or_else(|_| LOG4RS_CONFIG_FILE.to_string())
}

/// ***************************************************************************
//                             Parms Functions
// ***************************************************************************
// ---------------------------------------------------------------------------
// get_parms:
// ---------------------------------------------------------------------------
/** Retrieve the application parameters from the configuration file specified
 * either through an environment variable or as the first (and only) command
 * line argument.  If neither are provided, an attempt is made to use the
 * default file path.
 */
fn get_parms() -> Result<Parms> {
    // Get the config file path from the environment, command line or default.
    let config_file = env::var(ENV_CONFIG_FILE_KEY).unwrap_or_else(
        |_| {
            // Get the config file pathname as the first command line
            // parameter or use the default path.
            match env::args().nth(1) {
                Some(f) => f,
                None => DEFAULT_CONFIG_FILE.to_string()
            }
        });

    // Read the cofiguration file.
    let config_file_abs = tms_utils::get_absolute_path(&config_file);
    info!("{}", Errors::ReadingConfigFile(config_file_abs.clone()));
    let contents = match fs::read_to_string(&config_file_abs) {
        Ok(c) => c,
        Err(_) => {
            println!("{} {}{}", "Unable to read configuration at", config_file, ". Using default values.");
            return Ok(Parms { config_file: Default::default(), config: Config::new() });
        }
    };

    // Parse the toml configuration.
    let config : Config = match toml::from_str(&contents) {
        Ok(c)  => c,
        Err(e) => {
            let msg = format!("{}\n   {}", Errors::TOMLParseError(config_file_abs), e);
            error!("{}", msg);
            return Result::Err(anyhow!(msg));
        }
    };

    Ok(Parms { config_file: config_file_abs, config })
}

// ***************************************************************************
//                             Config Functions
// ***************************************************************************
// ---------------------------------------------------------------------------
// init_runtime_context:
// ---------------------------------------------------------------------------
pub fn init_runtime_context() -> RuntimeCtx {
    // If either of these fail the application aborts.
    let parms = get_parms().expect("FAILED to read configuration file.");

    // Return the context.
    RuntimeCtx {parms}
}

// ***************************************************************************
//                                  Tests
// ***************************************************************************
#[cfg(test)]
mod tests {
    use crate::utils::config::Config;

    #[test]
    fn here_i_am() {
        println!("file test: main.rs");
    }

    #[test]
    fn print_config() {
        println!("{:?}", Config::new());
    }
}

