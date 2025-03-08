mod error;

use axum::Router;
use error::Error;
use include_dir::{Dir, include_dir};
use tower_serve_static::ServeDir;

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt::init();

    static ASSETS_DIR: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/static");

    let static_dir = ServeDir::new(&ASSETS_DIR);
    let app = Router::new().fallback_service(static_dir.clone());

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;

    eprintln!("Server is running! http://localhost:3000");

    axum::serve(listener, app).await?;

    Ok(())
}
