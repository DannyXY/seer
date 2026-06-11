use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use serde_json::{json, Value};

use crate::{
    api::auth_guard::require_wallet,
    db::{load_user_settings, persist_user_settings},
    errors::ApiError,
    models::settings::UserSettings,
    AppState,
};

pub async fn get(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(address): Path<String>,
) -> Result<Json<Value>, ApiError> {
    require_wallet(&state, &headers, &address)?;

    // Check in-memory first; on miss load from DB and seed in-memory.
    let in_mem = state.services.settings.get(&address);
    let settings = if in_mem == UserSettings::default() {
        // May just be the default — try DB to see if user has saved settings.
        match load_user_settings(state.services.infra.postgres.as_ref(), &address).await {
            Ok(Some(db_settings)) => {
                state.services.settings.save(&address, db_settings.clone());
                db_settings
            }
            _ => in_mem,
        }
    } else {
        in_mem
    };

    Ok(Json(json!({
        "wallet_address": address,
        "settings": settings
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

    // Persist to DB so settings survive server restarts.
    let _ =
        persist_user_settings(state.services.infra.postgres.as_ref(), &address, &settings).await;

    Ok(Json(json!({
        "wallet_address": address,
        "settings": settings
    })))
}
