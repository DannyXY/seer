use axum::{extract::State, Json};
use serde_json::{json, Value};

use crate::AppState;

pub async fn health(State(state): State<AppState>) -> Json<Value> {
    Json(json!({
        "status": "ok",
        "env": state.settings.app_env,
        "role": state.settings.app_role.as_str(),
        "provider": state.services.provider_name(),
        "infrastructure": state.services.infra.status(),
    }))
}

pub async fn version(State(state): State<AppState>) -> Json<Value> {
    Json(json!({
        "name": "seer-api",
        "version": state.settings.version,
    }))
}
