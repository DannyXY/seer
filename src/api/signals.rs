use axum::{
    extract::{Path, State},
    Json,
};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::{db::persist_signals, errors::ApiError, AppState};

pub async fn list(State(state): State<AppState>) -> Result<Json<Value>, ApiError> {
    let provider = state.services.provider.provider().await;
    let mut signals = state
        .services
        .signals
        .generate(provider)
        .await
        .map_err(|err| ApiError::Service(err.to_string()))?;

    for signal in &mut signals {
        signal.explanation = state
            .services
            .claude
            .explain_signal(signal)
            .await
            .map_err(|err| ApiError::Service(err.to_string()))?;
    }

    if let Err(err) = persist_signals(state.services.infra.postgres.as_ref(), &signals).await {
        tracing::warn!(error = %err, "failed to persist generated signal snapshots");
    }

    Ok(Json(json!({ "signals": signals })))
}

pub async fn get(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>, ApiError> {
    let provider = state.services.provider.provider().await;
    let signals = state
        .services
        .signals
        .generate(provider)
        .await
        .map_err(|err| ApiError::Service(err.to_string()))?;
    let signal = signals
        .into_iter()
        .find(|signal| signal.id == id)
        .ok_or(ApiError::NotFound)?;
    Ok(Json(json!(signal)))
}
