mod handlers;
mod state;

use axum::{routing::{get, post}, Router};
use db_module::Engine;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use state::AppState;
use handlers::{handle_health, handle_query};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let engine = Engine::new();
    let state = Arc::new(AppState::new(engine));

    let app = Router::new()
        .route("/query", post(handle_query))
        .route("/health", get(handle_health))
        .with_state(state)
        .layer(CorsLayer::permissive());

    let addr = "0.0.0.0:6767";
    tracing::info!("query_service listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
