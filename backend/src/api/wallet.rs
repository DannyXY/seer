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
    let mut summary = state
        .services
        .wallet
        .summary(provider, &address)
        .await
        .map_err(|err| ApiError::Service(err.to_string()))?;

    // Prepend live MNT balance fetched directly from Mantle RPC.
    if let Ok((_, mnt_amount)) = state.services.contracts.get_native_balance(&address).await {
        let mnt_position = crate::models::provider::PortfolioPosition {
            symbol: "MNT".to_string(),
            amount: format!("{:.6}", mnt_amount),
            usd_value: 0.0,
            protocol: None,
        };
        summary.balances.insert(0, mnt_position);
    }

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
