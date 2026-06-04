use std::collections::HashMap;
use std::sync::RwLock;

use chrono::{Duration, Utc};
use serde_json::json;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::models::agent::{
    AgentExecutionLog, AgentIntent, CreateIntentRequest, CreateSessionPolicyRequest,
    ExecutionCondition, ExecutionPolicy, IntentExecutionMode, IntentStatus, IntentTrigger,
    ParsedIntent, SpendAmount,
};
use crate::models::execution::ExecutionProposal;

#[derive(Default)]
pub struct AgentService {
    intents: RwLock<HashMap<Uuid, AgentIntent>>,
    policies: RwLock<HashMap<Uuid, ExecutionPolicy>>,
    execution_logs: RwLock<HashMap<Uuid, Vec<AgentExecutionLog>>>,
}

impl AgentService {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn parse_intent(&self, raw: &str) -> ParsedIntent {
        let normalized = raw.to_lowercase();
        let has_schedule = normalized.contains("daily")
            || normalized.contains("weekly")
            || normalized.contains("recurring")
            || normalized.contains("recurrent")
            || normalized.contains("every ");
        let has_condition = normalized.contains("when ")
            || normalized.contains("if ")
            || normalized.contains("tvl")
            || normalized.contains("risk")
            || normalized.contains("apy");

        let mode = match (has_schedule, has_condition) {
            (false, false) => IntentExecutionMode::Instant,
            (true, false) => IntentExecutionMode::Recurring,
            (false, true) => IntentExecutionMode::Conditional,
            (true, true) => IntentExecutionMode::RecurringConditional,
        };

        let conditions = infer_conditions(&normalized);

        ParsedIntent {
            action: infer_action(&normalized),
            target_assets: infer_assets(&normalized),
            target_protocols: infer_protocols(&normalized),
            spend_amount: infer_spend_amount(&normalized),
            trigger: IntentTrigger {
                mode,
                schedule: has_schedule.then(|| "parsed natural-language schedule".to_string()),
                conditions,
            },
            constraints: vec![
                "requires scoped execution policy for automation".to_string(),
                "requires user signature unless delegated permission exists".to_string(),
            ],
            requires_user_signature: true,
        }
    }

    pub fn create_intent(&self, req: CreateIntentRequest) -> AgentIntent {
        let parsed = self.parse_intent(&req.raw_intent);
        let intent_hash = hash_json(&json!({
            "wallet_address": req.wallet_address,
            "raw_intent": req.raw_intent,
            "parsed_intent": parsed,
        }));

        let intent = AgentIntent {
            id: Uuid::new_v4(),
            wallet_address: req.wallet_address,
            raw_intent: req.raw_intent,
            parsed_intent: parsed,
            status: IntentStatus::Draft,
            intent_hash,
            onchain_intent_id: None,
            created_at: Utc::now(),
        };

        self.intents
            .write()
            .expect("agent intent store poisoned")
            .insert(intent.id, intent.clone());

        intent
    }

    pub fn create_policy(&self, intent: &AgentIntent) -> ExecutionPolicy {
        let policy_hash = hash_json(&json!({
            "intent_id": intent.id,
            "wallet_address": intent.wallet_address,
            "allowed_assets": intent.parsed_intent.target_assets,
            "allowed_protocols": intent.parsed_intent.target_protocols,
            "expires_at": "30d"
        }));

        let policy = ExecutionPolicy {
            id: Uuid::new_v4(),
            intent_id: intent.id,
            wallet_address: intent.wallet_address.clone(),
            smart_account_address: None,
            session_key_address: None,
            allowed_assets: intent.parsed_intent.target_assets.clone(),
            allowed_protocols: intent.parsed_intent.target_protocols.clone(),
            allowed_contracts: Vec::new(),
            max_spend_usd: Some(100.0),
            max_transaction_count: Some(10),
            transactions_used: 0,
            revoked_at: None,
            expires_at: Utc::now() + Duration::days(30),
            status: IntentStatus::Draft,
            policy_hash,
        };

        self.policies
            .write()
            .expect("agent policy store poisoned")
            .insert(policy.id, policy.clone());

        policy
    }

