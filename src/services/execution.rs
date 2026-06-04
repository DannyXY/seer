use std::collections::HashMap;
use std::str::FromStr;

use ethers_core::{
    abi::{encode, Token},
    types::{Address, U256},
    utils::id,
};
use serde_json::json;

use crate::{
    config::Settings,
    models::{
        agent::{
            AgentIntent, CreateIntentRequest, ExecutionCondition, ExecutionPolicy, ParsedIntent,
        },
        execution::{
            ConditionEvaluation, DelegatedExecutionResult, ExecutionProposal,
            ExecutionReadinessResponse, ProtocolExecutionReadiness, TransactionDraft,
            UserOperationDraft,
        },
    },
    services::data_provider::OnchainDataProvider,
};

pub struct ExecutionService {
    chain_id: u64,
    action_config: ActionConfig,
}

#[derive(Debug, Clone, Default)]
struct ActionConfig {
    token_addresses: HashMap<String, String>,
    approved_strategy_address: Option<String>,
    strategy_deposit_function: String,
    protocol_strategies: HashMap<String, ProtocolStrategy>,
}

#[derive(Debug, Clone)]
struct ProtocolStrategy {
    address: String,
    deposit_function: String,
}

impl ExecutionService {
    pub fn new(settings: Settings) -> Self {
        let token_addresses = [
            ("USDC", settings.mantle_usdc_address.clone()),
            ("USDT", settings.mantle_usdt_address.clone()),
            ("MNT", settings.mantle_mnt_address.clone()),
            ("mETH", settings.mantle_meth_address.clone()),
        ]
        .into_iter()
        .filter_map(|(symbol, address)| address.map(|address| (symbol.to_string(), address)))
        .collect();
        let protocol_strategies = protocol_strategies_from_settings(&settings);
        Self {
            chain_id: settings.mantle_chain_id,
            action_config: ActionConfig {
                token_addresses,
                approved_strategy_address: settings.approved_strategy_address,
                strategy_deposit_function: settings.strategy_deposit_function,
                protocol_strategies,
            },
        }
    }

    pub async fn evaluate_intent(
        &self,
        provider: &dyn OnchainDataProvider,
        request: CreateIntentRequest,
        parsed: ParsedIntent,
    ) -> anyhow::Result<ExecutionProposal> {
        let mut evaluations = Vec::new();
        for condition in &parsed.trigger.conditions {
            evaluations.push(self.evaluate_condition(provider, condition).await?);
        }

        let actionable = evaluations.iter().all(|evaluation| evaluation.passed);
        let transaction_draft = actionable.then(|| {
            self.build_transaction_draft(&parsed).unwrap_or_else(|| TransactionDraft {
                kind: "recommendation".to_string(),
                to: None,
                value: "0".to_string(),
                data: None,
                chain_id: self.chain_id,
                human_summary: "No contract call is generated yet; Seer can recommend the action and anchor the intent.".to_string(),
            })
        });

        Ok(ExecutionProposal {
            actionable,
            action: parsed.action,
            wallet_address: request.wallet_address,
            chain_id: self.chain_id,
            network: "mantle-testnet".to_string(),
            conditions: evaluations,
            transaction_draft,
            required_authorization: "user-signed transaction or scoped delegated execution policy"
                .to_string(),
        })
    }

    pub fn readiness(&self) -> ExecutionReadinessResponse {
        let mut configured_token_symbols: Vec<_> =
            self.action_config.token_addresses.keys().cloned().collect();
        configured_token_symbols.sort();

        ExecutionReadinessResponse {
            chain_id: self.chain_id,
            configured_token_symbols,
            generic_strategy_address: self.action_config.approved_strategy_address.clone(),
            generic_deposit_function: self.action_config.strategy_deposit_function.clone(),
            protocols: known_protocols()
                .iter()
                .map(|protocol| {
                    let strategy = self.action_config.protocol_strategies.get(*protocol);
                    ProtocolExecutionReadiness {
                        protocol: (*protocol).to_string(),
                        strategy_address: strategy.map(|strategy| strategy.address.clone()),
                        deposit_function: strategy
                            .map(|strategy| strategy.deposit_function.clone()),
                        ready_for_strategy_draft: strategy.is_some(),
                    }
                })
                .collect(),
        }
    }

