#![forbid(unsafe_code)]

use std::time::Duration;
use anyhow::Result;
use lazy_static::lazy_static;
use log::info;
use poem::listener::{Listener, OpensslTlsConfig};
use poem::{listener::TcpListener, Route};
use poem_openapi::{param::Query, payload::PlainText, OpenApi, OpenApiService};
use poem_extensions::api;
use futures::executor::block_on;

// TMS APIs
use crate::v1::tms::client_create::CreateClientApi;
use crate::v1::tms::client_delete::DeleteClientApi;
use crate::v1::tms::client_get::GetClientApi;
use crate::v1::tms::client_list::ListClientApi;
use crate::v1::tms::client_update_secret::UpdateClientSecretApi;
use crate::v1::tms::client_update::UpdateClientApi;
use crate::v1::tms::pubkeys_create::NewSshKeysApi;
use crate::v1::tms::pubkeys_retrieve::PublicKeyApi;
use crate::v1::tms::user_mfa_create::CreateUserMfaApi;
use crate::v1::tms::user_mfa_delete::DeleteUserMfaApi;
use crate::v1::tms::user_mfa_get::GetUserMfaApi;
use crate::v1::tms::user_mfa_list::ListUserMfaApi;
use crate::v1::tms::user_mfa_update::UpdateUserMfaApi;
use crate::v1::tms::pubkeys_delete::DeletePubkeysApi;
use crate::v1::tms::pubkeys_get::GetPubkeysApi;
use crate::v1::tms::pubkeys_list::ListPubkeysApi;
use crate::v1::tms::pubkeys_update::UpdatePubkeyApi;
use crate::v1::tms::user_hosts_create::CreateUserHostsApi;
use crate::v1::tms::user_hosts_get::GetUserHostsApi;
use crate::v1::tms::user_hosts_list::ListUserHostsApi;
use crate::v1::tms::user_hosts_delete::DeleteUserHostsApi;
use crate::v1::tms::user_hosts_update::UpdateUserHostsApi;
use crate::v1::tms::delegations_create::CreateDelegationsApi;
use crate::v1::tms::delegations_get::GetDelegationsApi;
use crate::v1::tms::delegations_list::ListDelegationsApi;
use crate::v1::tms::delegations_delete::DeleteDelegationsApi;
use crate::v1::tms::delegations_update::UpdateDelegationsApi;
use crate::v1::tms::tenants_create::CreateTenantsApi;
use crate::v1::tms::tenants_get::GetTenantsApi;
use crate::v1::tms::tenants_list::ListTenantsApi;
use crate::v1::tms::tenants_delete::DeleteTenantsApi;
use crate::v1::tms::tenants_update::UpdateTenantsApi;
use crate::v1::tms::tenants_wipe::WipeTenantsApi;
use crate::v1::tms::hosts_create::CreateHostsApi;
use crate::v1::tms::hosts_get::GetHostsApi;
use crate::v1::tms::hosts_delete::DeleteHostsApi;
use crate::v1::tms::hosts_list::ListHostsApi;
use crate::v1::tms::reservations_get::GetReservationApi;
use crate::v1::tms::reservations_delete::DeleteReservationApi;
use crate::v1::tms::reservations_delete_related::DeleteRelatedReservationsApi;
use crate::v1::tms::reservations_create::CreateReservationsApi;
use crate::v1::tms::reservations_extend::ExtendReservationsApi;
use crate::v1::tms::version::VersionApi;

// TMS Utilities
use crate::utils::config::{TMS_ARGS, TMS_DIRS, init_log, init_runtime_context, check_prior_installation, RuntimeCtx};
use crate::utils::errors::Errors;
use crate::utils::{keygen, db};

// Modules
mod utils;
mod v1;

// ***************************************************************************
//                                Constants
// ***************************************************************************
const SERVER_NAME : &str = "TmsServer"; // for poem logging

