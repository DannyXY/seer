use axum::{
    extract::{Path, State},
    Json,
};
use serde_json::{json, Value};

use crate::{errors::ApiError, AppState};

pub async fn summary(
    State(state): State<AppState>,
    Path(address): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let provider = state.services.provider.provider().await;
    let summary = state
        .services
        .wallet
        .summary(provider, &address)
        .await
        .map_err(|err| ApiError::Service(err.to_string()))?;
    Ok(Json(json!(summary)))
}

pub async fn activity(
    State(state): State<AppState>,
    Path(address): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let provider = state.services.provider.provider().await;
    let activity = provider
        .get_wallet_transactions(&address)
        .await
        .map_err(|err| ApiError::Service(err.to_string()))?;
    Ok(Json(json!({ "address": address, "activity": activity })))
}

pub async fn risk(
    State(state): State<AppState>,
    Path(address): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let provider = state.services.provider.provider().await;
    let profile = provider
        .get_wallet_profile(&address)
        .await
        .map_err(|err| ApiError::Service(err.to_string()))?;
    Ok(Json(json!({
        "address": address,
        "risk_score": profile.risk_score,
        "risk_band": if profile.risk_score >= 75 { "high" } else if profile.risk_score >= 45 { "medium" } else { "low" }
    })))
}
