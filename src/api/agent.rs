use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::{
    api::auth_guard::require_wallet,
    errors::ApiError,
    models::agent::{CreateIntentRequest, CreateSessionPolicyRequest, IntentStatus},
    AppState,
};

pub async fn parse_intent(
    State(state): State<AppState>,
    Json(req): Json<CreateIntentRequest>,
) -> Json<Value> {
    let parsed = state.services.agent.parse_intent(&req.raw_intent);
    let explanation = state
        .services
        .claude
        .parse_intent_explanation(&parsed)
        .await
        .unwrap_or_default();
    Json(
        json!({ "wallet_address": req.wallet_address, "parsed_intent": parsed, "explanation": explanation }),
    )
}

pub async fn evaluate_intent(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<CreateIntentRequest>,
) -> Result<Json<Value>, ApiError> {
    require_wallet(&state, &headers, &req.wallet_address)?;
    let parsed = state.services.agent.parse_intent(&req.raw_intent);
    let provider = state.services.provider.provider().await;
    let proposal = state
        .services
        .execution
        .evaluate_intent(provider, req, parsed)
        .await
        .map_err(|err| ApiError::Service(err.to_string()))?;

    Ok(Json(json!(proposal)))
}

pub async fn create_intent(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<CreateIntentRequest>,
) -> Result<Json<Value>, ApiError> {
    require_wallet(&state, &headers, &req.wallet_address)?;
    let intent = state.services.agent.create_intent(req);
    let policy = state.services.agent.create_policy(&intent);
    Ok(Json(json!({
        "intent": intent,
        "execution_policy_draft": policy,
        "authorization_model": "user-approved or scoped delegated execution only"
    })))
}

pub async fn intents(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(address): Path<String>,
) -> Result<Json<Value>, ApiError> {
    require_wallet(&state, &headers, &address)?;
    Ok(Json(json!({
        "wallet_address": address,
        "intents": state.services.agent.list_intents(&address)
    })))
}

pub async fn intent(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(intent_id): Path<Uuid>,
) -> Result<Json<Value>, ApiError> {
    let intent = state
        .services
        .agent
        .get_intent(intent_id)
        .ok_or(ApiError::NotFound)?;
    require_wallet(&state, &headers, &intent.wallet_address)?;
    Ok(Json(json!({
        "intent": intent,
        "execution_policies": state.services.agent.policies_for_intent(intent_id)
    })))
}

pub async fn reasoning(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(intent_id): Path<Uuid>,
) -> Result<Json<Value>, ApiError> {
    let intent = state
        .services
        .agent
        .get_intent(intent_id)
        .ok_or(ApiError::NotFound)?;
    require_wallet(&state, &headers, &intent.wallet_address)?;
    Ok(Json(json!({
        "intent_id": intent.id,
        "reasoning_logs": [{
            "action_type": "intent_created",
            "explanation": "Seer parsed the user intent, identified trigger mode and conditions, and drafted a scoped execution policy.",
            "reasoning_hash": intent.intent_hash
        }],
        "execution_logs": state.services.agent.execution_logs_for_intent(intent_id),
        "anchoring": "reasoning_hash and metadataURI are contract-ready"
    })))
}

pub async fn activate(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(intent_id): Path<Uuid>,
) -> Result<Json<Value>, ApiError> {
    let existing = state
        .services
        .agent
        .get_intent(intent_id)
        .ok_or(ApiError::NotFound)?;
    require_wallet(&state, &headers, &existing.wallet_address)?;
    let intent = state
        .services
        .agent
        .update_status(intent_id, IntentStatus::Active)
        .ok_or(ApiError::NotFound)?;

    let provider = state.services.provider.provider().await;
    let proposal = state
        .services
        .execution
        .evaluate_stored_intent(provider, &intent)
        .await
        .map_err(|err| ApiError::Service(err.to_string()))?;
    let execution_log = state.services.agent.record_execution_log(&intent, proposal);

    Ok(Json(json!({
        "intent": intent,
        "execution_log": execution_log,
        "status": "active"
    })))
}

pub async fn create_session_policy(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(intent_id): Path<Uuid>,
    Json(req): Json<CreateSessionPolicyRequest>,
) -> Result<Json<Value>, ApiError> {
    let intent = state
        .services
        .agent
        .get_intent(intent_id)
        .ok_or(ApiError::NotFound)?;
    require_wallet(&state, &headers, &intent.wallet_address)?;
    let policy = state.services.agent.create_session_policy(&intent, req);

    Ok(Json(json!({
        "policy": policy,
        "authorization_model": "smart-account session key",
        "note": "Policy is active in backend state. Production AA provider wiring should also register or validate this policy with the smart account."
    })))
}

pub async fn delegated_execute(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(intent_id): Path<Uuid>,
) -> Result<Json<Value>, ApiError> {
    let intent = state
        .services
        .agent
        .get_intent(intent_id)
        .ok_or(ApiError::NotFound)?;
    require_wallet(&state, &headers, &intent.wallet_address)?;
    let policy = state
        .services
        .agent
        .active_session_policy_for_intent(intent_id)
        .ok_or_else(|| ApiError::BadRequest("active session-key policy not found".to_string()))?;

    let provider = state.services.provider.provider().await;
    let proposal = state
        .services
        .execution
        .evaluate_stored_intent(provider, &intent)
        .await
        .map_err(|err| ApiError::Service(err.to_string()))?;
    let result =
        state
            .services
            .execution
            .build_delegated_execution(&intent, &policy, proposal.clone());
    let execution_log = state.services.agent.record_execution_log(&intent, proposal);

    if result.executable {
        state.services.agent.mark_policy_used(policy.id);
    }

    Ok(Json(json!({
        "delegated_execution": result,
        "execution_log": execution_log,
    })))
}

pub async fn revoke_policy(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(policy_id): Path<Uuid>,
) -> Result<Json<Value>, ApiError> {
    let policy = state
        .services
        .agent
        .get_policy(policy_id)
        .ok_or(ApiError::NotFound)?;
    require_wallet(&state, &headers, &policy.wallet_address)?;
    let revoked = state
        .services
        .agent
        .revoke_policy(policy_id)
        .ok_or(ApiError::NotFound)?;

    Ok(Json(json!({ "policy": revoked })))
}

pub async fn pause(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(intent_id): Path<Uuid>,
) -> Result<Json<Value>, ApiError> {
    let existing = state
        .services
        .agent
        .get_intent(intent_id)
        .ok_or(ApiError::NotFound)?;
    require_wallet(&state, &headers, &existing.wallet_address)?;
    let intent = state
        .services
        .agent
        .update_status(intent_id, IntentStatus::Paused)
        .ok_or(ApiError::NotFound)?;
    Ok(Json(json!({ "intent": intent })))
}

pub async fn stop(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(intent_id): Path<Uuid>,
) -> Result<Json<Value>, ApiError> {
    let existing = state
        .services
        .agent
        .get_intent(intent_id)
        .ok_or(ApiError::NotFound)?;
    require_wallet(&state, &headers, &existing.wallet_address)?;
    let intent = state
        .services
        .agent
        .update_status(intent_id, IntentStatus::Cancelled)
        .ok_or(ApiError::NotFound)?;
    Ok(Json(json!({ "intent": intent })))
}
