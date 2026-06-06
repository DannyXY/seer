use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use serde_json::{json, Value};

use crate::{api::auth_guard::require_wallet, errors::ApiError, AppState};

pub async fn get(
    State(state): State<AppState>,
    Path(address): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let provider = state.services.provider.provider().await;
    let identity = state
        .services
        .identity
        .generate(provider, &address)
        .await
        .map_err(|err| ApiError::Service(err.to_string()))?;
    Ok(Json(json!(identity)))
}

pub async fn generate(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(address): Path<String>,
) -> Result<Json<Value>, ApiError> {
    require_wallet(&state, &headers, &address)?;
    let provider = state.services.provider.provider().await;
    let identity = state
        .services
        .identity
        .generate(provider, &address)
        .await
        .map_err(|err| ApiError::Service(err.to_string()))?;
    Ok(Json(json!(identity)))
}

pub async fn mint_metadata(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(address): Path<String>,
) -> Result<Json<Value>, ApiError> {
    require_wallet(&state, &headers, &address)?;
    let provider = state.services.provider.provider().await;
    let identity = state
        .services
        .identity
        .generate(provider, &address)
        .await
        .map_err(|err| ApiError::Service(err.to_string()))?;
    Ok(Json(json!({
        "wallet_address": address,
        "metadata": {
            "name": format!("Seer Identity {:?}", identity.archetype),
            "description": "Non-transferable Seer portfolio identity metadata.",
            "attributes": identity.stats,
            "insights": identity.insights
        },
        "contract_configured": state.services.contracts.is_configured()
    })))
}
