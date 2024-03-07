use poem::{listener::TcpListener, Route};
use poem_openapi::{param::Query, payload::PlainText, OpenApi, OpenApiService};

mod v1;

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