    pub async fn evaluate_stored_intent(
        &self,
        provider: &dyn OnchainDataProvider,
        intent: &AgentIntent,
    ) -> anyhow::Result<ExecutionProposal> {
        self.evaluate_intent(
            provider,
            CreateIntentRequest {
                wallet_address: intent.wallet_address.clone(),
                raw_intent: intent.raw_intent.clone(),
            },
            intent.parsed_intent.clone(),
        )
        .await
    }

    pub async fn evaluate_intent_with_allowance(
        &self,
        provider: &dyn OnchainDataProvider,
        request: CreateIntentRequest,
        parsed: ParsedIntent,
        allowance: Option<U256>,
    ) -> anyhow::Result<ExecutionProposal> {
        let mut proposal = self
            .evaluate_intent(provider, request, parsed.clone())
            .await?;
        if proposal.actionable {
            if let Some(allowance) = allowance {
                if self
                    .spend_units(&parsed)
                    .is_some_and(|required| allowance >= required)
                {
                    proposal.transaction_draft =
                        self.build_strategy_deposit_draft(&parsed)
                            .or_else(|| {
                                Some(TransactionDraft {
                                    kind: "allowance_sufficient".to_string(),
                                    to: None,
                                    value: "0".to_string(),
                                    data: None,
                                    chain_id: self.chain_id,
                                    human_summary:
                                        "Existing ERC-20 allowance is sufficient; approval transaction is not needed."
                                            .to_string(),
                                })
                            });
                }
            }
        }
        Ok(proposal)
    }

    pub fn build_delegated_execution(
        &self,
        intent: &AgentIntent,
        policy: &ExecutionPolicy,
        proposal: ExecutionProposal,
    ) -> DelegatedExecutionResult {
        if !proposal.actionable {
            return DelegatedExecutionResult {
                executable: false,
                execution_status: "conditions_not_satisfied".to_string(),
                policy_hash: policy.policy_hash.clone(),
                proposal,
                user_operation: None,
                reason: "intent conditions have not passed".to_string(),
            };
        }

        if let Some(max) = policy.max_transaction_count {
            if policy.transactions_used >= max {
                return DelegatedExecutionResult {
                    executable: false,
                    execution_status: "policy_transaction_limit_reached".to_string(),
                    policy_hash: policy.policy_hash.clone(),
                    proposal,
                    user_operation: None,
                    reason: "session policy transaction budget is exhausted".to_string(),
                };
            }
        }

        let disallowed_assets: Vec<_> = intent
            .parsed_intent
            .target_assets
            .iter()
            .filter(|asset| {
                !policy
                    .allowed_assets
                    .iter()
                    .any(|allowed| allowed == *asset)
            })
            .cloned()
            .collect();
        if !disallowed_assets.is_empty() {
            return DelegatedExecutionResult {
                executable: false,
                execution_status: "policy_asset_violation".to_string(),
                policy_hash: policy.policy_hash.clone(),
                proposal,
                user_operation: None,
                reason: format!("assets not allowed by policy: {disallowed_assets:?}"),
            };
        }

        let disallowed_protocols: Vec<_> = intent
            .parsed_intent
            .target_protocols
            .iter()
            .filter(|protocol| {
                !policy
                    .allowed_protocols
                    .iter()
                    .any(|allowed| allowed == *protocol)
            })
            .cloned()
            .collect();
        if !disallowed_protocols.is_empty() {
            return DelegatedExecutionResult {
                executable: false,
                execution_status: "policy_protocol_violation".to_string(),
                policy_hash: policy.policy_hash.clone(),
                proposal,
                user_operation: None,
                reason: format!("protocols not allowed by policy: {disallowed_protocols:?}"),
            };
        }

        let target = proposal
            .transaction_draft
            .as_ref()
            .and_then(|draft| draft.to.clone());
        if let Some(target) = &target {
            let target_allowed = policy
                .allowed_contracts
                .iter()
                .any(|allowed| allowed.eq_ignore_ascii_case(target));
            if !target_allowed {
                return DelegatedExecutionResult {
                    executable: false,
                    execution_status: "policy_contract_violation".to_string(),
                    policy_hash: policy.policy_hash.clone(),
                    proposal,
                    user_operation: None,
                    reason: format!("contract {target} is not allowed by policy"),
                };
            }
        }

        let smart_account = policy
            .smart_account_address
            .clone()
            .unwrap_or_else(|| intent.wallet_address.clone());
        let draft = proposal.transaction_draft.as_ref();
        let user_operation = UserOperationDraft {
            sender: smart_account,
            call_data: draft.and_then(|draft| draft.data.clone()),
            target,
            value: draft.map(|draft| draft.value.clone()).unwrap_or_else(|| "0".to_string()),
            chain_id: self.chain_id,
            policy_hash: policy.policy_hash.clone(),
            human_summary: "Session-key policy passed. Build/sign an ERC-4337 user operation with the configured smart-account provider.".to_string(),
        };

        DelegatedExecutionResult {
            executable: true,
            execution_status: "user_operation_ready".to_string(),
            policy_hash: policy.policy_hash.clone(),
            proposal,
            user_operation: Some(user_operation),
            reason: "policy checks passed".to_string(),
        }
    }

