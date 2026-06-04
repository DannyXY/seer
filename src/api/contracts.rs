use axum::{extract::State, http::HeaderMap, Json};
use serde_json::{json, Value};

use crate::{
    errors::ApiError,
    models::execution::{
        Erc20AllowanceRequest, SendRawTransactionRequest, SendUserOperationRequest,
        UserOperationReceiptRequest,
    },
    AppState,
};

pub async fn readiness(State(state): State<AppState>) -> Result<Json<Value>, ApiError> {
    let observed_chain_id = state
        .services
        .contracts
        .chain_id()
        .await
        .map_err(|err| ApiError::Service(err.to_string()))?;

    Ok(Json(json!({
        "rpc_configured": state.services.contracts.rpc_configured(),
        "backend_signer_configured": state.services.contracts.is_configured(),
        "expected_chain_id": state.settings.mantle_chain_id,
        "observed_chain_id": observed_chain_id,
        "ready_for_signed_relay": state.services.contracts.rpc_configured(),
        "aa_bundler_configured": state.services.contracts.bundler_configured(),
        "ready_for_user_operation_relay": state.services.contracts.bundler_configured(),
        "ready_for_backend_signed_actions": state.services.contracts.is_configured()
            && observed_chain_id == Some(state.settings.mantle_chain_id)
    })))
}

pub async fn send_raw_transaction(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<SendRawTransactionRequest>,
) -> Result<Json<Value>, ApiError> {
    let token = headers
        .get("authorization")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .ok_or_else(|| ApiError::Unauthorized("missing Authorization bearer token".to_string()))?;
    state
        .services
        .auth
        .session_for_token(token)
        .ok_or_else(|| {
            ApiError::Unauthorized("missing, invalid, or expired session".to_string())
        })?;

    let response = state
        .services
        .contracts
        .send_raw_transaction(request)
        .await
        .map_err(|err| ApiError::BadRequest(err.to_string()))?;

    Ok(Json(json!(response)))
}

pub async fn send_user_operation(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<SendUserOperationRequest>,
) -> Result<Json<Value>, ApiError> {
    require_any_session(&state, &headers)?;
    let response = state
        .services
        .contracts
        .send_user_operation(request)
        .await
        .map_err(|err| ApiError::BadRequest(err.to_string()))?;

    Ok(Json(json!(response)))
}

pub async fn user_operation_receipt(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<UserOperationReceiptRequest>,
) -> Result<Json<Value>, ApiError> {
    require_any_session(&state, &headers)?;
    let receipt = state
        .services
        .contracts
        .user_operation_receipt(request)
        .await
        .map_err(|err| ApiError::BadRequest(err.to_string()))?;

    Ok(Json(json!({ "receipt": receipt })))
}

pub async fn erc20_allowance(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<Erc20AllowanceRequest>,
) -> Result<Json<Value>, ApiError> {
    require_any_session(&state, &headers)?;
    let response = state
        .services
        .contracts
        .erc20_allowance(request)
        .await
        .map_err(|err| ApiError::BadRequest(err.to_string()))?;

    Ok(Json(json!(response)))
}

fn require_any_session(state: &AppState, headers: &HeaderMap) -> Result<(), ApiError> {
    let token = headers
        .get("authorization")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .ok_or_else(|| ApiError::Unauthorized("missing Authorization bearer token".to_string()))?;
    state
        .services
        .auth
        .session_for_token(token)
        .ok_or_else(|| {
            ApiError::Unauthorized("missing, invalid, or expired session".to_string())
        })?;
    Ok(())
}
