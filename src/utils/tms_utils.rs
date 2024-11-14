#![forbid(unsafe_code)]

use path_absolutize::Absolutize;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use glob::glob;
use std::process::{Command, ExitStatus, Output, Stdio};
use std::os::unix::fs::MetadataExt;
use execute::Execute;
use chrono::{Utc, DateTime, SecondsFormat, FixedOffset, ParseError, Duration};
use semver::VersionReq;
use futures::executor::block_on;

use poem::Request;

use rand_core::{RngCore, OsRng};
use hex;
use sha2::{Sha512, Digest};

use anyhow::{Result, anyhow};
use log::{error, debug, LevelFilter};

use crate::utils::db_statements::PLACEHOLDER;
use crate::utils::authz::{AuthzResult, AuthzTypes};
use crate::utils::db::is_tenant_enabled;

// ----------- Constants
// The chrono library's MAX_UTC causes overflow during string
// converstions because year is more than 4 digits.  Use this
// value instead for long durations or timeouts.
pub const MAX_TMS_UTC: &str = "9999-12-31T23:59:59Z";

// ***************************************************************************
// GENERAL PUBLIC FUNCTIONS
// ***************************************************************************
// ---------------------------------------------------------------------------
// get_absolute_path:
// ---------------------------------------------------------------------------
/** Replace tilde (~) and environment variable values in a path name and
 * then construct the absolute path name.  The difference between 
 * absolutize and standard canonicalize methods is that absolutize does not 
 * care about whether the file exists and what the file really is.
 * 
 * Here's a short version of how canonicalize would be used: 
 * 
 *   let p = shellexpand::full(path).unwrap();
 *   fs::canonicalize(p.deref()).unwrap().into_os_string().into_string().unwrap()
 * 
 * We have the option of using these to two ways to generate a String from the
 * input path (&str):
 * 
 *   path.to_owned()
 *   path.deref().to_string()
 * 
 * I went with the former on a hunch that it's the most appropriate, happy
 * to change if my guess is wrong.
 */
#[allow(dead_code)]
pub fn get_absolute_path(path: &str) -> String {
    // Replace ~ and environment variable values if possible.
    // On error, return the string version of the original path.
    let s = match shellexpand::full(path) {
        Ok(x) => x,
        Err(_) => return path.to_owned(),
    };

    // Convert to absolute path if necessary.
    // Return original input on error.
    let p = Path::new(s.deref());
    let p1 = match p.absolutize() {
        Ok(x) => x,
        Err(_) => return path.to_owned(),
    };
    let p2 = match p1.to_str() {
        Some(x) => x,
        None => return path.to_owned(),
    };

    p2.to_owned()
}

// ---------------------------------------------------------------------------
// get_files_in_dir:
// ---------------------------------------------------------------------------
/** Return a list of PathBufs representing the immediate children of the directory.
 * This function is not recursive and does not include subdirectories.
 */
pub fn get_files_in_dir(dir: &str) -> Result<Vec<PathBuf>> {
    
    // Create the result vector and globify the directory string.
    let mut v = vec!();
    let pattern = if dir.ends_with('/') {dir.to_string() + "*"} else {dir.to_string() + "/*"};

    // Collect all the immediate files in the directory. 
    for entry in glob(&pattern)? {
        match entry {
            Ok(f) => {
                if f.is_file() {v.push(f);}
            },
            Err(e) => {
                let msg = format!("Unable to access an directory entry in {}: {:?}.", &pattern, e);
                error!("{}", msg);
                return Result::Err(anyhow!(msg));
            },
        }
    }

    //let v = vec!();
    Ok(v)
}

// ---------------------------------------------------------------------------
// calc_expires_at:
// ---------------------------------------------------------------------------
/** The ttl should never be negative, but we handle it if it is. */
pub fn calc_expires_at(now : DateTime<Utc>, ttl_minutes : i32) -> String {
    if ttl_minutes < 0 {
        MAX_TMS_UTC.to_string()
    } else {
        timestamp_utc_secs_to_str(now + Duration::minutes(ttl_minutes as i64))
    }
}

// ---------------------------------------------------------------------------
// timestamp_utc:
// ---------------------------------------------------------------------------
/** Get the current UTC timestamp */
pub fn timestamp_utc() -> DateTime<Utc> {
    Utc::now()
}