    async fn evaluate_condition(
        &self,
        provider: &dyn OnchainDataProvider,
        condition: &ExecutionCondition,
    ) -> anyhow::Result<ConditionEvaluation> {
        let observed = match condition.metric.as_str() {
            "tvl_usd" => {
                let metrics = provider.get_protocol_metrics(&condition.subject).await?;
                Some(json!(metrics.tvl_usd))
            }
            "risk_score" => {
                let metrics = provider.get_protocol_metrics(&condition.subject).await?;
                Some(json!(metrics.risk_score))
            }
            "apy" => {
                let metrics = provider.get_protocol_metrics(&condition.subject).await?;
                metrics.apy.map(|apy| json!(apy))
            }
            _ => None,
        };

        let threshold = condition
            .value
            .get("amount")
            .and_then(|value| value.as_f64());
        let observed_number = observed.as_ref().and_then(|value| value.as_f64());
        let passed = match (observed_number, threshold, condition.operator.as_str()) {
            (Some(observed), Some(threshold), "greater_than_or_equal") => observed >= threshold,
            (Some(observed), Some(threshold), "less_than_or_equal") => observed <= threshold,
            _ => false,
        };

        Ok(ConditionEvaluation {
            condition: condition.clone(),
            observed_value: observed,
            passed,
            reason: if passed {
                "condition satisfied by provider facts".to_string()
            } else {
                "condition not satisfied or missing comparable provider facts".to_string()
            },
        })
    }

    fn build_transaction_draft(&self, parsed: &ParsedIntent) -> Option<TransactionDraft> {
        match parsed.action.as_str() {
            "accumulate" => self
                .build_approval_draft(parsed)
                .or_else(|| Some(TransactionDraft {
                    kind: "swap_or_strategy_deposit".to_string(),
                    to: None,
                    value: "0".to_string(),
                    data: None,
                    chain_id: self.chain_id,
                    human_summary: self.missing_strategy_summary(parsed),
                })),
            "reduce_exposure" => Some(TransactionDraft {
                kind: "exit_or_reduce_position".to_string(),
                to: None,
                value: "0".to_string(),
                data: None,
                chain_id: self.chain_id,
                human_summary:
                    "Prepare a user-signed Mantle testnet reduction action after policy checks pass."
                        .to_string(),
            }),
            _ => None,
        }
    }

