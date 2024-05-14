#![forbid(unsafe_code)]

use anyhow::Result;
use lazy_static::lazy_static;
use log::info;
use poem::listener::{Listener, RustlsCertificate, RustlsConfig};
use poem::{listener::TcpListener, Route};
use poem_openapi::{param::Query, payload::PlainText, OpenApi, OpenApiService};

use futures::executor::block_on;

// TMS Utilities
use crate::v1::tms::new_ssh_keys::NewSshKeysApi;
use crate::v1::tms::public_key::PublicKeyApi;
use crate::v1::tms::version::VersionApi;
use crate::utils::config::{init_log, init_runtime_context, RuntimeCtx};
use crate::utils::errors::Errors;
use crate::utils::{keygen, db};

// Modules
mod utils;
mod v1;

// ***************************************************************************
//                                Constants
// ***************************************************************************
const SERVER_NAME : &str = "TmsServer"; // for poem logging

// ***************************************************************************
//                             Static Variables
// ***************************************************************************
// Lazily initialize the parameters variable so that is has a 'static lifetime.
// We also initialize the database connection pool and run db migrations.
// We exit if we can't read our parameters or access the database.
lazy_static! {
    static ref RUNTIME_CTX: RuntimeCtx = init_runtime_context();
}

// ---------------------------------------------------------------------------
// main:
// ---------------------------------------------------------------------------
#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    // --------------- Initialize TMS -----------------
    // Announce ourselves.
    println!("Starting tms_server!");

    // Initialize the server.
    tms_init();

    // --------------- Main Loop Set Up ---------------
    // Assign base URL.
    let tms_url = format!("{}:{}{}",
        RUNTIME_CTX.parms.config.http_addr, 
        RUNTIME_CTX.parms.config.http_port, 
        "/v1");

    // Create a tuple with both the HelloApi struct and the imported user::UserApi struct
    let endpoints = (HelloApi, NewSshKeysApi, PublicKeyApi, VersionApi);
    let api_service = 
        OpenApiService::new(endpoints, "TMS Server", "0.0.1").server(tms_url);

    // Allow the generated openapi specs to be retrieved from the server.
    let spec = api_service.spec_endpoint();
    let spec_yaml = api_service.spec_endpoint_yaml();

    // Create the routes and run the server.
    let addr = format!("{}{}", "0.0.0.0:", RUNTIME_CTX.parms.config.http_port);
    let ui = api_service.swagger_ui();
    let app = Route::new()
        .nest("/v1", api_service)
        .nest("/", ui)
        .at("/spec", spec)
        .at("/spec_yaml", spec_yaml);

    // ------------------ Main Loop -------------------
    poem::Server::new(
        TcpListener::bind(addr).rustls(
            RustlsConfig::new().fallback(
                RustlsCertificate::new()
                    .key(std::fs::read("key.pem")?)
                    .cert(std::fs::read("cert.pem")?),
            ),
        ),
    )
    .name(SERVER_NAME)
    .run(app)
    .await
}

// ***************************************************************************
//                             Private Functions
// ***************************************************************************
// ---------------------------------------------------------------------------
// tms_init:
// ---------------------------------------------------------------------------
/** Initialing all subsystems and data structures other than those needed
 * to configure the main loop processor.
 */
fn tms_init() {
    // Configure out log.
    init_log();
    
    // Force the reading of input parameters and initialization of runtime context.
    // The runtime context also initializes the database, which makes db connections
    // available to all modules.
    info!("{}", Errors::InputParms(format!("{:#?}", *RUNTIME_CTX)));

    // Log build info.
    print_version_info();

    // Insert default records into database if they don't already exist.
    let inserts = block_on(db::create_std_tenants())
        .expect("Unable to create or access standard tenant records.");
    info!("Number of standard tenants created: {}.", inserts);

    // Optionally insert test records into test tenant.
    db::check_test_data();

    // Initialize keygen subsystem.
    keygen::init_keygen();
}

// ---------------------------------------------------------------------------
// print_version_info:
// ---------------------------------------------------------------------------
fn print_version_info() {
    // Log build info.
    info!("{}.", format!("\n*** Running TMS={}, BRANCH={}, COMMIT={}, DIRTY={}, SRC_TS={}, RUSTC={}",
                        option_env!("CARGO_PKG_VERSION").unwrap_or("unknown"),
                        env!("GIT_BRANCH"),
                        env!("GIT_COMMIT_SHORT"),
                        env!("GIT_DIRTY"),
                        env!("SOURCE_TIMESTAMP"),
                        env!("RUSTC_VERSION")),
    );
}

// ***************************************************************************
//                             Hello Endpoint
// ***************************************************************************
// Hello structure.
struct HelloApi;

// ---------------------------------------------------------------------------
// hello endpoint:
// ---------------------------------------------------------------------------
#[OpenApi]
impl HelloApi {
    #[oai(path = "/tms/hello", method = "get")]
    async fn index(&self, name: Query<Option<String>>) -> PlainText<String> {
        match name.0 {
            Some(name) => PlainText(format!("hello, {}!", name)),
            None => PlainText("hello!".to_string()),
        }
    }
}

// // ---------------------------------------------------------------------------
// // create_std_tenants:
// // ---------------------------------------------------------------------------
// async fn create_std_tenants() -> Result<u64> {
//     // Get the timestamp string.
//     let now = timestamp_utc();
//     let current_ts = timestamp_utc_secs_to_str(now);

//     // Get a connection to the db and start a transaction.
//     let mut tx = RUNTIME_CTX.db.begin().await?;

//     // -------- Insert the two standard tenants.
//     let dft_result = sqlx::query(INSERT_STD_TENANTS)
//         .bind(DEFAULT_TENANT)
//         .bind(&current_ts)
//         .bind(&current_ts)
//         .execute(&mut *tx)
//         .await?;

//     let tst_result = sqlx::query(INSERT_STD_TENANTS)
//         .bind(TEST_TENANT)
//         .bind(&current_ts)
//         .bind(&current_ts)
//         .execute(&mut *tx)
//         .await?;

//     // Commit the transaction.
//     tx.commit().await?;

//     // Only create the test data once.
//     let rows = dft_result.rows_affected() + tst_result.rows_affected();
//     check_test_data(rows, &current_ts);

//     // Return the number of tenant insertions that took place.
//     Ok(rows)
// }

// // ---------------------------------------------------------------------------
// // check_test_data:
// // ---------------------------------------------------------------------------
// fn check_test_data(rows: u64, current_ts: &String) {
//     if rows != 0 {
//         return;
//     }

//     // Assume we are initializing for the first time and need
//     // to populate the test tenant with some dummy data.
//     match block_on(create_test_data(current_ts)) {
//         Ok(b) => {
//             if b {
//                 info!("Test records inserted in test tenant.");
//             } 
//         }
//         Err(e) => {
//             warn!("Ignoring error while inserting test records into test tenant: {}", e.to_string());
//         }
//     };
// }

// // ---------------------------------------------------------------------------
// // create_test_data:
// // ---------------------------------------------------------------------------
// /** This function either experiences an error or returns true (false is never returned). */
// async fn create_test_data(current_ts: &String) -> Result<bool> {

//     // Get a connection to the db and start a transaction.
//     //let mut tx = RUNTIME_CTX.db.begin().await?;

//     // -------- Populate clients



//     Ok(true)
// }