// ---------------------------------------------------------------------------
// timestamp_str:
// ---------------------------------------------------------------------------
/** Get the current UTC timestamp as a string in rfc3339 format, which looks
 * like this:  2022-09-13T14:14:42.719849Z
 */
#[allow(dead_code)]
pub fn timestamp_str() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Micros, true)
}

// ---------------------------------------------------------------------------
// timestamp_utc_to_str:
// ---------------------------------------------------------------------------
/** Convert a UTC datetime to rfc3339 format with microsecond precision, which 
 * looks like this:  2022-09-13T14:14:42.719849Z
 */
#[allow(dead_code)]
pub fn timestamp_utc_to_str(ts: DateTime<Utc>) -> String {
    ts.to_rfc3339_opts(SecondsFormat::Micros, true)
}

// ---------------------------------------------------------------------------
// timestamp_utc_secs_to_str:
// ---------------------------------------------------------------------------
/** Convert a UTC datetime to rfc3339 format with second precision, which looks 
 * like this:  2022-09-13T14:14:42Z
 */
#[allow(dead_code)]
pub fn timestamp_utc_secs_to_str(ts: DateTime<Utc>) -> String {
    ts.to_rfc3339_opts(SecondsFormat::Secs, true)
}

// ---------------------------------------------------------------------------
// timestamp_str_to_datetime:
// ---------------------------------------------------------------------------
/** Convert a timestamp string in rfc3339 format (ex: 2022-09-13T14:14:42.719849912+00:00)
 * to a DateTime object.  The result will contain a parse error if the string
 * does not conform to rfc3339.
 */
#[allow(dead_code)]
pub fn timestamp_str_to_datetime(ts: &str) -> Result<DateTime<FixedOffset>, ParseError> {
    DateTime::parse_from_rfc3339(ts)
}

// ---------------------------------------------------------------------------
// run_command:
// ---------------------------------------------------------------------------
/** Make an operating system call and return an Output object that contains
 * the result code and stdout/stderr as vectors.  If the command cannot be run
 * or if it runs and returns a non-zero exit code, this method writes the log 
 * before returning an error.  
 * 
 * The task parameter prefixes any error message logged or returned by this
 * function.
 * 
 * The only way Ok is returned is when the command has a zero exit code.
 */
#[allow(clippy::needless_return, dead_code)]
pub fn run_command(mut command: Command, task: &str) -> Result<Output> {
    // Capture all output.
    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());
 
    // Return an output object or error.
    // Errors are logged before returning.
    match command.execute_output() {
        Ok(o) => {
            // Check for success here.
            if o.status.success() {return Result::Ok(o);}
                else {
                    let msg = task.to_string() + ": " + 
                        &String::from_utf8(o.stderr)
                        .unwrap_or(run_command_emsg(command, o.status));
                    error!("{}", msg);
                    return Result::Err(anyhow!(msg));
                };
        },
        Err(e) => {
            let msg = task.to_string() + ": " + &e.to_string();
            error!("{}", msg);
            return Result::Err(anyhow!(msg));
        },
    };
}

// ---------------------------------------------------------------------------
// is_executable:
// ---------------------------------------------------------------------------
// Determine whether a path--typically a file--is executable.
#[allow(dead_code)]
pub fn is_executable(path: &Path) -> bool {
    let meta = path.metadata()
        .unwrap_or_else(|_| panic!("Unable to retrieve metadata for {:?}", path));
    meta.mode() & 0o111 != 0
}

// ***************************************************************************
//                                  Traits
// ***************************************************************************
pub trait RequestDebug {
    type Req;
    fn get_request_info(&self) -> String;
}

// ---------------------------------------------------------------------------
// debug_request:
// ---------------------------------------------------------------------------
// Dump http request information to the log.
pub fn debug_request(http_req: &Request, req: &impl RequestDebug) {
    // Check that debug or higher logging is in effect.
    let level = log::max_level();
    if level < LevelFilter::Debug {
        return;
    }
    
    // Accumulate the output.
    let mut s = "\n".to_string();

    // Restate the URI.
    let uri = http_req.uri();
    s += format!("  URI: {:?}\n", uri).as_str();

    // Accumulate the headers except for those that have substring "SECRET" in name.
    let it = http_req.headers().iter();
    for v in it {
        if v.0.as_str().to_lowercase().contains("secret") {continue};
        s += format!("  Header: {} = {:?} \n", v.0, v.1).as_str();
    };

    // List query parameters.
    if let Some(q) = uri.query() {
        s += format!("  Query Parameters: {:?}\n", q).as_str();
    } else {
        s += "  * No Query Parameters\n";
    }

    // Add the request's information.
    s += req.get_request_info().as_str();

    // Write the single log record.
    debug!("{}", s);
}

