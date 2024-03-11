use anyhow::Result;
use lazy_static::lazy_static;
use log::info;
use poem::{listener::TcpListener, Route};
use poem_openapi::{param::Query, payload::PlainText, OpenApi, OpenApiService};

// TMS Utilities
use utils::errors::Errors;
use utils::config::{RuntimeCtx, init_log, init_runtime_context};


// Modules
mod v1;
mod utils;

// ***************************************************************************
//                             Static Variables 
// ***************************************************************************
// Lazily initialize the parameters variable so that is has a 'static lifetime.
// We exit if we can't read our parameters.
lazy_static! {
    static ref RUNTIME_CTX: RuntimeCtx = init_runtime_context();
}

struct Api;

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

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    // Announce ourselves.
    println!("Starting tms_server!");

    // Configure out log.
    init_log();

    // Force the reading of input parameters and initialization of runtime context.
    info!("{}", Errors::InputParms(format!("{:#?}", *RUNTIME_CTX)));

	// Create a tuple with both the Api struct and the imported user::UserApi struct
    let endpoints = (Api, v1::tms::TMSApi);
    let api_service =
        OpenApiService::new(endpoints, "TMS Server", "0.0.1").server("http://localhost:3000/v1");
    
    // Allow the generated openapi specs to be retrieved from the server.
    let spec = api_service.spec_endpoint();
    let spec_yaml = api_service.spec_endpoint_yaml();

    let ui = api_service.swagger_ui();
    let app = Route::new()
            .nest("/v1", api_service)
            .nest("/", ui)
            .at("/spec", spec)
            .at("/spec_yaml", spec_yaml);
        poem::Server::new(TcpListener::bind("0.0.0.0:3000"))
        .run(app)
        .await
}
