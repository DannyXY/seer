use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use ethers_core::types::U256;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::{
    api::auth_guard::require_wallet,
    db::{persist_agent_execution_log, persist_agent_execution_policy, persist_agent_intent},
    errors::ApiError,
    models::{
        agent::{
            CreateIntentRequest, CreateSessionPolicyRequest, EvaluateIntentWithAllowanceRequest,
            IntentStatus,
        },
        execution::{
            Erc20AllowanceRequest, ExecutionProposal, SimulateTransactionRequest,
            SimulateTransactionResponse, TransactionDraft,
        },
    },
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

pub async fn evaluate_intent_with_allowance(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<EvaluateIntentWithAllowanceRequest>,
) -> Result<Json<Value>, ApiError> {
    require_wallet(&state, &headers, &req.wallet_address)?;
    let parsed = state.services.agent.parse_intent(&req.raw_intent);
    let owner_address = req
        .owner_address
        .clone()
        .unwrap_or_else(|| req.wallet_address.clone());
    let derived_allowance_request = state
        .services
        .execution
        .allowance_request_for_intent(&parsed, &owner_address);
    let allowance_request = match derived_allowance_request {
        Some(derived) => {
            if let Some(token_address) = &req.token_address {
                if !token_address.eq_ignore_ascii_case(&derived.token_address) {
                    return Err(ApiError::BadRequest(
                        "token_address does not match configured intent asset".to_string(),
                    ));
                }
            }
            if let Some(spender_address) = &req.spender_address {
                if !spender_address.eq_ignore_ascii_case(&derived.spender_address) {
                    return Err(ApiError::BadRequest(
                        "spender_address does not match configured protocol spender".to_string(),
                    ));
                }
            }
            derived
        }
        None => Erc20AllowanceRequest {
            token_address: req.token_address.clone().ok_or_else(|| {
                ApiError::BadRequest(
                    "token_address is required when the intent asset is not configured".to_string(),
                )
            })?,
            owner_address,
            spender_address: req.spender_address.clone().ok_or_else(|| {
                ApiError::BadRequest(
                    "spender_address is required when the protocol spender is not configured"
                        .to_string(),
                )
            })?,
        },
    };
    let allowance = state
        .services
        .contracts
        .erc20_allowance(allowance_request)
        .await
        .map_err(|err| ApiError::BadRequest(err.to_string()))?;
    let allowance_value = parse_rpc_u256(&allowance.allowance)
        .map_err(|err| ApiError::BadRequest(format!("invalid allowance value: {err}")))?;
    let provider = state.services.provider.provider().await;
    let mut proposal = state
        .services
        .execution
        .evaluate_intent_with_allowance(
            provider,
            CreateIntentRequest {
                wallet_address: req.wallet_address.clone(),
                raw_intent: req.raw_intent.clone(),
            },
            parsed.clone(),
            Some(allowance_value),
        )
        .await
        .map_err(|err| ApiError::Service(err.to_string()))?;
    let simulation = simulate_transaction_draft(&state, &mut proposal).await?;

    Ok(Json(json!({
        "parsed_intent": parsed,
        "allowance": allowance,
        "simulation": simulation,
        "proposal": proposal
    })))
}

async fn simulate_transaction_draft(
    state: &AppState,
    proposal: &mut ExecutionProposal,
) -> Result<Option<SimulateTransactionResponse>, ApiError> {
    let Some(draft) = proposal.transaction_draft.as_ref() else {
        return Ok(None);
    };
    let Some(to) = draft.to.clone() else {
        return Ok(None);
    };
    let Some(data) = draft.data.clone() else {
        return Ok(None);
    };

    let simulation = state
        .services
        .contracts
        .simulate_transaction(SimulateTransactionRequest {
            from: Some(simulation_from_address(proposal)),
            to,
            value: Some(draft.value.clone()),
            data: Some(data),
        })
        .await
        .map_err(|err| ApiError::BadRequest(err.to_string()))?;

    if !simulation.success {
        proposal.transaction_draft = Some(simulation_failed_draft(
            proposal.chain_id,
            simulation.error.as_deref(),
        ));
    }

    Ok(Some(simulation))
}

fn simulation_from_address(proposal: &ExecutionProposal) -> String {
    proposal
        .allowance_check
        .as_ref()
        .map(|allowance| allowance.owner_address.clone())
        .unwrap_or_else(|| proposal.wallet_address.clone())
}

fn simulation_failed_draft(chain_id: u64, error: Option<&str>) -> TransactionDraft {
    let reason = error.unwrap_or("RPC did not provide a revert reason");
    TransactionDraft {
        kind: "simulation_failed".to_string(),
        to: None,
        value: "0".to_string(),
        data: None,
        chain_id,
        human_summary: format!(
            "Transaction simulation failed on Mantle RPC; Seer will not surface executable calldata until this is fixed. {reason}"
        ),
    }
}

pub async fn create_intent(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<CreateIntentRequest>,
) -> Result<Json<Value>, ApiError> {
    require_wallet(&state, &headers, &req.wallet_address)?;
    let intent = state.services.agent.create_intent(req);
    persist_agent_intent(state.services.infra.postgres.as_ref(), &intent)
        .await
        .map_err(|err| ApiError::Service(err.to_string()))?;
    let policy = state.services.agent.create_policy(&intent);
    persist_agent_execution_policy(state.services.infra.postgres.as_ref(), &policy)
        .await
        .map_err(|err| ApiError::Service(err.to_string()))?;
    Ok(Json(json!({
        "intent": intent,
        "execution_policy_draft": policy,
        "authorization_model": "user-approved or scoped delegated execution only"
    })))
}

fn parse_rpc_u256(value: &str) -> anyhow::Result<U256> {
    let trimmed = value.trim();
    let digits = trimmed.strip_prefix("0x").unwrap_or(trimmed);
    if digits.is_empty() {
        anyhow::bail!("empty value")
    }
    Ok(U256::from_str_radix(digits, 16)?)
}

#[cfg(test)]
mod tests {
    use super::{
        parse_rpc_u256, simulation_failed_draft, simulation_from_address,
        validate_session_policy_request,
    };
    use crate::models::agent::CreateSessionPolicyRequest;
    use crate::models::execution::{Erc20AllowanceRequest, ExecutionProposal};

    #[test]
    fn parses_rpc_hex_allowance() {
        assert_eq!(parse_rpc_u256("0x17d7840").unwrap().as_u64(), 25_000_000);
    }

    #[test]
    fn rejects_empty_rpc_allowance() {
        assert!(parse_rpc_u256("0x").is_err());
    }

    #[test]
    fn simulation_failed_draft_strips_executable_fields() {
        let draft = simulation_failed_draft(5003, Some("execution reverted"));

        assert_eq!(draft.kind, "simulation_failed");
        assert!(draft.to.is_none());
        assert!(draft.data.is_none());
        assert!(draft.human_summary.contains("execution reverted"));
    }

    #[test]
    fn simulation_uses_allowance_owner_when_available() {
        let proposal = ExecutionProposal {
            actionable: true,
            action: "accumulate".to_string(),
            wallet_address: "0x00000000000000000000000000000000000000aa".to_string(),
            chain_id: 5003,
            network: "mantle-testnet".to_string(),
            conditions: vec![],
            allowance_check: Some(Erc20AllowanceRequest {
                token_address: "0x0000000000000000000000000000000000000001".to_string(),
                owner_address: "0x00000000000000000000000000000000000000bb".to_string(),
                spender_address: "0x0000000000000000000000000000000000000002".to_string(),
            }),
            transaction_draft: None,
            required_authorization: "test".to_string(),
            protocol_operation: None,
        };

        assert_eq!(
            simulation_from_address(&proposal),
            "0x00000000000000000000000000000000000000bb"
        );
    }

    #[test]
    fn rejects_invalid_session_policy_addresses() {
        let request = CreateSessionPolicyRequest {
            smart_account_address: "0xsmartaccount".to_string(),
            session_key_address: "0x00000000000000000000000000000000000000cc".to_string(),
            allowed_assets: vec!["USDC".to_string()],
            allowed_protocols: vec!["mETH Protocol".to_string()],
            allowed_contracts: vec!["0x0000000000000000000000000000000000000002".to_string()],
            max_spend_usd: Some(25.0),
            max_transaction_count: Some(1),
            expires_in_days: Some(1),
        };

        assert!(validate_session_policy_request(&request).is_err());
    }
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
    persist_agent_intent(state.services.infra.postgres.as_ref(), &intent)
        .await
        .map_err(|err| ApiError::Service(err.to_string()))?;
    persist_agent_execution_log(state.services.infra.postgres.as_ref(), &execution_log)
        .await
        .map_err(|err| ApiError::Service(err.to_string()))?;

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
    validate_session_policy_request(&req)?;
    let policy = state.services.agent.create_session_policy(&intent, req);
    persist_agent_execution_policy(state.services.infra.postgres.as_ref(), &policy)
        .await
        .map_err(|err| ApiError::Service(err.to_string()))?;

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
    let execution_log =
        state
            .services
            .agent
            .record_execution_log_with_policy(&intent, Some(policy.id), proposal);
    persist_agent_execution_log(state.services.infra.postgres.as_ref(), &execution_log)
        .await
        .map_err(|err| ApiError::Service(err.to_string()))?;

    if result.executable {
        if let Some(updated_policy) = state.services.agent.mark_policy_used(policy.id) {
            persist_agent_execution_policy(state.services.infra.postgres.as_ref(), &updated_policy)
                .await
                .map_err(|err| ApiError::Service(err.to_string()))?;
        }
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
    persist_agent_execution_policy(state.services.infra.postgres.as_ref(), &revoked)
        .await
        .map_err(|err| ApiError::Service(err.to_string()))?;

    Ok(Json(json!({ "policy": revoked })))
}

fn validate_session_policy_request(req: &CreateSessionPolicyRequest) -> Result<(), ApiError> {
    validate_hex_address(&req.smart_account_address, "smart_account_address")?;
    validate_hex_address(&req.session_key_address, "session_key_address")?;
    for contract in &req.allowed_contracts {
        validate_hex_address(contract, "allowed_contracts[]")?;
    }
    if req.allowed_assets.is_empty() {
        return Err(ApiError::BadRequest(
            "allowed_assets must include at least one asset".to_string(),
        ));
    }
    if req.allowed_protocols.is_empty() {
        return Err(ApiError::BadRequest(
            "allowed_protocols must include at least one protocol".to_string(),
        ));
    }
    Ok(())
}

fn validate_hex_address(value: &str, name: &str) -> Result<(), ApiError> {
    let hex = value
        .strip_prefix("0x")
        .ok_or_else(|| ApiError::BadRequest(format!("{name} must be a 0x-prefixed address")))?;
    if hex.len() != 40 || !hex.chars().all(|character| character.is_ascii_hexdigit()) {
        return Err(ApiError::BadRequest(format!(
            "{name} must be a 0x-prefixed address"
        )));
    }
    Ok(())
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
    persist_agent_intent(state.services.infra.postgres.as_ref(), &intent)
        .await
        .map_err(|err| ApiError::Service(err.to_string()))?;
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
    persist_agent_intent(state.services.infra.postgres.as_ref(), &intent)
        .await
        .map_err(|err| ApiError::Service(err.to_string()))?;
    Ok(Json(json!({ "intent": intent })))
}