    pub fn create_session_policy(
        &self,
        intent: &AgentIntent,
        request: CreateSessionPolicyRequest,
    ) -> ExecutionPolicy {
        let expires_at = Utc::now() + Duration::days(request.expires_in_days.unwrap_or(30));
        let policy_hash = hash_json(&json!({
            "intent_id": intent.id,
            "wallet_address": intent.wallet_address,
            "smart_account_address": request.smart_account_address,
            "session_key_address": request.session_key_address,
            "allowed_assets": request.allowed_assets,
            "allowed_protocols": request.allowed_protocols,
            "allowed_contracts": request.allowed_contracts,
            "max_spend_usd": request.max_spend_usd,
            "max_transaction_count": request.max_transaction_count,
            "expires_at": expires_at,
        }));

        let policy = ExecutionPolicy {
            id: Uuid::new_v4(),
            intent_id: intent.id,
            wallet_address: intent.wallet_address.clone(),
            smart_account_address: Some(request.smart_account_address),
            session_key_address: Some(request.session_key_address),
            allowed_assets: request.allowed_assets,
            allowed_protocols: request.allowed_protocols,
            allowed_contracts: request.allowed_contracts,
            max_spend_usd: request.max_spend_usd,
            max_transaction_count: request.max_transaction_count,
            transactions_used: 0,
            revoked_at: None,
            expires_at,
            status: IntentStatus::Active,
            policy_hash,
        };

        self.policies
            .write()
            .expect("agent policy store poisoned")
            .insert(policy.id, policy.clone());

        policy
    }

    pub fn list_intents(&self, wallet_address: &str) -> Vec<AgentIntent> {
        self.intents
            .read()
            .expect("agent intent store poisoned")
            .values()
            .filter(|intent| intent.wallet_address.eq_ignore_ascii_case(wallet_address))
            .cloned()
            .collect()
    }

    pub fn get_intent(&self, intent_id: Uuid) -> Option<AgentIntent> {
        self.intents
            .read()
            .expect("agent intent store poisoned")
            .get(&intent_id)
            .cloned()
    }

    pub fn update_status(&self, intent_id: Uuid, status: IntentStatus) -> Option<AgentIntent> {
        let mut intents = self.intents.write().expect("agent intent store poisoned");
        let intent = intents.get_mut(&intent_id)?;
        intent.status = status;
        Some(intent.clone())
    }

    pub fn policies_for_intent(&self, intent_id: Uuid) -> Vec<ExecutionPolicy> {
        self.policies
            .read()
            .expect("agent policy store poisoned")
            .values()
            .filter(|policy| policy.intent_id == intent_id)
            .cloned()
            .collect()
    }

    pub fn active_session_policy_for_intent(&self, intent_id: Uuid) -> Option<ExecutionPolicy> {
        self.policies
            .read()
            .expect("agent policy store poisoned")
            .values()
            .find(|policy| {
                policy.intent_id == intent_id
                    && matches!(policy.status, IntentStatus::Active)
                    && policy.revoked_at.is_none()
                    && policy.session_key_address.is_some()
                    && policy.smart_account_address.is_some()
                    && policy.expires_at > Utc::now()
            })
            .cloned()
    }

    pub fn get_policy(&self, policy_id: Uuid) -> Option<ExecutionPolicy> {
        self.policies
            .read()
            .expect("agent policy store poisoned")
            .get(&policy_id)
            .cloned()
    }

    pub fn revoke_policy(&self, policy_id: Uuid) -> Option<ExecutionPolicy> {
        let mut policies = self.policies.write().expect("agent policy store poisoned");
        let policy = policies.get_mut(&policy_id)?;
        policy.status = IntentStatus::Cancelled;
        policy.revoked_at = Some(Utc::now());
        Some(policy.clone())
    }

