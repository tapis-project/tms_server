use anyhow::Result;
use lazy_static::lazy_static;
use log::info;
use poem::listener::{Listener, RustlsCertificate, RustlsConfig};
use poem::{listener::TcpListener, Route};
use poem_openapi::{param::Query, payload::PlainText, OpenApi, OpenApiService};

// TMS Utilities
use crate::v1::tms::new_ssh_keys::NewSshKeysApi;
use crate::v1::tms::public_key::PublicKeyApi;
use utils::config::{init_log, init_runtime_context, RuntimeCtx};
use utils::errors::Errors;

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
// server main loop:
// ---------------------------------------------------------------------------
#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    // Announce ourselves.
    println!("Starting tms_server!");

    // Configure out log.
    init_log();
    
    // Force the reading of input parameters and initialization of runtime context.
    // The runtime context also initializes the database, which makes db connections
    // available to all modules.
    info!("{}", Errors::InputParms(format!("{:#?}", *RUNTIME_CTX)));

    // Assign base URL.
    let tms_url = format!("{}:{}{}",
        RUNTIME_CTX.parms.config.http_addr, 
        RUNTIME_CTX.parms.config.http_port, 
        "/v1");

    // Create a tuple with both the Api struct and the imported user::UserApi struct
    let endpoints = (Api, NewSshKeysApi, PublicKeyApi);
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
//                             Hello Endpoint
// ***************************************************************************
// Hello structure.
struct Api;

// ---------------------------------------------------------------------------
// hello endpoint:
// ---------------------------------------------------------------------------
#[OpenApi]
impl Api {
    #[oai(path = "/hello", method = "get")]
    async fn index(&self, name: Query<Option<String>>) -> PlainText<String> {
        match name.0 {
            Some(name) => PlainText(format!("hello, {}!", name)),
            None => PlainText("hello!".to_string()),
        }
    }
}
