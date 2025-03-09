mod error;

use axum::{Json, Router, http::StatusCode, routing::post};
use error::Error;

#[cfg(not(feature = "dynamic_assets"))]
use include_dir::{Dir, include_dir};
use serde::Deserialize;
#[cfg(feature = "dynamic_assets")]
use tower_http::services::ServeDir;
#[cfg(not(feature = "dynamic_assets"))]
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

    // Dynamically serve the static directory for development
    #[cfg(feature = "dynamic_assets")]
    let static_dir = ServeDir::new("static");

    // Package the files in static inside built binary
    #[cfg(not(feature = "dynamic_assets"))]
    static ASSETS_DIR: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/static");
    #[cfg(not(feature = "dynamic_assets"))]
    let static_dir = ServeDir::new(&ASSETS_DIR);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    let addr = listener.local_addr()?;
    let url = format!("http://{addr}");

    eprintln!("reqlang-web-client: {url}");

    webbrowser::open(&url)?;

    let app = Router::new()
        .route("/parse", post(parse_request_file))
        .fallback_service(static_dir.clone());
    axum::serve(listener, app).await?;

    Ok(())
}

#[derive(Deserialize)]
struct ParseRequestFile {
    payload: String,
}

async fn parse_request_file(Json(body): Json<ParseRequestFile>) -> (StatusCode, String) {
    let ast = reqlang::Ast::from(&body.payload);
    let result = reqlang::parse(&ast);

    match &result {
        Ok(result) => match serde_json::to_string_pretty(result) {
            Ok(result) => (StatusCode::OK, result),
            Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()),
        },
        Err(err) => match serde_json::to_string_pretty(err) {
            Ok(result) => (StatusCode::BAD_REQUEST, result),
            Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()),
        },
    }
}
