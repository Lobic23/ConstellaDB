mod handlers;
mod state;

use clap::Parser;
use axum::{routing::{get, post}, Router};
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use state::AppState;
use handlers::{handle_health, handle_query};

use constella_db::modules::db::Engine;

#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long)]
    port: Option<u32>,
}

fn get_local_ip() -> std::io::Result<std::net::IpAddr> {
    let socket = std::net::UdpSocket::bind("0.0.0.0:0")?;
    socket.connect("8.8.8.8:80")?;
    Ok(socket.local_addr()?.ip())
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    // Extract port from args
    let port = args.port.unwrap_or(8080);
    let ip = get_local_ip().unwrap();

    tracing_subscriber::fmt::init();

    // Engine::new() is async, so await it
    let engine = Engine::new().await;
    let state = Arc::new(AppState::new(engine));

    let app = Router::new()
        .route("/query", post(handle_query))
        .route("/health", get(handle_health))
        .with_state(state)
        .layer(CorsLayer::permissive());

    let addr = format!("{}:{}", ip, port);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    let bound_port = listener.local_addr().unwrap().port();

    tracing::info!("query_service listening on {}:{}", ip, bound_port);
    axum::serve(listener, app).await.unwrap();
}