    pub fn mark_policy_used(&self, policy_id: Uuid) -> Option<ExecutionPolicy> {
        let mut policies = self.policies.write().expect("agent policy store poisoned");
        let policy = policies.get_mut(&policy_id)?;
        policy.transactions_used = policy.transactions_used.saturating_add(1);
        Some(policy.clone())
    }

    pub fn active_executable_intents(&self) -> Vec<AgentIntent> {
        self.intents
            .read()
            .expect("agent intent store poisoned")
            .values()
            .filter(|intent| {
                matches!(intent.status, IntentStatus::Active)
                    && matches!(
                        intent.parsed_intent.trigger.mode,
                        IntentExecutionMode::Instant
                            | IntentExecutionMode::Recurring
                            | IntentExecutionMode::Conditional
                            | IntentExecutionMode::RecurringConditional
                    )
            })
            .cloned()
            .collect()
    }

    pub fn record_execution_log(
        &self,
        intent: &AgentIntent,
        proposal: ExecutionProposal,
    ) -> AgentExecutionLog {
        let execution_status = if proposal.actionable {
            "proposal_ready_for_user_signature"
        } else {
            "conditions_not_satisfied"
        }
        .to_string();
        let reasoning_hash = hash_json(&json!({
            "intent_id": intent.id,
            "proposal": proposal,
            "execution_status": execution_status,
        }));
        let log = AgentExecutionLog {
            id: Uuid::new_v4(),
            intent_id: intent.id,
            wallet_address: intent.wallet_address.clone(),
            action_type: intent.parsed_intent.action.clone(),
            proposal,
            execution_status,
            reasoning_hash,
            created_at: Utc::now(),
        };

        self.execution_logs
            .write()
            .expect("agent execution log store poisoned")
            .entry(intent.id)
            .or_default()
            .push(log.clone());

        log
    }

    pub fn execution_logs_for_intent(&self, intent_id: Uuid) -> Vec<AgentExecutionLog> {
        self.execution_logs
            .read()
            .expect("agent execution log store poisoned")
            .get(&intent_id)
            .cloned()
            .unwrap_or_default()
    }
}

fn infer_conditions(normalized: &str) -> Vec<ExecutionCondition> {
    let mut conditions = Vec::new();
    let subject = infer_subject(normalized);

    if normalized.contains("tvl") {
        conditions.push(ExecutionCondition {
            metric: "tvl_usd".to_string(),
            subject: subject.clone(),
            operator: infer_operator_for_metric(normalized, "tvl", "greater_than_or_equal"),
            value: threshold_value(normalized, "tvl", Some("usd")),
            provider_hint: Some("nansen-or-mock".to_string()),
        });
    }

    if normalized.contains("risk") {
        conditions.push(ExecutionCondition {
            metric: "risk_score".to_string(),
            subject: subject.clone(),
            operator: infer_operator_for_metric(normalized, "risk", "less_than_or_equal"),
            value: threshold_value(normalized, "risk", None),
            provider_hint: Some("nansen-or-mock".to_string()),
        });
    }

    if normalized.contains("apy") || normalized.contains("yield") {
        conditions.push(ExecutionCondition {
            metric: "apy".to_string(),
            subject,
            operator: infer_operator_for_metric(normalized, "apy", "greater_than_or_equal"),
            value: threshold_value(normalized, "apy", Some("percent")),
            provider_hint: Some("nansen-or-mock".to_string()),
        });
    }

    conditions
}

fn infer_action(normalized: &str) -> String {
    if normalized.contains("buy") || normalized.contains("accumulate") {
        "accumulate".to_string()
    } else if normalized.contains("sell") || normalized.contains("exit") {
        "reduce_exposure".to_string()
    } else if normalized.contains("rebalance") {
        "rebalance".to_string()
    } else {
        "monitor_and_recommend".to_string()
    }
}

