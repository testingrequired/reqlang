mod error;

use axum::Router;
use error::Error;
use tower_http::services::ServeDir;

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt::init();

    let static_dir = ServeDir::new("static");
    let app = Router::new().fallback_service(static_dir.clone());

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;

    eprintln!("Server is running! http://localhost:3000");

    axum::serve(listener, app).await?;

    Ok(())
}
