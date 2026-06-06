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
    let predictions = state
        .services
        .arena
        .predictions()
        .into_iter()
        .map(|prediction| {
            json!({
                "claim": prediction.claim,
                "comparison_operator": prediction.comparison_operator,
                "created_at": prediction.created_at,
                "expiry_time": prediction.expiry_time,
                "final_value": prediction.final_value,
                "id": prediction.id,
                "metric": prediction.metric,
                "onchain_prediction_id": prediction.onchain_prediction_id,
                "pool_points": state.services.arena.prediction_pool(prediction.id),
                "reasoning": prediction.reasoning,
                "result": prediction.result,
                "seer_confidence": prediction.seer_confidence,
                "seer_position": prediction.seer_position,
                "status": prediction.status,
                "target_value": prediction.target_value
            })
        })
        .collect::<Vec<_>>();
    Json(json!({ "predictions": predictions }))
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
    let user_points = state.services.arena.user_points(&entry.wallet_address);
    Ok(Json(json!({
        "prediction_id": id,
        "entry": entry,
        "user_points": user_points,
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
        "entries": state.services.arena.entries_for_wallet(&address),
        "user_points": state.services.arena.user_points(&address)
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
