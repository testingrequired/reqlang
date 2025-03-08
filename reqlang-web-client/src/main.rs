mod error;

use axum::Router;
use error::Error;
use include_dir::{Dir, include_dir};
use tower_serve_static::ServeDir;

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt::init();

    // Get port from env var
    let port = std::env::var("REQLANG_WEB_CLIENT_PORT")
        .unwrap_or("0".to_string())
        .parse::<u16>()
        .expect("Unable to parse port from REQLANG_WEB_CLIENT_PORT");

    static ASSETS_DIR: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/static");

    let static_dir = ServeDir::new(&ASSETS_DIR);
    let app = Router::new().fallback_service(static_dir.clone());

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;

    let addr = listener.local_addr()?;

    let url = format!("http://{addr}");

    eprintln!("Server is running! {url}");

    webbrowser::open(&url)?;

    axum::serve(listener, app).await?;

    Ok(())
}