// ---------------------------------------------------------------------------
// create_hex_secret:
// ---------------------------------------------------------------------------
/** Get 24 bytes of random bits and convert them to a hex string. */
pub fn create_hex_secret() -> String {
    let mut dft_key = [0u8; 24];
    OsRng.fill_bytes(&mut dft_key);
    hex::encode(dft_key)
}

// ---------------------------------------------------------------------------
// hash_hex_secret:
// ---------------------------------------------------------------------------
/** Take a hex secret as provided to the user and hash it for storage in the
 * database.
 */
pub fn hash_hex_secret(hex_str: &String) -> String {
    let mut hasher = Sha512::new();
    hasher.update(hex_str);
    let raw = hasher.finalize();
    hex::encode(raw)
}

// ---------------------------------------------------------------------------
// validate_semver:
// ---------------------------------------------------------------------------
/** Use the semver library to enforce cargo's implementation of semantic 
 * versioning.  Either the candidate string parses and true is returned or
 * an error is returned.  
 * 
 * See https://docs.rs/semver/latest/semver/enum.Op.html for an discussion
 * on versioning operators and their meanings. 
 */
pub fn validate_semver(semver: &str) -> Result<bool> {
    match VersionReq::parse(semver) {
        Ok(_) => Ok(true),
        Err(e) => Err(anyhow!(e)),
    }
}

// ---------------------------------------------------------------------------
// sql_substitute_client_constraint:
// ---------------------------------------------------------------------------
/** Complete the sql select statement by substituting an appropriate value
 * for the placeholder text in the query template.  The substitution 
 * restricts the query to a specific client when the request was authorized 
 * using X_TMS_CLIENT_ID.  When authorized using a tenant admin, the record
 * created by any client in the tenant can be returned.
 * 
 * The sql_query is the template text into which a value is substituted 
 * for the placeholder.  The authz_result is the result of a prior authorize()
 * call.
 */
pub fn sql_substitute_client_constraint(sql_query: &str, authz_result: &AuthzResult) -> String {
    // We can only get here if authorization succeeded, so we don't
    // really have to worry about missing authz result data.  To be
    // doubly safe, we return the unresolved template query which 
    // will be rejected by the database.
    let authz_type = match &authz_result.authz_type {
        Some(a) => a,
        None => return sql_query.to_string(), // poison pill
    };

    // Construct the replacement text.
    let replacement = if *authz_type == AuthzTypes::ClientOwn {
        // Get the client ID that was used for authorization.
        let client_id = match &authz_result.hdr_id {
            Some(id) => id,
            None => return sql_query.to_string(), // another poison pill 
        };
        " AND client_id = '".to_string() + client_id + "'"
    }
    else {"".to_string()};

    // Return the template after substitution.
    sql_query.replace(PLACEHOLDER, replacement.as_str())    
}

// ---------------------------------------------------------------------------
// check_tenant_enabled:
// ---------------------------------------------------------------------------
/** Wrapper for the actual database call that handles errors and logging. */
pub fn check_tenant_enabled(tenant: &String) -> bool {
    match block_on(is_tenant_enabled(tenant)) {
        Ok(enabled) => enabled,
        Err(e) => {
            error!("Unable to determine if tenant '{}' is enabled: {}", tenant, e);
            false
        }
    }
}

// ***************************************************************************
// PRIVATE FUNCTIONS
// ***************************************************************************
// ---------------------------------------------------------------------------
// run_command_emsg:
// ---------------------------------------------------------------------------
/** Return a message for commands that return non-zero exit codes. */
#[allow(dead_code)]
fn run_command_emsg(command: Command, status: ExitStatus) -> String {
    "Unknown error condition returned by command: ".to_owned() + 
    command.get_program().to_str().unwrap_or("unknown") +
    " with exit status: " + &status.to_string()
}