    fn build_approval_draft(&self, parsed: &ParsedIntent) -> Option<TransactionDraft> {
        let spend = parsed.spend_amount.as_ref()?;
        let token_address = self.action_config.token_addresses.get(&spend.asset)?;
        let spender = self.strategy_for_intent(parsed)?;
        let token = Address::from_str(token_address).ok()?;
        let spender_address = Address::from_str(&spender.address).ok()?;
        let amount = self.spend_units(parsed)?;
        let mut data = id("approve(address,uint256)")[..4].to_vec();
        data.extend(encode(&[
            Token::Address(spender_address),
            Token::Uint(amount),
        ]));

        Some(TransactionDraft {
            kind: "erc20_approve".to_string(),
            to: Some(format!("{token:?}").to_lowercase()),
            value: "0".to_string(),
            data: Some(format!("0x{}", hex_encode(&data))),
            chain_id: self.chain_id,
            human_summary: format!(
                "Approve {} {} for strategy contract {} on Mantle testnet.",
                spend.amount, spend.asset, spender.address
            ),
        })
    }

    fn build_strategy_deposit_draft(&self, parsed: &ParsedIntent) -> Option<TransactionDraft> {
        let spend = parsed.spend_amount.as_ref()?;
        let token_address = self.action_config.token_addresses.get(&spend.asset)?;
        let strategy = self.strategy_for_intent(parsed)?;
        let token = Address::from_str(token_address).ok()?;
        let strategy_address = Address::from_str(&strategy.address).ok()?;
        let amount = self.spend_units(parsed)?;
        let signature = strategy.deposit_function.trim().to_string();
        if signature.is_empty() || !signature.contains("(address,uint256)") {
            return None;
        }

        let mut data = id(&signature)[..4].to_vec();
        data.extend(encode(&[Token::Address(token), Token::Uint(amount)]));

        Some(TransactionDraft {
            kind: "strategy_deposit".to_string(),
            to: Some(format!("{strategy_address:?}").to_lowercase()),
            value: "0".to_string(),
            data: Some(format!("0x{}", hex_encode(&data))),
            chain_id: self.chain_id,
            human_summary: format!(
                "Execute {} {} into strategy contract {} on Mantle testnet using {}.",
                spend.amount, spend.asset, strategy.address, signature
            ),
        })
    }

    fn strategy_for_intent(&self, parsed: &ParsedIntent) -> Option<ProtocolStrategy> {
        let requested_config_required_protocol = parsed
            .target_protocols
            .iter()
            .find(|protocol| protocol_requires_explicit_config(protocol));
        if let Some(protocol) = requested_config_required_protocol {
            return self
                .action_config
                .protocol_strategies
                .get(protocol)
                .cloned();
        }

        self.action_config
            .approved_strategy_address
            .as_ref()
            .map(|address| ProtocolStrategy {
                address: address.clone(),
                deposit_function: self.action_config.strategy_deposit_function.clone(),
            })
    }

    fn spend_units(&self, parsed: &ParsedIntent) -> Option<U256> {
        let spend = parsed.spend_amount.as_ref()?;
        decimal_amount_to_units(spend.amount, token_decimals(&spend.asset))
    }

    fn missing_strategy_summary(&self, parsed: &ParsedIntent) -> String {
        let requested_known_protocols: Vec<_> = parsed
            .target_protocols
            .iter()
            .filter(|protocol| protocol_requires_explicit_config(protocol))
            .cloned()
            .collect();
        if !requested_known_protocols.is_empty() {
            return format!(
                "Protocol destination {:?} is named in the intent but not configured. Configure the matching strategy address and deposit function before emitting runnable calldata.",
                requested_known_protocols
            );
        }
        format!(
            "Prepare a user-signed Mantle testnet action for assets {:?} through approved protocols {:?}. Configure token and strategy addresses to emit runnable calldata.",
            parsed.target_assets, parsed.target_protocols
        )
    }
}

fn token_decimals(asset: &str) -> u32 {
    match asset {
        "USDC" | "USDT" => 6,
        _ => 18,
    }
}

fn known_protocols() -> [&'static str; 4] {
    ["Merchant Moe", "Lendle", "Agni Finance", "mETH Protocol"]
}