// Server identity.
const TMSS_KEY_FILE:  &str = "/key.pem";
const TMSS_CERT_FILE: &str = "/cert.pem";

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
#[tokio::main(flavor = "multi_thread", worker_threads = 20)]
async fn main() -> Result<(), std::io::Error> {
    // --------------- Initialize TMS -----------------
    // Announce ourselves.
    println!("Starting tms_server!");

    // Initialize the server and allow for early exit.
    if !tms_init() { return Ok(()); }

    // --------------- Main Loop Set Up ---------------
    // Create a tuple with all the endpoints, create the service and add the server urls to it.
    // Note the use of the poem-extensions api! macro, which allows more than 16 non-generic 
    // endpoints to be defined (!).  Consult the poem_extensions documentation if generic 
    // endpoint support is needed.
    let endpoints = 
        api!(HelloApi, NewSshKeysApi, PublicKeyApi, VersionApi, 
         CreateClientApi, GetClientApi, UpdateClientApi, DeleteClientApi, UpdateClientSecretApi, ListClientApi, 
         CreateUserMfaApi, GetUserMfaApi, UpdateUserMfaApi, DeleteUserMfaApi, ListUserMfaApi,
         GetPubkeysApi, ListPubkeysApi, DeletePubkeysApi, UpdatePubkeyApi,
         CreateUserHostsApi, GetUserHostsApi, ListUserHostsApi, DeleteUserHostsApi, UpdateUserHostsApi,
         CreateDelegationsApi, GetDelegationsApi, ListDelegationsApi, DeleteDelegationsApi, UpdateDelegationsApi,
         CreateTenantsApi, GetTenantsApi, ListTenantsApi, DeleteTenantsApi, UpdateTenantsApi, WipeTenantsApi,
         CreateHostsApi, GetHostsApi, DeleteHostsApi, ListHostsApi,
         GetReservationApi, DeleteReservationApi, CreateReservationsApi, ExtendReservationsApi, DeleteRelatedReservationsApi);
    let mut api_service = 
        OpenApiService::new(endpoints, "TMS Server", "0.0.1");
    let urls = &RUNTIME_CTX.parms.config.server_urls;
    for url in urls.iter() {
        api_service = api_service.server(url);
    }
 
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
    // Create and start the server, either https or http
    if RUNTIME_CTX.parms.config.http_addr.starts_with("https") {
        // ** HTTPS: We expect the certificate and key to be in the external data directory.
        let key = RUNTIME_CTX.tms_dirs.certs_dir.clone() + TMSS_KEY_FILE;
        let cert = RUNTIME_CTX.tms_dirs.certs_dir.clone() + TMSS_CERT_FILE;
        poem::Server::new(
            TcpListener::bind(addr).openssl_tls(
                OpensslTlsConfig::new()
                        .cert_from_file(cert)
                        .key_from_file(key)
            )
        )
        .name(SERVER_NAME)
        .idle_timeout(Duration::from_secs(50))
        .run(app)
        .await
    } else {
        // ** HTTP only
        poem::Server::new(TcpListener::bind(addr)).name(SERVER_NAME).run(app).await
     }
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
fn tms_init() -> bool {
    // Parse command line args and determine if early exit.
    println!("*** Command line arguments *** \n{:?}\n", *TMS_ARGS);
    check_prior_installation(); // Cannot run before installation.
    
    // Directory setup.
    println!("*** Runtime file locations *** \n{:?}\n", *TMS_DIRS);

    // Configure out log.
    init_log();

    // Force the reading of input parameters and initialization of runtime context.
    // The runtime context also initializes the database, which makes db connections
    // available to all modules.
    info!("{}", Errors::InputParms(format!("{:#?}", *RUNTIME_CTX)));

    // Log build info.
    print_version_info();

    // Insert default records into database if they don't already exist.
    // This call is a no-op except when the --install option is set.
    let inserts = block_on(db::create_std_tenants())
        .expect("Unable to create or access standard tenant records.");
    info!("Number of standard tenants created: {}.", inserts);

    // Optionally insert test records into test tenant
    // only if we just created the standard tenants.
    if inserts > 0 {db::check_test_data();}

    // Initialize the key generator.
    keygen::init_keygen();

    // Was this an initial installation?
    if TMS_ARGS.install { 
        println!("Exiting: TMS root directory installed and initialized at {}", &TMS_DIRS.root_dir);
        false 
    } else {
        // Fully initialized.
        true
    }
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
