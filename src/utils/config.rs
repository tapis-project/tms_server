#![forbid(unsafe_code)]

use anyhow::{Result, anyhow};
use log::{info, error};
use serde::Deserialize;
use std::{env, fs::{self, Permissions}, path::Path};
use std::collections::HashMap;
use toml;
use fs_mistrust::Mistrust;
use std::os::unix::fs::PermissionsExt;
use lazy_static::lazy_static;
use tera::Tera;
use structopt::StructOpt;
use users::get_effective_uid;

// See https://users.rust-lang.org/t/relationship-between-std-futures-futures-and-tokio/38077
// for a cogent explanation on dealing with futures and async programming in Rust.  More 
// background can be found at https://rust-lang.github.io/async-book/.
//use sqlx::{Sqlite, Pool};
use sqlx::{Pool, Postgres};
use futures::executor::block_on;

// TMS Utilities
use crate::utils::{tms_utils, db_init, errors::Errors};
use super::db_statements::{GET_CLIENT_SECRET, GET_ADMIN_SECRET};
use super::authz::{AuthzTypes, X_TMS_ADMIN_ID, X_TMS_ADMIN_SECRET, X_TMS_CLIENT_ID, X_TMS_CLIENT_SECRET};

use super::tms_utils::get_absolute_path;

// ***************************************************************************
//                                Constants
// ***************************************************************************
// Directory and file locations. Unless otherwise noted, all files and directories
// are relative to the TMS root directory.
const ENV_TMS_ROOT_DIR     : &str = "TMS_ROOT_DIR";
const DEFAULT_ROOT_DIR     : &str = "~/.tms";
const MIGRATIONS_DIR       : &str = "/migrations";
const CONFIG_DIR           : &str = "/config";
const LOGS_DIR             : &str = "/logs";
const DATABASE_DIR         : &str = "/database";
const CERTS_DIR            : &str = "/certs";
const RESOURCES_DIR        : &str = "./resources"; // relative to currnent dir

const LOG4RS_CONFIG_FILE   : &str = "/log4rs.yml"; // relative to config dir
const TMS_CONFIG_FILE      : &str = "/tms.toml";   // relative to config dir
const CERT_PEM_FILE        : &str = "/cert.pem";   // relative to certs dir
const KEY_PEM_FILE         : &str = "/key.pem";    // relative to certs dir

// Networking.
const DEFAULT_HTTP_ADDR    : &str = "https://localhost";
const DEFAULT_HTTP_PORT    : u16  = 3000;
const DEFAULT_SVR_URL      : &str = "https://localhost:3000/v1";

// Tenants used in all installations.
pub const DEFAULT_TENANT   : &str = "default";
pub const TEST_TENANT      : &str = "test";

// Admin table constants.
#[allow(dead_code)]
pub const ADMIN_ID_PREFIX  : &str = "~~"; // literal value repeated in next line
pub const DEFAULT_ADMIN_ID : &str = concat!("~~", "admin"); // admin ids always start with prefix
pub const PERM_ADMIN       : &str = "PERM_ADMIN";

// New client creation is allowed by default.
pub const NEW_CLIENTS_ALLOW: &str = "allow";
pub const NEW_CLIENTS_DISALLOW: &str = "disallow";
pub const NEW_CLIENTS_ON_APPROVAL: &str = "on_approval";
pub const DEFAULT_NEW_CLIENTS: &str = NEW_CLIENTS_ALLOW;

// Database constants.
pub const SQLITE_TRUE      : i64 = 1;
#[allow(dead_code)]
pub const SQLITE_FALSE     : i64 = 0;

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

// Initialize the authz parameter sets.
lazy_static! {
    pub static ref AUTHZ_ARGS: AuthzArgs = init_authz_args();
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

    /// Create the data directories and initial database records and then exit.
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
    pub install: bool,
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
// AuthzArgs:
// ---------------------------------------------------------------------------
// Define the parameter specs used for the different types of authz validation.
// See authz.rs for details.
#[derive(Debug)]
pub struct AuthzSpec<'a>{
    pub id: &'a str,           // HTTP header for subject being authorized
    pub secret: &'a str,       // HTTP header for secret used to authorize
    pub display_name: &'a str, // User friendly name of subject being authorized
    pub sql_query: &'a str,    // SQL query with required signature for secret retrieval 
}

#[derive(Debug)]
pub struct AuthzArgs {
    pub specs: HashMap<AuthzTypes, AuthzSpec<'static>>,
}

