#![forbid(unsafe_code)]

use path_absolutize::Absolutize;
use std::ops::Deref;
use std::path::Path;
use std::process::{Command, ExitStatus, Output, Stdio};
use execute::Execute;

use anyhow::{Result, anyhow};
use log::error;

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

// ***************************************************************************
// PRIVATE FUNCTIONS
// ***************************************************************************
// ---------------------------------------------------------------------------
// run_command_emsg:
// ---------------------------------------------------------------------------
/** Return a message for commands that return non-zero exit codes. */
fn run_command_emsg(command: Command, status: ExitStatus) -> String {
    "Unknown error condition returned by command: ".to_owned() + 
    command.get_program().to_str().unwrap_or("unknown") +
    " with exit status: " + &status.to_string()
}
