use axum::{
    extract::{Path, State},
    routing::{delete, get, post, put},
    Json,
    Router,
};

use serde::Deserialize;

use std::sync::{Arc, Mutex};

use db_module::{
    Attr, 
    Engine, 
    Table, 
    Type,
    Entity,
    Value,
    Data,
    Condition,
    Operator,
};

#[derive(Clone)]
struct AppState {
    engine: Arc<Mutex<Engine>>,
}

#[derive(Deserialize)]
struct CreateTableRequest {
    name: String,
}

#[derive(Deserialize)]
struct InsertRequest {
    id: i32,
    name: String,
}

#[derive(Deserialize)]
struct UpdateRequest {
    id: i32,
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

async fn list_table(
    State(state): State<AppState>,
) -> Json<Vec<Table>> {
    let engine = state.engine.lock().unwrap();

    Json(engine.get_tables())
}

async fn drop_table(
    Path(table_name): Path<String>,
    State(state): State<AppState>,
) -> String {
    let mut engine = state.engine.lock().unwrap();

    match engine.drop_table(&table_name) {
        Ok(_) => format!("Table '{}' deleted", table_name),
        Err(e) => e,
    }
}

async fn insert_row(
    Path(table_name): Path<String>,
    State(state): State<AppState>,
    Json(req): Json<InsertRequest>,
) -> String {
    let mut engine = state.engine.lock().unwrap();

    let entity = Entity {
        of: table_name,
        data: vec![
            Data {
                name: "id".to_string(),
                value: Value::Int(req.id),
            },
            Data {
                name: "name".to_string(),
                value: Value::VarChar(req.name),
            },
        ],
    };

    match engine.insert(&entity) {
        Ok(_) => "Row inserted".to_string(),
        Err(e) => e,
    }
}

async fn select_rows(
    Path(table_name): Path<String>,
    State(state): State<AppState>,
) -> String {
    let mut engine = state.engine.lock().unwrap();

    match engine.select(
        &table_name,
        vec!["*"],
        vec![],
    ) {
        Ok(rows) => format!("{:#?}", rows),
        Err(e) => e,
    }
}

async fn update_row(
    Path(table_name): Path<String>,
    State(state): State<AppState>,
    Json(req): Json<UpdateRequest>,
) -> String {
    let mut engine = state.engine.lock().unwrap();

    let updates = vec![
        Data {
            name: "name".to_string(),
            value: Value::VarChar(req.name),
        }
    ];

    let conditions = vec! [
        Condition::Compare {
            attr: "id".to_string(),
            value: Value::Int(req.id),
            op: Operator::Eq,
        }
    ];

    match engine.update(
        &table_name,
        updates,
        conditions,
    ) {
        Ok(count) => format!("{} row(s) updated", count),
        Err(e) => e,
    }
}

async fn delete_row(
    Path((table_name, id)): Path<(String, i32)>,
    State(state): State<AppState>,
) -> String {
    let mut engine = state.engine.lock().unwrap();

    let conditions = vec![
        Condition::Compare {
            attr: "id".to_string(),
            value: Value::Int(id),
            op: Operator::Eq,
        }
    ];

    match engine.delete(&table_name, conditions) {
        Ok(count) => format!("{} row(s) deleted", count),
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
        .route("/tables", get(list_table).post(create_table))
        .route("/tables/{name}", delete(drop_table))
        .route(
            "/tables/{name}/rows", 
            get(select_rows)
                .post(insert_row)
                .put(update_row),
        )
        .route(
            "/tables/{name}/rows/{id}",
            delete(delete_row),
        )
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .unwrap();

    println!("DB Service running at http://localhost:3000");

    axum::serve(listener, app)
        .await
        .unwrap();
}