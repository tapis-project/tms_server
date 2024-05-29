#![forbid(unsafe_code)]

use anyhow::{Result, anyhow};
use log::{info, error};
use serde::Deserialize;
use std::{env, fs, path::Path};
use toml;
use fs_mistrust::Mistrust;
use std::os::unix::fs::PermissionsExt;
use lazy_static::lazy_static;
use structopt::StructOpt;

// See https://users.rust-lang.org/t/relationship-between-std-futures-futures-and-tokio/38077
// for a cogent explanation on dealing with futures and async programming in Rust.  More 
// background can be found at https://rust-lang.github.io/async-book/.
use sqlx::{Sqlite, Pool};
use futures::executor::block_on;

// TMS Utilities
use crate::utils::{tms_utils, db_init, errors::Errors};

use super::tms_utils::get_absolute_path;

// ***************************************************************************
//                                Constants
// ***************************************************************************
// Directory and file locations. Unless otherwise noted, all files and directories
// are relative to the root directory.
const ENV_TMS_ROOT_DIR     : &str = "TMS_ROOT_DIR";
const DEFAULT_ROOT_DIR     : &str = "~/.tms";
const MIGRATIONS_DIR       : &str = "/migrations";
const CONFIG_DIR           : &str = "/config";
const LOGS_DIR             : &str = "/logs";
const DATABASE_DIR         : &str = "/database";
const CERTS_DIR            : &str = "/certs";
const LOG4RS_CONFIG_FILE   : &str = "/log4rs.yml"; // relative to config dir
const TMS_CONFIG_FILE      : &str = "/tms.toml";   // relative to config dir

// Netorking.
const DEFAULT_HTTP_ADDR    : &str = "https://localhost";
const DEFAULT_HTTP_PORT    : u16  = 3000;

// Tenants used in all installations.
pub const DEFAULT_TENANT   : &str = "default";
pub const TEST_TENANT      : &str = "test";

// Database constants.
pub const SQLITE_TRUE      : i32 = 1;
#[allow(dead_code)]
pub const SQLITE_FALSE     : i32 = 0;

// ***************************************************************************
//                             Static Variables
// ***************************************************************************
// Assign the command line arguments BEFORE RUNTIME_CTX is initialized in main.
lazy_static! {
    pub static ref TMS_ARGS: TmsArgs = init_tms_args();
}

// Calculate the data directories BEFORE RUNTIME_CTX is initialized in main.
lazy_static! {
    pub static ref TMS_DIRS: TmsDirs = init_tms_dirs();
}

// ***************************************************************************
//                             Directory Structs
// ***************************************************************************
// ---------------------------------------------------------------------------
// TmsDirs:
// ---------------------------------------------------------------------------
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct TmsDirs {
    pub root_dir: String,
    pub migrations_dir: String,
    pub config_dir: String,
    pub logs_dir: String,
    pub database_dir: String,
    pub certs_dir: String,
}

// ***************************************************************************
//                               Config Structs
// ***************************************************************************
// ---------------------------------------------------------------------------
// CommandLineArgs:
// ---------------------------------------------------------------------------
#[derive(Debug, StructOpt)]
#[structopt(name = "tms_args", about = "Command line arguments for TMS Server.")]
pub struct TmsArgs {
    /// Specify TMS's root data directory.
    /// 
    /// This directory contains all the files TMS uses during execution.
    #[structopt(short, long)]
    pub root_dir: Option<String>,

    /// Create the data directories and then exit.
    /// 
    /// The data directories will be rooted at a root directory calculated 
    /// using the following priority order:
    /// 
    ///   1. If set, the value of the TMS_ROOT_DIR environment,
    /// 
    ///   2. Otherwise, if set, the value of the --root_dir command line argument,
    /// 
    ///   3. Otherwise, ~/.tms
    /// 
    #[structopt(short, long)]
    pub create_dirs_only: bool,
}

// ---------------------------------------------------------------------------
// Parms:
// ---------------------------------------------------------------------------
#[derive(Debug)]
#[allow(dead_code)]
pub struct Parms {
    pub config_file: String,
    pub config: Config,
}

// ---------------------------------------------------------------------------
// RuntimeCtx:
// ---------------------------------------------------------------------------
#[derive(Debug)]
#[allow(dead_code)]
pub struct RuntimeCtx {
    pub parms: Parms,
    pub db: Pool<Sqlite>,
    pub tms_args: &'static TmsArgs,
    pub tms_dirs: &'static TmsDirs,
}

// ---------------------------------------------------------------------------
// Config:
// ---------------------------------------------------------------------------
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
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
//                            Directory Functions
// ***************************************************************************
// ---------------------------------------------------------------------------
// init_tms_args:
// ---------------------------------------------------------------------------
/** Get the command line arguments. */
fn init_tms_args() -> TmsArgs {
    let args = TmsArgs::from_args();
    println!("{:?}", args);
    args
}

