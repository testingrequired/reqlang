mod error;

use axum::Router;
use error::Error;
use include_dir::{Dir, include_dir};
use tower_serve_static::ServeDir;

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Get port from env var
    let port = std::env::var("REQLANG_WEB_CLIENT_PORT")
        // Default to a random port
        .unwrap_or("0".to_string())
        // Ensure it's a number
        .parse::<u16>()
        .expect("Invalid port set on REQLANG_WEB_CLIENT_PORT");

    // Package the files in static inside built binary
    static ASSETS_DIR: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/static");
    let static_dir = ServeDir::new(&ASSETS_DIR);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    let addr = listener.local_addr()?;
    let url = format!("http://{addr}");

    eprintln!("reqlang-web-client: {url}");

    webbrowser::open(&url)?;

    let app = Router::new().fallback_service(static_dir.clone());
    axum::serve(listener, app).await?;

    Ok(())
}
