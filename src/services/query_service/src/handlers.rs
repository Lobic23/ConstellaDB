use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use cmd_module::{execute, parse_cmd, CmdError};
use super::state::AppState;

#[derive(Deserialize)]
pub struct QueryRequest {
    pub query: String,
}

#[derive(Serialize)]
pub struct QueryResponse {
    pub success: bool,
    pub message: String,
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
                message: e.to_string(),
            }));
        }
    };

    let result = {
        let mut engine = state.engine.lock().unwrap();
        execute(&mut engine, cmd)
    };

    let success = result.starts_with("OK");
    let status = if success { StatusCode::OK } else { StatusCode::BAD_REQUEST };
    (status, Json(QueryResponse { success, message: result }))
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
