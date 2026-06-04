use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::{
    api::auth_guard::require_wallet, errors::ApiError, models::arena::ArenaEntryRequest, AppState,
};

pub async fn predictions(State(state): State<AppState>) -> Json<Value> {
    Json(json!({ "predictions": state.services.arena.predictions() }))
}

pub async fn prediction(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>, ApiError> {
    let prediction = state
        .services
        .arena
        .get_prediction(id)
        .ok_or(ApiError::NotFound)?;
    Ok(Json(json!(prediction)))
}

pub async fn enter(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(entry): Json<ArenaEntryRequest>,
) -> Result<Json<Value>, ApiError> {
    require_wallet(&state, &headers, &entry.wallet_address)?;
    let entry = state
        .services
        .arena
        .enter_prediction(id, entry)
        .map_err(|err| ApiError::BadRequest(err.to_string()))?;
    Ok(Json(json!({
        "prediction_id": id,
        "entry": entry,
        "status": "active",
        "contract_configured": state.services.contracts.is_configured()
    })))
}

pub async fn entries(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(address): Path<String>,
) -> Result<Json<Value>, ApiError> {
    require_wallet(&state, &headers, &address)?;
    Ok(Json(json!({
        "wallet_address": address,
        "entries": state.services.arena.entries_for_wallet(&address)
    })))
}

pub async fn leaderboard(State(state): State<AppState>) -> Json<Value> {
    Json(json!({
        "leaderboard": state.services.arena.leaderboard()
    }))
}

pub async fn seer_record() -> Json<Value> {
    Json(json!({ "accuracy_rate": 0.68, "resolved_predictions": 25, "current_streak": 3 }))
}

pub async fn resolve_due(State(state): State<AppState>) -> Json<Value> {
    Json(json!({
        "resolved": [],
        "contract_configured": state.services.contracts.is_configured(),
        "note": "Resolution job is scaffolded; production path evaluates metric facts before contract settlement."
    }))
}