fn infer_assets(normalized: &str) -> Vec<String> {
    ["mETH", "MNT", "USDT", "USDC"]
        .iter()
        .filter(|asset| normalized.contains(&asset.to_lowercase()))
        .map(|asset| asset.to_string())
        .collect()
}

fn infer_spend_amount(normalized: &str) -> Option<SpendAmount> {
    let tokens = tokenized(normalized);
    for window in tokens.windows(2) {
        let Some(amount) = parse_number_token(window[0]) else {
            continue;
        };
        let Some(asset) = ["meth", "mnt", "usdt", "usdc"]
            .iter()
            .find(|asset| window[1].eq_ignore_ascii_case(asset))
        else {
            continue;
        };
        return Some(SpendAmount {
            amount,
            asset: asset.to_uppercase().replace("METH", "mETH"),
        });
    }
    None
}

fn infer_protocols(normalized: &str) -> Vec<String> {
    let mut protocols = Vec::new();
    if normalized.contains("agni") {
        protocols.push("Agni Finance".to_string());
    }
    if normalized.contains("meth") {
        protocols.push("mETH Protocol".to_string());
    }
    protocols
}

fn infer_subject(normalized: &str) -> String {
    if normalized.contains("meth") {
        "mETH".to_string()
    } else if normalized.contains("mnt") {
        "MNT".to_string()
    } else if normalized.contains("agni") {
        "Agni Finance".to_string()
    } else {
        "portfolio_or_protocol".to_string()
    }
}

fn infer_operator_for_metric(normalized: &str, metric: &str, fallback: &str) -> String {
    let window = phrase_window(normalized, metric, 8);
    let has_lower_bound_language = window.contains("above")
        || window.contains("over")
        || window.contains("at least")
        || window.contains("minimum")
        || window.contains("greater than")
        || window.contains("climbs")
        || window.contains("rises")
        || window.contains("reaches")
        || window.contains("crosses")
        || window.contains("exceeds")
        || window.contains("hits")
        || window.contains(" to ");
    let has_upper_bound_language = window.contains("below")
        || window.contains("under")
        || window.contains("at most")
        || window.contains("maximum")
        || window.contains("less than")
        || window.contains("drops")
        || window.contains("falls");

    if fallback == "greater_than_or_equal" && has_lower_bound_language {
        "greater_than_or_equal".to_string()
    } else if fallback == "less_than_or_equal" && has_upper_bound_language {
        "less_than_or_equal".to_string()
    } else if has_upper_bound_language {
        "less_than_or_equal".to_string()
    } else if has_lower_bound_language {
        "greater_than_or_equal".to_string()
    } else {
        fallback.to_string()
    }
}

fn threshold_value(normalized: &str, metric: &str, unit: Option<&str>) -> serde_json::Value {
    match extract_threshold(normalized, metric, metric == "risk") {
        Some(amount) => json!({ "amount": amount, "unit": unit }),
        None => json!({ "raw": "threshold_not_extracted", "unit": unit }),
    }
}

fn extract_threshold(normalized: &str, metric: &str, prefer_after: bool) -> Option<f64> {
    let tokens = tokenized(normalized);
    let metric_index = tokens.iter().position(|token| *token == metric)?;

    if prefer_after {
        if let Some(value) = scan_after(&tokens, metric_index) {
            return Some(value);
        }
    }

    let before_start = metric_index.saturating_sub(4);
    for token in tokens[before_start..metric_index].iter().rev() {
        if let Some(value) = parse_number_token(token) {
            return Some(value);
        }
    }

    scan_after(&tokens, metric_index)
}

fn scan_after(tokens: &[&str], metric_index: usize) -> Option<f64> {
    for token in tokens.iter().skip(metric_index + 1).take(6) {
        if let Some(value) = parse_number_token(token) {
            return Some(value);
        }
    }
    None
}

