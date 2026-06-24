use axum::{
    extract::State,
    routing::{get, post},
    Json,
    Router,
};

use serde::Deserialize;

use std::sync::{Arc, Mutex};

use db_module::{Attr, Engine, Table, Type};

#[derive(Clone)]
struct AppState {
    engine: Arc<Mutex<Engine>>,
}

#[derive(Deserialize)]
struct CreateTableRequest {
    name: String,
}

async fn health() -> &'static str {
    "DB Service Running"
}

async fn create_table(
    State(state): State<AppState>,
    Json(req): Json<CreateTableRequest>,
) -> String {
    let mut engine = state.engine.lock().unwrap();

    let table = Table {
        name: req.name,
        attrs: vec![
            Attr {
                name: "id".to_string(),
                data_type: Type::Int,
            },
            Attr {
                name: "name".to_string(),
                data_type: Type::VarChar(255),
            },
        ],
    };

    match engine.create_table(&table) {
        Ok(_) => "Table created successfully".to_string(),
        Err(e) => e,
    }
}

#[tokio::main]
async fn main() {
    let state = AppState {
        engine: Arc::new(Mutex::new(Engine::new())),
    };

    let app = Router::new()
        .route("/", get(health))
        .route("/tables", post(create_table))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .unwrap();

    println!("DB Service running at http://localhost:3000");

    axum::serve(listener, app)
        .await
        .unwrap();
}