use axum::{
    extract::{Path, State},
    routing::{delete, get},
    Json,
    Router,
};

use clap::Parser;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::Mutex;

use constella_db::modules::db::{
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

#[derive(Parser, Debug)]
struct Args {
  #[arg(short, long)]
  port: Option<u32>,
}

/// Gets the local ip of the machine
pub fn get_local_ip() -> std::io::Result<std::net::IpAddr> {
  let socket = std::net::UdpSocket::bind("0.0.0.0:0")?;
  socket.connect("8.8.8.8:80")?;
  Ok(socket.local_addr()?.ip())
}

pub enum ExecuteResult {
    Ok(String),
    Error(String),
    Rows(Vec<Entity>),
}

#[derive(Serialize)]
struct ApiResponse {
    success: bool,
    message: Option<String>,
    rows: Option<Vec<Entity>>,
}

fn map_response(result: ExecuteResult) -> Json<ApiResponse> {
    match result {
        ExecuteResult::Ok(msg) => Json(ApiResponse {
            success: true,
            message: Some(msg),
            rows: None,
        }),

        ExecuteResult::Error(msg) => Json(ApiResponse {
            success: false,
            message: Some(msg),
            rows: None,
        }),

        ExecuteResult::Rows(rows) => Json(ApiResponse {
            success: true,
            message: None,
            rows: Some(rows),
        }),
    }
}

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
) -> Json<ApiResponse> {
    let mut engine = state.engine.lock().await;

    let attrs = req.attrs
        .into_iter()
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

    let result = match engine.create_table(&Table {
        name: req.name,
        attrs,
    }).await {
        Ok(_) => ExecuteResult::Ok("Table created successfully".into()),
        Err(e) => ExecuteResult::Error(e),
    };

    map_response(result)
}

async fn list_table(
    State(state): State<AppState>,
) -> Json<Vec<Table>> {
    let engine = state.engine.lock().await;
    Json(engine.get_tables())
}

async fn drop_table(
    Path(table_name): Path<String>,
    State(state): State<AppState>,
) -> Json<ApiResponse> {
    let mut engine = state.engine.lock().await;

    let result = match engine.drop_table(&table_name).await {
        Ok(_) => {
            ExecuteResult::Ok(format!("Table '{}' deleted", table_name))
        }
        Err(e) => ExecuteResult::Error(e),
    };

    map_response(result)
}

async fn insert_row(
    Path(table_name): Path<String>,
    State(state): State<AppState>,
    Json(payload): Json<HashMap<String, JsonValue>>,
) -> Json<ApiResponse> {
    let mut data = Vec::new();

    for (name, value) in payload {
        let value = match json_to_db_value(value) {
            Ok(v) => v,
            Err(e) => return map_response(ExecuteResult::Error(e)),
        };

        data.push(Data { name, value });
    }

    let entity = Entity {
        of: table_name,
        data,
    };

    let mut engine = state.engine.lock().await;

    let result = match engine.insert(&entity).await {
        Ok(_) => ExecuteResult::Ok("Row inserted".into()),
        Err(e) => ExecuteResult::Error(e),
    };

    map_response(result)
}

async fn select_rows(
    Path(table_name): Path<String>,
    State(state): State<AppState>,
) -> Json<ApiResponse> {
    let mut engine = state.engine.lock().await;

    let result = match engine.select(
        &table_name,
        vec!["*"],
        vec![],
    ).await {
        Ok(rows) => ExecuteResult::Rows(rows),
        Err(e) => ExecuteResult::Error(e),
    };

    map_response(result)
}

async fn update_row(
    Path(table_name): Path<String>,
    State(state): State<AppState>,
    Json(req): Json<UpdateRequest>,
) -> Json<ApiResponse> {
    let mut updates = Vec::new();

    for (name, value) in req.updates {
        let value = match json_to_db_value(value) {
            Ok(v) => v,
            Err(e) => return map_response(ExecuteResult::Error(e)),
        };

        updates.push(Data { name, value });
    }

    let mut conditions = Vec::new();

    for (attr, value) in req.conditions {
        let value = match json_to_db_value(value) {
            Ok(v) => v,
            Err(e) => return map_response(ExecuteResult::Error(e)),
        };

        conditions.push(
            Condition::Compare {
                attr,
                value,
                op: Operator::Eq,
            }
        );
    }

    let mut engine = state.engine.lock().await;

    let result = match engine.update(
        &table_name,
        updates,
        conditions,
    ).await {
        Ok(count) => {
            ExecuteResult::Ok(
                format!("{} row(s) updated", count)
            )
        }
        Err(e) => ExecuteResult::Error(e),
    };

    map_response(result)
}

async fn delete_row(
    Path(table_name): Path<String>,
    State(state): State<AppState>,
    Json(payload): Json<HashMap<String, JsonValue>>,
) -> Json<ApiResponse> {
    let mut conditions = Vec::new();

    for (attr, value) in payload {
        let value = match json_to_db_value(value) {
            Ok(v) => v,
            Err(e) => return map_response(ExecuteResult::Error(e)),
        };

        conditions.push(
            Condition::Compare {
                attr,
                value,
                op: Operator::Eq,
            }
        );
    }

    let mut engine = state.engine.lock().await;

    let result = match engine.delete(
        &table_name,
        conditions,
    ).await {
        Ok(count) => {
            ExecuteResult::Ok(
                format!("{} row(s) deleted", count)
            )
        }
        Err(e) => ExecuteResult::Error(e),
    };

    map_response(result)
}

fn json_to_db_value(value: JsonValue) -> Result<Value, String> {
    match value {
        JsonValue::Number(n) => {
            match n.as_i64() {
                Some(i) => Ok(Value::Int(i as i32)),
                None => Err("Invalid integer".into()),
            }
        }
        JsonValue::String(s) => {
            Ok(Value::VarChar(s))
        }
        _ => Err("Unsupported data format".into()),
    }
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    
    let engine = Engine::new().await;
    
    let state = AppState {
        engine: Arc::new(Mutex::new(engine)),
    };

    // Extract port from args
    let port = args.port.unwrap_or(8080);

    let app = Router::new()
        .route("/", get(health))
        .route(
            "/tables",
            get(list_table)
                .post(create_table),
        )
        .route(
            "/tables/{name}",
            delete(drop_table),
        )
        .route(
            "/tables/{name}/rows",
            get(select_rows)
                .post(insert_row)
                .put(update_row)
                .delete(delete_row),
        )
        .with_state(state);

    let ip = get_local_ip().unwrap_or_else(|_| {
        "127.0.0.1".parse().unwrap()
    });

    let listener = TcpListener::bind(
        format!("{}:{}", ip, port)
    ).await.unwrap();
    
    let bound_port = listener.local_addr().unwrap().port();
    let full_ip = format!("{}:{}", ip, bound_port);
    println!("DB Service running at http://{}", full_ip);

    axum::serve(listener, app)
        .await
        .unwrap();
}