// ---------------------------------------------------------------------------
// RuntimeCtx:
// ---------------------------------------------------------------------------
#[derive(Debug)]
#[allow(dead_code)]
pub struct RuntimeCtx {
    pub parms: Parms,
    pub db: Pool<Postgres>,
    pub authz: &'static AuthzArgs,
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
    pub enable_mvp: bool,
    pub enable_test_tenant: bool,
    pub new_clients: String,
    pub server_urls: Vec<String>,
}

impl Config {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Config::default()
    }

    // Validation beyond type checking.
    fn validate(&self) -> Result<()> {
        match self.new_clients.as_str() {
            NEW_CLIENTS_ALLOW => Ok(()),
            NEW_CLIENTS_DISALLOW => Ok(()),
            NEW_CLIENTS_ON_APPROVAL =>  {
                let msg = "The new_clients 'on_approval' setting is not implemented yet.";
                error!("{}", msg);
                Result::Err(anyhow!(msg))
            },           
            other => {
                let msg = format!("Invalid value '{}' assigned to the new_clients configuration setting.  \
                                          Currently supported values are: 'allow', 'disallow'.", other);
                error!("{}", msg);
                Result::Err(anyhow!(msg))
            },
        } 
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            title: "TMS Server".to_string(),
            http_addr: DEFAULT_HTTP_ADDR.to_string(),
            http_port: DEFAULT_HTTP_PORT,
            enable_mvp: false,
            enable_test_tenant: false,
            new_clients: DEFAULT_NEW_CLIENTS.to_string(),
            server_urls: vec![DEFAULT_SVR_URL.to_string()],
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
// prohibit_root_user:
// ---------------------------------------------------------------------------
/** This function makes a reasonable attempt to stop execution if we are running
 * as root.  It's not meant to be foolproof, just likely to catch inadvertent, 
 * high-privilege executions before they can cause trouble.
 */
pub fn prohibit_root_user() {
    // Get the effective user ID.
    let uid = get_effective_uid();
    if uid == 0 {
        let msg = 
            format!("\n***********************************************************************\n\
                    ERROR: This program should not execute under UID 0 (root). \n\n\
                    Please restart as a non-privileged user.\n\
                    ***********************************************************************\n");
        panic!("{}", msg);
    }
}

// ---------------------------------------------------------------------------
// check_prior_installation:
// ---------------------------------------------------------------------------
/** Panic if we are trying to run the server before an installation run. */
pub fn check_prior_installation() {
    let rootdir = get_root_dir();
    let path = Path::new(&rootdir);
    if path.is_file() {
        // No directory found.
        let msg = 
            format!("\n***********************************************************************\n\
                    ERROR: Expected the TMS root directory but found a file at {}. \n\n\
                    Please correct the path and try again.\n\
                    ***********************************************************************\n", rootdir);
        panic!("{}", msg);
    }
    else if !path.is_dir() && !TMS_ARGS.install {
        // No directory found.
        let msg = 
            format!("\n***********************************************************************\n\
                    ERROR: Expected the TMS root directory to exist at {}. \n\n\
                    Please run 'tms_server --install' to install TMS's root directory \n\
                    in it's default location or consult the README file for configuring \n\
                    a non-default root directory location.\n\
                    ***********************************************************************\n", rootdir);
        panic!("{}", msg);
    } else if path.is_dir() && TMS_ARGS.install {
        // Root directory already exists.
        let msg = 
            format!("\n***********************************************************************\n\
                    ERROR: Cannot install over existing TMS root directory at {}. \n\n\
                    Please run tms_server without the --install option.\n\
                    ***********************************************************************\n", rootdir);
        panic!("{}", msg);
    }
}

// ---------------------------------------------------------------------------
// init_tms_dirs:
// ---------------------------------------------------------------------------
/** Calculate the external data directories. */
fn init_tms_dirs() -> TmsDirs {
    // Initialize the mistrust object.
    let mistrust = get_mistrust();

    // Declare directory create flag to control file copying.
    let mut dir_created;

    // Check that each path is absolute and is a directory with the
    // proper permission assign if it exists.  If it doesn't exist,
    // create it.
    let root_dir = get_root_dir();
    check_tms_dir(&root_dir, "root directory", &mistrust);

    let migrations_dir = root_dir.clone() + MIGRATIONS_DIR;
    dir_created = check_tms_dir(&migrations_dir, "migrations directory", &mistrust);
    if dir_created {copy_resource_files(&migrations_dir, MIGRATIONS_DIR, &root_dir);}
    
    let config_dir = root_dir.clone() + CONFIG_DIR;
    dir_created = check_tms_dir(&config_dir, "config directory", &mistrust);
    if dir_created {copy_resource_files(&config_dir, CONFIG_DIR, &root_dir);}
    
    let logs_dir = root_dir.clone() + LOGS_DIR;
    check_tms_dir(&logs_dir, "logs directory", &mistrust);
    
    let database_dir = root_dir.clone() + DATABASE_DIR;
    check_tms_dir(&database_dir, "database directory", &mistrust);

    let certs_dir = root_dir.clone() + CERTS_DIR;
    dir_created = check_tms_dir(&certs_dir, "certs directory", &mistrust);
    if dir_created {copy_resource_files(&certs_dir, CERTS_DIR, &root_dir);}
    
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
fn check_tms_dir(dir: &String, msgname: &str, mistrust: &Mistrust) -> bool {
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
        // Directory not created.
        false
    } else {
        // Create the directory with the correct permissions.
        match mistrust.make_directory(path) {
            Ok(_) => (),
            Err(e) => {
                panic!("Make directory error for {:?}: {}", path, &e.to_string());
            }
        }
        // Directory created.
        true
    }
}

// ---------------------------------------------------------------------------
// copy_resource_files:
// ---------------------------------------------------------------------------
/** Copy the resource files to the target directory from the ./resources directory.
 * This function will not copy any files if the current working directory of this
 * program does have a subdirectory named "resources".
 * 
 * Since this function is only called on source code resource directories that contain 
 * files copied during installation process, at least one file must be available
 * for copying each time this function is called.
 * 
 * Source files known to contain jinja2-style template variables will have those 
 * variables replaced with values hardcoded in this function.     
 */
fn copy_resource_files(target_dir: &String, dir_suffix: &str, root_dir: &String) {
    // Create the source directory pathname.
    let source_dir = RESOURCES_DIR.to_string() + dir_suffix;

    // Get the files in the specified resource directory.
    let pathbufs = match tms_utils::get_files_in_dir(source_dir.as_str()) {
        Ok(p) => p,
        Err(e) => {
            panic!("Unable to list files in directy {}: {}", &source_dir, e);
        }
    };

    // Don't call this function if there's nothing to copy.
    if pathbufs.is_empty() {
        let mut msg = format!("Installation aborted because no files were found in directory {}. ", &source_dir);
        msg += "For new installations, copy the 'resources' directory from source code ";
        msg += format!("and remove the TMS root directory ({}) before retrying.", &root_dir).as_str();
        panic!("{}", msg);
    }

    // Process directories that don't contain files with template variables.
    // Copy each of the files to target directory and set permissions.
    for pathbuf in pathbufs {
        // Construct the full pathname of the target file.
        let os_filename = pathbuf.file_name().expect("Unable to read file name");
        let filename = os_filename.to_string_lossy();
        let target_file = target_dir.to_string() + "/" + &filename;

        // Do we need to replace template variables in the source file?
        // If so, take the first branch and perform jinja2-style substitutions.
        if dir_suffix == CONFIG_DIR && filename == LOG4RS_CONFIG_FILE[1..] {
            // Create the source file path.
            let source_file = source_dir.to_string() + "/" + &filename;

            // Create the template processor and initialize with a single file.
            let mut tera = Tera::default();
            match tera.add_template_file(&source_file, None) {
                Ok(_) => (),
                Err(e) => {
                    panic!("Unable to read and parse template file {}: {}", &source_file, e);
                },
            };

            // Set the replacement value in a context and render the final output string. 
            let mut context = tera::Context::new();
            context.insert("TMS_ROOT_DIR", root_dir);
            let rendered = match tera.render(&source_file, &context) {
                Ok(s) => s,
                Err(e) => {
                    panic!("Unable to render template file {}: {}", &source_file, e);
                },
            };

            // Write the file with all substitutions performed.
            match fs::write(&target_file, rendered) {
                Ok(_) => (),
                Err(e) => {
                    panic!("Unable to write rendered file {}: {}", &target_file, e);
                },
           };
        } else {
            // Copy the local resource file to the target file with no template substitutions.
            match fs::copy(&pathbuf, &target_file) {
                Ok(_) => (),
                Err(e) => {
                    panic!("File copy from {:?} to {} failed: {}", &pathbuf, &target_file, e);
                },
            }
        }

        // Set the target's permissions.
        match fs::set_permissions(&target_file, Permissions::from_mode(0o600)) {
            Ok(_) => (),
            Err(e) => {
                panic!("Unable to set 0o600 permission on {}: {}", target_file, e);
            },    
        };

        // Print installation information to stdout since logger has not initialized yet.
        println!("*** Copied default installation file to '{}'", &target_file);
    }
}

// ---------------------------------------------------------------------------
// check_resource_files:
// ---------------------------------------------------------------------------
/** Make sure all required configuration files are present in the tms directory
 * subtree.  The log4rs.yml and tms.toml files have already been checked and 
 * read, so we don't need to do that here (see init_log() and get_parms()).  
 * 
 * We panic if either of the pem files are not found or don't have the proper permissions.
 */
fn check_resource_files() {
    // Get the directory in which the pem files reside.
    let cert_path = Path::new(&TMS_DIRS.certs_dir);

    // Set up loop for pem file checking.
    let files = vec!(CERT_PEM_FILE, KEY_PEM_FILE, );
    for f in files {
        // Get the pem file as a path. Remove leading slash for join to work.
        let pem_file_buf = cert_path.join(&f[1..f.to_string().len()]); 
        let pem_file_path = pem_file_buf.as_path();

        // Make sure the pem file exists as a file.
        if !pem_file_path.is_file() {
            panic!("The TMS pem file must exist: {}", pem_file_path.to_string_lossy());
        }

        // Make sure the pem file has the proper permissions.
        let meta = pem_file_path.metadata()
            .unwrap_or_else(|_| panic!("Unable to read metadata for: {}", pem_file_path.to_string_lossy()));
        let perm = meta.permissions().mode();
        if perm & 0o777 != 0o600 {
            panic!("The TMS pem file must be have 0o600 permissions: {}", pem_file_path.to_string_lossy());
        }
    }

    // Make sure we can read the migration directory.
    let migration_path = Path::new(&TMS_DIRS.migrations_dir);
    let paths = match fs::read_dir(migration_path) {
        Ok(p) => p,
        Err(e)=> {
            panic!("The TMS migration directory '{}' must exist: {}", migration_path.to_string_lossy(), e);
        }
    };

    // Check that there's at least 1 database migration file in the configured directory.
    let mut found = false;
    for path in paths {
        match path {
            Ok(entry) => {
                if entry.path().is_file() {
                    found = true;
                    break;
                }
            },
            Err(e) => {
                panic!("Error reading TMS migration directory '{}': {}", migration_path.to_string_lossy(), e);
            },
        }
    }
    if !found {
        panic!("No migration files found in TMS migration directory: {}", migration_path.to_string_lossy());
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

    // Validate config values
    config.validate()?;

    Ok(Parms { config_file: config_file_abs, config })
}

// ---------------------------------------------------------------------------
// init_authz_args:
// ---------------------------------------------------------------------------
fn init_authz_args() -> AuthzArgs {
    // Create the authz specs for each authz validation type.
    // Note that only two of the four types are currently implemented.
    let client_spec = AuthzSpec {
        id: X_TMS_CLIENT_ID, 
        secret: X_TMS_CLIENT_SECRET, 
        display_name: "client",
        sql_query: GET_CLIENT_SECRET,
    };
    let admin_spec = AuthzSpec {
        id: X_TMS_ADMIN_ID, 
        secret: X_TMS_ADMIN_SECRET, 
        display_name: "admin",
        sql_query: GET_ADMIN_SECRET,
    };

    // Create and fill in the hashmap of authz specs.
    let mut args = AuthzArgs {specs: HashMap::new()};
    args.specs.insert(AuthzTypes::ClientOwn, client_spec);
    args.specs.insert(AuthzTypes::TenantAdmin, admin_spec);
    args
}

// ***************************************************************************
//                             Config Functions
// ***************************************************************************
// ---------------------------------------------------------------------------
// init_runtime_context:
// ---------------------------------------------------------------------------
pub fn init_runtime_context() -> RuntimeCtx {
    // Make sure the 2 pem files are installed in the configured directory.
    check_resource_files();

    // If either of these fail the application aborts.
    let parms = get_parms().expect("FAILED to read configuration file.");
    let db_pool = block_on(db_init::init_db()).expect("Error initializing DB");
    
    // Reset the test tenant's enable flag.

    // Return the runtime context.
    RuntimeCtx {parms, db: db_pool, authz: &AUTHZ_ARGS, tms_args: &TMS_ARGS, tms_dirs: &TMS_DIRS}
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
