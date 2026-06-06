use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use serde_json::{json, Value};

use crate::{
    api::auth_guard::require_wallet, errors::ApiError, models::settings::UserSettings, AppState,
};

pub async fn get(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(address): Path<String>,
) -> Result<Json<Value>, ApiError> {
    require_wallet(&state, &headers, &address)?;
    Ok(Json(json!({
        "wallet_address": address,
        "settings": state.services.settings.get(&address)
    })))
}

pub async fn save(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(address): Path<String>,
    Json(settings): Json<UserSettings>,
) -> Result<Json<Value>, ApiError> {
    require_wallet(&state, &headers, &address)?;
    let settings = state.services.settings.save(&address, settings);
    Ok(Json(json!({
        "wallet_address": address,
        "settings": settings
    })))
}
