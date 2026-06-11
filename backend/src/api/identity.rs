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
    let onchain_tx_count = state
        .services
        .contracts
        .get_transaction_count(&address)
        .await
        .ok();
    let onchain_portfolio = state.services.contracts.get_portfolio(&address).await;
    let mut identity = state
        .services
        .identity
        .generate(
            provider,
            &address,
            onchain_tx_count,
            Some(&onchain_portfolio),
        )
        .await
        .map_err(|err| ApiError::Service(err.to_string()))?;

    // Hydrate on-chain mint status so the frontend shows the correct minted state.
    if state.services.contracts.rpc_configured() {
        if let Ok(token_id) = state
            .services
            .contracts
            .identity_token_of_owner(&address)
            .await
        {
            if token_id > 0 {
                identity.sbt_token_id = Some(token_id);
            }
        }
    }

    Ok(Json(json!(identity)))
}

pub async fn generate(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(address): Path<String>,
) -> Result<Json<Value>, ApiError> {
    require_wallet(&state, &headers, &address)?;
    let provider = state.services.provider.provider().await;
    let onchain_tx_count = state
        .services
        .contracts
        .get_transaction_count(&address)
        .await
        .ok();
    let onchain_portfolio = state.services.contracts.get_portfolio(&address).await;
    let mut identity = state
        .services
        .identity
        .generate(
            provider,
            &address,
            onchain_tx_count,
            Some(&onchain_portfolio),
        )
        .await
        .map_err(|err| ApiError::Service(err.to_string()))?;

    if state.services.contracts.rpc_configured() {
        if let Ok(token_id) = state
            .services
            .contracts
            .identity_token_of_owner(&address)
            .await
        {
            if token_id > 0 {
                identity.sbt_token_id = Some(token_id);
            }
        }
    }

    Ok(Json(json!(identity)))
}

pub async fn mint_metadata(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(address): Path<String>,
) -> Result<Json<Value>, ApiError> {
    require_wallet(&state, &headers, &address)?;
    let provider = state.services.provider.provider().await;
    let onchain_tx_count = state
        .services
        .contracts
        .get_transaction_count(&address)
        .await
        .ok();
    let onchain_portfolio = state.services.contracts.get_portfolio(&address).await;
    let identity = state
        .services
        .identity
        .generate(
            provider,
            &address,
            onchain_tx_count,
            Some(&onchain_portfolio),
        )
        .await
        .map_err(|err| ApiError::Service(err.to_string()))?;

    let metadata_name = format!("Seer Identity — {:?}", identity.archetype);
    let metadata_uri = format!(
        "data:application/json,{{\"name\":{:?},\"description\":\"Non-transferable Seer portfolio identity.\",\"attributes\":{}}}",
        metadata_name,
        identity.stats,
    );

    if !state.services.contracts.is_configured() {
        return Ok(Json(json!({
            "wallet_address": address,
            "minted": false,
            "contract_configured": false,
            "metadata": {
                "name": metadata_name,
                "description": "Non-transferable Seer portfolio identity metadata.",
                "attributes": identity.stats,
                "insights": identity.insights,
            }
        })));
    }

    // Check if already minted
    if let Ok(token_id) = state
        .services
        .contracts
        .identity_token_of_owner(&address)
        .await
    {
        if token_id > 0 {
            return Ok(Json(json!({
                "wallet_address": address,
                "minted": true,
                "token_id": token_id,
                "contract_configured": true,
            })));
        }
    }

    // Actually mint on-chain — backend signer pays gas
    let token_id = state
        .services
        .contracts
        .mint_identity_on_chain(&address, &metadata_uri)
        .await
        .map_err(|err| ApiError::Service(format!("on-chain mint failed: {err}")))?;

    Ok(Json(json!({
        "wallet_address": address,
        "minted": true,
        "token_id": token_id,
        "contract_configured": true,
        "metadata": {
            "name": metadata_name,
            "description": "Non-transferable Seer portfolio identity.",
            "attributes": identity.stats,
            "insights": identity.insights,
        }
    })))
}