fn phrase_window(normalized: &str, metric: &str, radius: usize) -> String {
    let tokens = tokenized(normalized);
    let Some(metric_index) = tokens.iter().position(|token| *token == metric) else {
        return normalized.to_string();
    };
    let start = metric_index.saturating_sub(radius);
    let end = (metric_index + radius + 1).min(tokens.len());
    tokens[start..end].join(" ")
}

fn tokenized(normalized: &str) -> Vec<&str> {
    normalized
        .split_whitespace()
        .map(|token| {
            token.trim_matches(|ch: char| {
                matches!(ch, ',' | '.' | ':' | ';' | '(' | ')' | '$' | '%')
            })
        })
        .filter(|token| !token.is_empty())
        .collect()
}

fn parse_number_token(token: &str) -> Option<f64> {
    let compact = token.replace(',', "");
    let (number, multiplier) = match compact.chars().last()? {
        'k' => (&compact[..compact.len() - 1], 1_000.0),
        'm' => (&compact[..compact.len() - 1], 1_000_000.0),
        'b' => (&compact[..compact.len() - 1], 1_000_000_000.0),
        _ => (compact.as_str(), 1.0),
    };
    number.parse::<f64>().ok().map(|value| value * multiplier)
}

fn hash_json(value: &serde_json::Value) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value.to_string().as_bytes());
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_recurring_conditional_intent_for_meth_tvl_and_risk() {
        let service = AgentService::new();
        let parsed = service.parse_intent(
            "When mETH climbs to 50M TVL and risk level is below 60, buy 25 USDC weekly",
        );

        assert!(matches!(
            parsed.trigger.mode,
            IntentExecutionMode::RecurringConditional
        ));
        assert!(parsed.requires_user_signature);
        assert_eq!(parsed.spend_amount.as_ref().unwrap().amount, 25.0);
        assert_eq!(parsed.spend_amount.as_ref().unwrap().asset, "USDC");
        assert_eq!(parsed.trigger.conditions.len(), 2);
        assert!(parsed
            .trigger
            .conditions
            .iter()
            .any(|condition| condition.metric == "tvl_usd"
                && condition.operator == "greater_than_or_equal"
                && condition.value["amount"] == json!(50_000_000.0)));
        assert!(parsed
            .trigger
            .conditions
            .iter()
            .any(|condition| condition.metric == "risk_score"
                && condition.operator == "less_than_or_equal"
                && condition.value["amount"] == json!(60.0)));
        assert!(parsed.target_assets.contains(&"mETH".to_string()));
        assert!(parsed.target_assets.contains(&"USDC".to_string()));
    }

    #[test]
    fn parses_instant_intent_without_conditions() {
        let service = AgentService::new();
        let parsed = service.parse_intent("Buy MNT now");

        assert!(matches!(parsed.trigger.mode, IntentExecutionMode::Instant));
        assert!(parsed.trigger.conditions.is_empty());
        assert!(parsed.target_assets.contains(&"MNT".to_string()));
        assert!(parsed.spend_amount.is_none());
    }

    #[test]
    fn parses_recurring_intent_without_conditions() {
        let service = AgentService::new();
        let parsed = service.parse_intent("Accumulate 10 USDC every week");

        assert!(matches!(
            parsed.trigger.mode,
            IntentExecutionMode::Recurring
        ));
        assert!(parsed.trigger.conditions.is_empty());
        assert!(parsed.trigger.schedule.is_some());
    }

    #[test]
    fn parses_conditional_tvl_intent() {
        let service = AgentService::new();
        let parsed = service.parse_intent("When mETH TVL goes above 42M, alert me");

        assert!(matches!(
            parsed.trigger.mode,
            IntentExecutionMode::Conditional
        ));
        assert_eq!(parsed.trigger.conditions.len(), 1);
        assert_eq!(parsed.trigger.conditions[0].metric, "tvl_usd");
        assert_eq!(
            parsed.trigger.conditions[0].value["amount"],
            json!(42_000_000.0)
        );
    }

    #[test]
    fn parses_recurrent_intent_wording() {
        let service = AgentService::new();
        let parsed = service.parse_intent("Recurrent buy 10 USDC into mETH");

        assert!(matches!(
            parsed.trigger.mode,
            IntentExecutionMode::Recurring
        ));
        assert_eq!(parsed.spend_amount.as_ref().unwrap().amount, 10.0);
        assert_eq!(parsed.spend_amount.as_ref().unwrap().asset, "USDC");
    }

    #[test]
    fn parses_crosses_and_at_least_tvl_language() {
        let service = AgentService::new();
        let parsed =
            service.parse_intent("If mETH TVL crosses at least 55M, accumulate 20 USDC weekly");

        assert!(matches!(
            parsed.trigger.mode,
            IntentExecutionMode::RecurringConditional
        ));
        assert_eq!(parsed.trigger.conditions.len(), 1);
        assert_eq!(parsed.trigger.conditions[0].metric, "tvl_usd");
        assert_eq!(
            parsed.trigger.conditions[0].operator,
            "greater_than_or_equal"
        );
        assert_eq!(
            parsed.trigger.conditions[0].value["amount"],
            json!(55_000_000.0)
        );
    }

    #[test]
    fn parses_at_most_risk_language() {
        let service = AgentService::new();
        let parsed = service.parse_intent("If mETH risk level is at most 45, buy 10 USDC");

        assert!(matches!(
            parsed.trigger.mode,
            IntentExecutionMode::Conditional
        ));
        assert_eq!(parsed.trigger.conditions.len(), 1);
        assert_eq!(parsed.trigger.conditions[0].metric, "risk_score");
        assert_eq!(parsed.trigger.conditions[0].operator, "less_than_or_equal");
        assert_eq!(parsed.trigger.conditions[0].value["amount"], json!(45.0));
    }

    #[test]
    fn stores_and_updates_intent_lifecycle() {
        let service = AgentService::new();
        let intent = service.create_intent(CreateIntentRequest {
            wallet_address: "0x123".to_string(),
            raw_intent: "Buy MNT now".to_string(),
        });
        let policy = service.create_policy(&intent);

        assert_eq!(service.list_intents("0x123").len(), 1);
        assert_eq!(
            service.get_intent(intent.id).unwrap().intent_hash,
            intent.intent_hash
        );
        assert_eq!(service.policies_for_intent(intent.id)[0].id, policy.id);

        let paused = service
            .update_status(intent.id, IntentStatus::Paused)
            .unwrap();
        assert!(matches!(paused.status, IntentStatus::Paused));
    }

    #[test]
    fn lists_active_executable_intents_and_records_execution_logs() {
        let service = AgentService::new();
        let intent = service.create_intent(CreateIntentRequest {
            wallet_address: "0x123".to_string(),
            raw_intent: "When mETH TVL climbs above 40M, buy 25 USDC weekly".to_string(),
        });
        assert!(service.active_executable_intents().is_empty());

        let active = service
            .update_status(intent.id, IntentStatus::Active)
            .unwrap();
        assert_eq!(service.active_executable_intents().len(), 1);

        let proposal = crate::models::execution::ExecutionProposal {
            actionable: true,
            action: active.parsed_intent.action.clone(),
            wallet_address: active.wallet_address.clone(),
            chain_id: 5003,
            network: "mantle-testnet".to_string(),
            conditions: Vec::new(),
            transaction_draft: None,
            required_authorization: "user-signed transaction".to_string(),
        };

        let log = service.record_execution_log(&active, proposal);
        assert_eq!(log.intent_id, active.id);
        assert_eq!(service.execution_logs_for_intent(active.id).len(), 1);
    }
}