fn protocol_requires_explicit_config(protocol: &str) -> bool {
    matches!(protocol, "Merchant Moe" | "Lendle" | "Agni Finance")
}

fn protocol_strategies_from_settings(settings: &Settings) -> HashMap<String, ProtocolStrategy> {
    let mut strategies = HashMap::new();
    insert_protocol_strategy(
        &mut strategies,
        known_protocols()[0],
        settings.merchant_moe_strategy_address.clone(),
        settings.merchant_moe_deposit_function.clone(),
        &settings.strategy_deposit_function,
    );
    insert_protocol_strategy(
        &mut strategies,
        known_protocols()[1],
        settings.lendle_strategy_address.clone(),
        settings.lendle_deposit_function.clone(),
        &settings.strategy_deposit_function,
    );
    insert_protocol_strategy(
        &mut strategies,
        known_protocols()[2],
        settings.agni_strategy_address.clone(),
        settings.agni_deposit_function.clone(),
        &settings.strategy_deposit_function,
    );
    insert_protocol_strategy(
        &mut strategies,
        known_protocols()[3],
        settings.meth_strategy_address.clone(),
        settings.meth_deposit_function.clone(),
        &settings.strategy_deposit_function,
    );
    strategies
}

fn insert_protocol_strategy(
    strategies: &mut HashMap<String, ProtocolStrategy>,
    protocol: &str,
    address: Option<String>,
    deposit_function: Option<String>,
    default_deposit_function: &str,
) {
    let Some(address) = address else {
        return;
    };
    strategies.insert(
        protocol.to_string(),
        ProtocolStrategy {
            address,
            deposit_function: deposit_function
                .unwrap_or_else(|| default_deposit_function.to_string()),
        },
    );
}