// ---------------------------------------------------------------------------
// init_tms_dirs:
// ---------------------------------------------------------------------------
/** Calculate the external data directories. */
fn init_tms_dirs() -> TmsDirs {
    // Initialize the mistrust object.
    let mistrust = get_mistrust();

    // Check that each path is absolute and is a directory with the
    // proper permission assign if it exists.  If it doesn't exist,
    // create it.
    let root_dir = get_root_dir();
    check_tms_dir(&root_dir, "root directory", &mistrust);

    let migrations_dir = root_dir.clone() + MIGRATIONS_DIR;
    check_tms_dir(&migrations_dir, "resources directory", &mistrust);
    
    let config_dir = root_dir.clone() + CONFIG_DIR;
    check_tms_dir(&config_dir, "config directory", &mistrust);
    
    let logs_dir = root_dir.clone() + LOGS_DIR;
    check_tms_dir(&logs_dir, "logs directory", &mistrust);
    
    let database_dir = root_dir.clone() + DATABASE_DIR;
    check_tms_dir(&database_dir, "database directory", &mistrust);

    let certs_dir = root_dir.clone() + CERTS_DIR;
    check_tms_dir(&certs_dir, "certs directory", &mistrust);
    
    // Package up and return the directories.
    TmsDirs {
        root_dir, migrations_dir, config_dir, logs_dir, database_dir, certs_dir,
    }
}

// ---------------------------------------------------------------------------
// check_tms_dir:
// ---------------------------------------------------------------------------
/** Check that the path is absolute and, if it exists, that is has the proper
 * permissions assigned.  If it doesn't exist, create it.  The mistrust package
 * creates directories with 0o700 permissions.  
 * 
 * Any failure results in a panic. 
 */
fn check_tms_dir(dir: &String, msgname: &str, mistrust: &Mistrust ) {
    // Get the path object.
    let path = Path::new(dir);
    if !path.is_absolute() {
        panic!("The TMS {} path must be absolute: {}", msgname, dir);
    }
    if path.exists() {
        // Make sure the path represents a directory.
        if !path.is_dir() {
            panic!("The TMS {} path must be a directory: {}", msgname, dir);
        }

        // Make sure the directory had rwx for owner only.
        let meta = path.metadata().unwrap_or_else(|_| panic!("Unable to read metadata for {}: {}", msgname, dir));
        let perm = meta.permissions().mode();
        if perm & 0o777 != 0o700 {
            panic!("The TMS {} path must be have 0o700 permissions: {}", msgname, dir);
        }
    } else {
        // Create the directory with the correct permissions.
        match mistrust.make_directory(path) {
            Ok(_) => (),
            Err(e) => {
                panic!("Make directory error for {:?}: {}", path, &e.to_string());
            }
        }
    }
}

// ---------------------------------------------------------------------------
// get_mistrust:
// ---------------------------------------------------------------------------
/** Configure a new mistrust object for initial directory processing. */
fn get_mistrust() -> Mistrust {
    // Configure our mistrust object.
    let mistrust = match Mistrust::builder() 
        .ignore_prefix(get_absolute_path("~"))
        .trust_group(0)
        .build() {
            Ok(m) => m,
            Err(e) => {
                panic!("Mistrust configuration error: {}", &e.to_string());
            }
        };
    mistrust
}

// ---------------------------------------------------------------------------
// get_root_dir:
// ---------------------------------------------------------------------------
fn get_root_dir() -> String {
    // Order of precedence:
    //  1. Environment variable
    //  2. Command line --root-dir argument
    //  3. Default location
    //
    let root_dir = env::var(ENV_TMS_ROOT_DIR).unwrap_or_else(
        |_| {
            match TMS_ARGS.root_dir.clone() {
                Some(r) => r,
                None => DEFAULT_ROOT_DIR.to_string(),
            }
        });

    // Canonicalize the path.
    get_absolute_path(&root_dir)    
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
    info!("Log4rs initialized using: {}", logconfig);
}

// ---------------------------------------------------------------------------
// init_log_config:
// ---------------------------------------------------------------------------
fn init_log_config() -> String {
    TMS_DIRS.config_dir.clone() + LOG4RS_CONFIG_FILE 
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
    // Get the config file path from its data directory.
    let config_file = TMS_DIRS.config_dir.clone() + TMS_CONFIG_FILE;
    
    // Read the cofiguration file.
    let config_file_abs = tms_utils::get_absolute_path(&config_file);
    info!("{}", Errors::ReadingConfigFile(config_file_abs.clone()));
    let contents = match fs::read_to_string(&config_file_abs) {
        Ok(c) => c,
        Err(_) => {
            println!("Unable to read configuration at {}. Using default values.", config_file);
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
    let db = block_on(db_init::init_db());
    RuntimeCtx {parms, db, tms_args: &TMS_ARGS, tms_dirs: &TMS_DIRS}
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

