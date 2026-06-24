use axum::{
    extract::{Path, State},
    routing::{delete, get, post, put},
    Json,
    Router,
};

use serde::Deserialize;
use serde_json::Value as JsonValue;
use std::collections::HashMap;

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
struct CreateAttrRequest {
    name: String,
    data_type: String,
}

#[derive(Deserialize)]
struct CreateTableRequest {
    name: String,
    attrs: Vec<CreateAttrRequest>,
}

#[derive(Deserialize)]
struct UpdateRequest {
    conditions: HashMap<String, JsonValue>,
    updates: HashMap<String, JsonValue>,
}

async fn health() -> &'static str {
    "DB Service Running"
}

async fn create_table(
    State(state): State<AppState>,
    Json(req): Json<CreateTableRequest>,
) -> String {
    let mut engine = state.engine.lock().unwrap();

    let attrs: Vec<Attr> = req
        .attrs.into_iter()
        .map(|a| {
            let data_type = match a.data_type.to_uppercase().as_str() {
                "INT" => Type::Int,
                "STRING" => Type::VarChar(255),
                _ => Type::VarChar(255),
            };

            Attr {
                name: a.name,
                data_type,
            }
        })
        .collect();

    let table = Table {
        name: req.name,
        attrs,
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
    Json(payload): Json<HashMap<String,JsonValue>>,
) -> String {
    let mut data = Vec::new();

    for (name,value) in payload {
        let value = match json_to_db_value(value) {
            Ok(v) => v,
            Err(e) => return e,
        };

        data.push(Data { 
            name, 
            value 
        });  
    }

    let entity = Entity {
        of: table_name,
        data,
    };

    let mut engine = state.engine.lock().unwrap();

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
    let mut updates = Vec::new();
    
    for (name, value) in req.updates {
        let value = match json_to_db_value(value) {
            Ok(v) => v,
            Err(e) => return e,
        };

        updates.push(Data {
            name,
            value,
        });
    }

    let mut conditions = Vec::new();

    for (attr, value) in req.conditions {
        let value = match json_to_db_value(value) {
            Ok(v) => v,
            Err(e) => return e,
        };

        conditions.push(
            Condition::Compare {
                attr,
                value,
                op: Operator::Eq,
            }
        );
    }

    let mut engine = state.engine.lock().unwrap();

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
    Path(table_name): Path<String>,
    State(state): State<AppState>,
    Json(payload): Json<HashMap<String, JsonValue>>,
) -> String {
    let mut conditions = Vec::new();

    for (attr, value) in payload {
        let value = match json_to_db_value(value) {
            Ok(v) => v,
            Err(e) => return e,
        };

        conditions.push(
            Condition::Compare {
                attr,
                value,
                op: Operator::Eq,
            }
        );
    }

    let mut engine = state.engine.lock().unwrap();

    match engine.delete(&table_name, conditions) {
        Ok(count) => format!("{} row(s) deleted", count),
        Err(e) => e,
    }
}

fn json_to_db_value(value: JsonValue) -> Result<Value, String> {
    match value {
        JsonValue::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(Value::Int(i as i32))
            } else {
                Err("Invalid integer".to_string())
            }
        }

        JsonValue::String(s) => {
            Ok(Value::VarChar(s))
        }

        _ => Err("Unsupprted data formats".to_string()),
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
                .put(update_row)
                .delete(delete_row),
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