fn decimal_amount_to_units(amount: f64, decimals: u32) -> Option<U256> {
    if !amount.is_finite() || amount < 0.0 {
        return None;
    }
    let multiplier = 10u128.checked_pow(decimals)?;
    let scaled = (amount * multiplier as f64).round();
    if scaled > u128::MAX as f64 {
        return None;
    }
    Some(U256::from(scaled as u128))
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

#[cfg(test)]
mod tests {
    use ethers_core::types::U256;

    use crate::{
        config::Settings,
        models::agent::{CreateIntentRequest, CreateSessionPolicyRequest, IntentExecutionMode},
        services::{
            agent::AgentService,
            data_provider::{MockProvider, OnchainDataProvider},
        },
    };

    use super::ExecutionService;

    #[tokio::test]
    async fn evaluates_recurring_conditional_intent_into_actionable_proposal() {
        let agent = AgentService::new();
        let settings = Settings::from_env().unwrap();
        let execution = ExecutionService::new(settings);
        let request = CreateIntentRequest {
            wallet_address: "0x123".to_string(),
            raw_intent: "When mETH TVL climbs above 40M and risk score is below 60, accumulate 25 USDC weekly into mETH".to_string(),
        };
        let parsed = agent.parse_intent(&request.raw_intent);

        assert!(matches!(
            parsed.trigger.mode,
            IntentExecutionMode::RecurringConditional
        ));

        let provider: &dyn OnchainDataProvider = &MockProvider;
        let proposal = execution
            .evaluate_intent(provider, request, parsed)
            .await
            .unwrap();

        assert!(proposal.actionable);
        assert_eq!(proposal.conditions.len(), 2);
        assert!(proposal.transaction_draft.is_some());
    }

    #[tokio::test]
    async fn delegated_execution_requires_session_policy_and_builds_user_operation() {
        let agent = AgentService::new();
        let settings = Settings::from_env().unwrap();
        let execution = ExecutionService::new(settings);
        let intent = agent.create_intent(CreateIntentRequest {
            wallet_address: "0x123".to_string(),
            raw_intent: "When mETH TVL climbs above 40M and risk score is below 60, accumulate 25 USDC weekly into mETH".to_string(),
        });
        let policy = agent.create_session_policy(
            &intent,
            CreateSessionPolicyRequest {
                smart_account_address: "0xsmartaccount".to_string(),
                session_key_address: "0xsessionkey".to_string(),
                allowed_assets: vec!["mETH".to_string(), "USDC".to_string()],
                allowed_protocols: vec!["mETH Protocol".to_string()],
                allowed_contracts: Vec::new(),
                max_spend_usd: Some(100.0),
                max_transaction_count: Some(2),
                expires_in_days: Some(7),
            },
        );

        let provider: &dyn OnchainDataProvider = &MockProvider;
        let proposal = execution
            .evaluate_stored_intent(provider, &intent)
            .await
            .unwrap();
        let result = execution.build_delegated_execution(&intent, &policy, proposal);

        assert!(result.executable);
        assert_eq!(result.execution_status, "user_operation_ready");
        assert!(result.user_operation.is_some());
    }

    #[tokio::test]
    async fn delegated_execution_blocks_policy_asset_violation() {
        let agent = AgentService::new();
        let settings = Settings::from_env().unwrap();
        let execution = ExecutionService::new(settings);
        let intent = agent.create_intent(CreateIntentRequest {
            wallet_address: "0x123".to_string(),
            raw_intent: "When mETH TVL climbs above 40M, accumulate 25 USDC weekly into mETH"
                .to_string(),
        });
        let policy = agent.create_session_policy(
            &intent,
            CreateSessionPolicyRequest {
                smart_account_address: "0xsmartaccount".to_string(),
                session_key_address: "0xsessionkey".to_string(),
                allowed_assets: vec!["MNT".to_string()],
                allowed_protocols: vec!["mETH Protocol".to_string()],
                allowed_contracts: Vec::new(),
                max_spend_usd: Some(100.0),
                max_transaction_count: Some(2),
                expires_in_days: Some(7),
            },
        );

        let provider: &dyn OnchainDataProvider = &MockProvider;
        let proposal = execution
            .evaluate_stored_intent(provider, &intent)
            .await
            .unwrap();
        let result = execution.build_delegated_execution(&intent, &policy, proposal);

        assert!(!result.executable);
        assert_eq!(result.execution_status, "policy_asset_violation");
    }

    #[tokio::test]
    async fn builds_real_erc20_approval_transaction_draft_when_addresses_are_configured() {
        let agent = AgentService::new();
        let mut settings = Settings::from_env().unwrap();
        settings.mantle_usdc_address =
            Some("0x0000000000000000000000000000000000000001".to_string());
        settings.approved_strategy_address =
            Some("0x0000000000000000000000000000000000000002".to_string());
        settings.strategy_deposit_function = "deposit(address,uint256)".to_string();
        let execution = ExecutionService::new(settings);
        let request = CreateIntentRequest {
            wallet_address: "0x123".to_string(),
            raw_intent: "When mETH TVL climbs above 40M, accumulate 25 USDC weekly into mETH"
                .to_string(),
        };
        let parsed = agent.parse_intent(&request.raw_intent);
        let provider: &dyn OnchainDataProvider = &MockProvider;
        let proposal = execution
            .evaluate_intent(provider, request, parsed)
            .await
            .unwrap();
        let draft = proposal.transaction_draft.unwrap();

        assert_eq!(draft.kind, "erc20_approve");
        assert_eq!(
            draft.to,
            Some("0x0000000000000000000000000000000000000001".to_string())
        );
        assert!(draft.data.unwrap().starts_with("0x095ea7b3"));
        assert_eq!(draft.chain_id, 5003);
    }

    #[tokio::test]
    async fn approval_draft_uses_named_protocol_destination() {
        let agent = AgentService::new();
        let mut settings = Settings::from_env().unwrap();
        settings.mantle_usdc_address =
            Some("0x0000000000000000000000000000000000000001".to_string());
        settings.approved_strategy_address =
            Some("0x0000000000000000000000000000000000000002".to_string());
        settings.merchant_moe_strategy_address =
            Some("0x0000000000000000000000000000000000000003".to_string());
        let execution = ExecutionService::new(settings);
        let request = CreateIntentRequest {
            wallet_address: "0x123".to_string(),
            raw_intent:
                "When mETH TVL climbs above 40M, accumulate 25 USDC weekly into Merchant Moe"
                    .to_string(),
        };
        let parsed = agent.parse_intent(&request.raw_intent);
        let provider: &dyn OnchainDataProvider = &MockProvider;
        let proposal = execution
            .evaluate_intent(provider, request, parsed)
            .await
            .unwrap();
        let draft = proposal.transaction_draft.unwrap();

        assert_eq!(draft.kind, "erc20_approve");
        assert_eq!(
            draft.human_summary,
            "Approve 25 USDC for strategy contract 0x0000000000000000000000000000000000000003 on Mantle testnet."
        );
    }

    #[tokio::test]
    async fn explicit_unconfigured_protocol_does_not_use_generic_strategy() {
        let agent = AgentService::new();
        let mut settings = Settings::from_env().unwrap();
        settings.mantle_usdc_address =
            Some("0x0000000000000000000000000000000000000001".to_string());
        settings.approved_strategy_address =
            Some("0x0000000000000000000000000000000000000002".to_string());
        let execution = ExecutionService::new(settings);
        let request = CreateIntentRequest {
            wallet_address: "0x123".to_string(),
            raw_intent:
                "When mETH TVL climbs above 40M, accumulate 25 USDC weekly into Merchant Moe"
                    .to_string(),
        };
        let parsed = agent.parse_intent(&request.raw_intent);
        let provider: &dyn OnchainDataProvider = &MockProvider;
        let proposal = execution
            .evaluate_intent(provider, request, parsed)
            .await
            .unwrap();
        let draft = proposal.transaction_draft.unwrap();

        assert_eq!(draft.kind, "swap_or_strategy_deposit");
        assert!(draft.to.is_none());
        assert!(draft.data.is_none());
        assert!(draft
            .human_summary
            .contains("named in the intent but not configured"));
    }

    #[tokio::test]
    async fn explicit_unconfigured_protocol_with_allowance_does_not_build_deposit() {
        let agent = AgentService::new();
        let mut settings = Settings::from_env().unwrap();
        settings.mantle_usdc_address =
            Some("0x0000000000000000000000000000000000000001".to_string());
        settings.approved_strategy_address =
            Some("0x0000000000000000000000000000000000000002".to_string());
        let execution = ExecutionService::new(settings);
        let request = CreateIntentRequest {
            wallet_address: "0x123".to_string(),
            raw_intent:
                "When mETH TVL climbs above 40M, accumulate 25 USDC weekly into Merchant Moe"
                    .to_string(),
        };
        let parsed = agent.parse_intent(&request.raw_intent);
        let provider: &dyn OnchainDataProvider = &MockProvider;
        let proposal = execution
            .evaluate_intent_with_allowance(
                provider,
                request,
                parsed,
                Some(U256::from(25_000_000u64)),
            )
            .await
            .unwrap();
        let draft = proposal.transaction_draft.unwrap();

        assert_eq!(draft.kind, "allowance_sufficient");
        assert!(draft.to.is_none());
        assert!(draft.data.is_none());
    }

    #[test]
    fn readiness_reports_protocol_configuration() {
        let mut settings = Settings::from_env().unwrap();
        settings.mantle_usdc_address =
            Some("0x0000000000000000000000000000000000000001".to_string());
        settings.merchant_moe_strategy_address =
            Some("0x0000000000000000000000000000000000000003".to_string());
        let execution = ExecutionService::new(settings);
        let readiness = execution.readiness();

        assert!(readiness
            .configured_token_symbols
            .contains(&"USDC".to_string()));
        assert!(readiness.protocols.iter().any(|protocol| {
            protocol.protocol == "Merchant Moe" && protocol.ready_for_strategy_draft
        }));
        assert!(readiness.protocols.iter().any(|protocol| {
            protocol.protocol == "Lendle" && !protocol.ready_for_strategy_draft
        }));
    }

    #[tokio::test]
    async fn builds_strategy_deposit_when_allowance_is_sufficient() {
        let agent = AgentService::new();
        let mut settings = Settings::from_env().unwrap();
        settings.mantle_usdc_address =
            Some("0x0000000000000000000000000000000000000001".to_string());
        settings.approved_strategy_address =
            Some("0x0000000000000000000000000000000000000002".to_string());
        let execution = ExecutionService::new(settings);
        let request = CreateIntentRequest {
            wallet_address: "0x123".to_string(),
            raw_intent: "When mETH TVL climbs above 40M, accumulate 25 USDC weekly into mETH"
                .to_string(),
        };
        let parsed = agent.parse_intent(&request.raw_intent);
        let provider: &dyn OnchainDataProvider = &MockProvider;
        let proposal = execution
            .evaluate_intent_with_allowance(
                provider,
                request,
                parsed,
                Some(U256::from(25_000_000u64)),
            )
            .await
            .unwrap();
        let draft = proposal.transaction_draft.unwrap();

        assert_eq!(draft.kind, "strategy_deposit");
        assert_eq!(
            draft.to,
            Some("0x0000000000000000000000000000000000000002".to_string())
        );
        assert!(draft.data.unwrap().starts_with("0x47e7ef24"));
    }

    #[tokio::test]
    async fn routes_strategy_deposit_to_named_protocol_destination() {
        let agent = AgentService::new();
        let mut settings = Settings::from_env().unwrap();
        settings.mantle_usdc_address =
            Some("0x0000000000000000000000000000000000000001".to_string());
        settings.approved_strategy_address =
            Some("0x0000000000000000000000000000000000000002".to_string());
        settings.merchant_moe_strategy_address =
            Some("0x0000000000000000000000000000000000000003".to_string());
        settings.merchant_moe_deposit_function = Some("deposit(address,uint256)".to_string());
        let execution = ExecutionService::new(settings);
        let request = CreateIntentRequest {
            wallet_address: "0x123".to_string(),
            raw_intent:
                "When mETH TVL climbs above 40M, accumulate 25 USDC weekly into Merchant Moe"
                    .to_string(),
        };
        let parsed = agent.parse_intent(&request.raw_intent);
        let provider: &dyn OnchainDataProvider = &MockProvider;
        let proposal = execution
            .evaluate_intent_with_allowance(
                provider,
                request,
                parsed,
                Some(U256::from(25_000_000u64)),
            )
            .await
            .unwrap();
        let draft = proposal.transaction_draft.unwrap();

        assert_eq!(draft.kind, "strategy_deposit");
        assert_eq!(
            draft.to,
            Some("0x0000000000000000000000000000000000000003".to_string())
        );
    }

    #[tokio::test]
    async fn falls_back_when_strategy_signature_is_not_supported() {
        let agent = AgentService::new();
        let mut settings = Settings::from_env().unwrap();
        settings.mantle_usdc_address =
            Some("0x0000000000000000000000000000000000000001".to_string());
        settings.approved_strategy_address =
            Some("0x0000000000000000000000000000000000000002".to_string());
        settings.strategy_deposit_function = "deposit(uint256)".to_string();
        let execution = ExecutionService::new(settings);
        let request = CreateIntentRequest {
            wallet_address: "0x123".to_string(),
            raw_intent: "When mETH TVL climbs above 40M, accumulate 25 USDC weekly into mETH"
                .to_string(),
        };
        let parsed = agent.parse_intent(&request.raw_intent);
        let provider: &dyn OnchainDataProvider = &MockProvider;
        let proposal = execution
            .evaluate_intent_with_allowance(
                provider,
                request,
                parsed,
                Some(U256::from(25_000_000u64)),
            )
            .await
            .unwrap();
        let draft = proposal.transaction_draft.unwrap();

        assert_eq!(draft.kind, "allowance_sufficient");
        assert!(draft.data.is_none());
    }
}
