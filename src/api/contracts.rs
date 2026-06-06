use axum::{extract::State, http::HeaderMap, Json};
use serde_json::{json, Value};

use crate::{
    errors::ApiError,
    models::execution::{
        Erc20AllowanceRequest, SendRawTransactionRequest, SendUserOperationRequest,
        SimulateTransactionRequest, UserOperationReceiptRequest,
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
    let safe_user_operation = safe_user_operation_readiness(&state, observed_chain_id);
    let protocol_swaps = protocol_swap_readiness(&state, observed_chain_id);

    Ok(Json(json!({
        "rpc_configured": state.services.contracts.rpc_configured(),
        "backend_signer_configured": state.services.contracts.is_configured(),
        "expected_chain_id": state.settings.mantle_chain_id,
        "observed_chain_id": observed_chain_id,
        "ready_for_signed_relay": state.services.contracts.rpc_configured(),
        "aa_provider_stack": state.services.contracts.aa_provider_stack.clone(),
        "aa_bundler_configured": state.services.contracts.bundler_configured(),
        "aa_entry_point_configured": state.services.contracts.entry_point_configured(),
        "aa_paymaster_configured": state.services.contracts.paymaster_configured(),
        "ready_for_user_operation_relay": state.services.contracts.bundler_configured()
            && state.services.contracts.entry_point_configured(),
        "ready_for_backend_signed_actions": state.services.contracts.is_configured()
            && observed_chain_id == Some(state.settings.mantle_chain_id),
        "live_validation": {
            "safe_user_operation": safe_user_operation,
            "protocol_swaps": protocol_swaps
        }
    })))
}

fn safe_user_operation_readiness(state: &AppState, observed_chain_id: Option<u64>) -> Value {
    let missing = missing_safe_user_operation_requirements(state, observed_chain_id);
    json!({
        "ready": missing.is_empty(),
        "provider_stack": state.settings.aa_provider_stack.clone(),
        "missing": missing,
        "optional": {
            "AA_PAYMASTER_URL": state.services.contracts.paymaster_configured()
        },
        "next_step": "Build and sign a Safe ERC-4337 user operation with the selected provider stack, then submit it to /api/contracts/send-user-operation."
    })
}

fn protocol_swap_readiness(state: &AppState, observed_chain_id: Option<u64>) -> Value {
    let missing = missing_protocol_swap_requirements(state, observed_chain_id);
    json!({
        "ready": missing.is_empty(),
        "missing": missing,
        "supported_protocols": ["Agni Finance", "Merchant Moe", "Fluxion Network"],
        "next_step": "Submit swap intent via /api/agent/evaluate-intent, confirm quote via QuoterV2/LBQuoter, then execute via user operation or signed transaction."
    })
}

fn missing_safe_user_operation_requirements(
    state: &AppState,
    observed_chain_id: Option<u64>,
) -> Vec<&'static str> {
    let mut missing = Vec::new();
    if state.settings.aa_provider_stack.trim().is_empty() {
        missing.push("AA_PROVIDER_STACK");
    }
    if !state.services.contracts.bundler_configured() {
        missing.push("AA_BUNDLER_URL");
    }
    if !state.services.contracts.entry_point_configured() {
        missing.push("AA_ENTRY_POINT_ADDRESS");
    }
    if observed_chain_id != Some(state.settings.mantle_chain_id) {
        missing.push("MANTLE_RPC_URL matching MANTLE_CHAIN_ID");
    }
    missing
}

fn missing_protocol_swap_requirements(
    state: &AppState,
    observed_chain_id: Option<u64>,
) -> Vec<&'static str> {
    let mut missing = Vec::new();
    if observed_chain_id != Some(state.settings.mantle_chain_id) {
        missing.push("MANTLE_RPC_URL matching MANTLE_CHAIN_ID");
    }
    if state.settings.mantle_usdt_address.is_none() {
        missing.push("MANTLE_USDT_ADDRESS");
    }
    if state.settings.mantle_usdc_address.is_none() {
        missing.push("MANTLE_USDC_ADDRESS");
    }
    missing
}

pub async fn execution_readiness(State(state): State<AppState>) -> Result<Json<Value>, ApiError> {
    Ok(Json(json!(state.services.execution.readiness())))
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

pub async fn simulate_transaction(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<SimulateTransactionRequest>,
) -> Result<Json<Value>, ApiError> {
    require_any_session(&state, &headers)?;
    let response = state
        .services
        .contracts
        .simulate_transaction(request)
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
