// 8 Dec 2025
// Claude prompt: Basic HTTP Server with Poem
use poem::{
    get, handler, listener::TcpListener, web::Path, Route, Server,
};

#[handler]
fn hello() -> String {
    "Hello, World!".to_string()
}

#[handler]
fn greet(Path(name): Path<String>) -> String {
    format!("Hello, {}!", name)
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let app = Route::new()
        .at("/", get(hello))
        .at("/greet/:name", get(greet));

    println!("Server running on http://127.0.0.1:3000");
    
    Server::new(TcpListener::bind("127.0.0.1:3000"))
        .run(app)
        .await
}
