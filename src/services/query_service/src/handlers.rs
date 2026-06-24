use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use cmd_module::{ExecuteResult, execute, parse_cmd, CmdError};
use db_module::Entity;
use super::state::AppState;

#[derive(Deserialize)]
pub struct QueryRequest {
    pub query: String,
}

#[derive(Serialize)]
pub struct QueryResponse {
    pub success: bool,
    pub message: Option<String>,
    pub rows: Option<Vec<Entity>>,
}

pub async fn handle_query(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<QueryRequest>,
) -> impl IntoResponse {
    let cmd = match parse_cmd(&payload.query) {
        Ok(cmd) => cmd,
        Err(e) => {
            return (error_status(&e), Json(QueryResponse {
                success: false,
                message: Some(e.to_string()),
                rows: None,
            }));
        }
    };

    let result = {
        let mut engine = state.engine.lock().unwrap();
        execute(&mut engine, cmd)
    };

    match result {
        ExecuteResult::Ok(msg) => (
            StatusCode::OK,
            Json(QueryResponse {
                success: true,
                message: Some(msg),
                rows: None,
            }),
        ),

        ExecuteResult::Error(msg) => (
            StatusCode::BAD_REQUEST,
            Json(QueryResponse {
                success: false,
                message: Some(msg),
                rows: None,
            }),
        ),

        ExecuteResult::Rows(rows) => (
            StatusCode::OK,
            Json(QueryResponse {
                success: true,
                message: None,
                rows: Some(rows),
            }),
        ),
    }
}

pub async fn handle_health() -> impl IntoResponse {
    (StatusCode::OK, Json(serde_json::json!({ "status": "ok" })))
}

fn error_status(e: &CmdError) -> StatusCode {
    match e {
        CmdError::Empty | CmdError::Syntax(_) => StatusCode::BAD_REQUEST,
        CmdError::Unsupported(_) | CmdError::UnsupportedExpr(_) => StatusCode::NOT_IMPLEMENTED,
    }
}
