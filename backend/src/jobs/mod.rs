use chrono::Utc;
use serde_json::json;
use tokio::time::{interval, MissedTickBehavior};
use tracing::info;

use crate::{
    db::{
        persist_agent_execution_log, persist_agent_execution_policy, persist_job_run,
        persist_signals, JobRunRecord, JobRunStatus,
    },
    AppState,
};

pub fn spawn_internal_jobs(state: AppState) {
    tokio::spawn(async move {
        run_scheduler(state).await;
    });
}

pub async fn run_worker(state: AppState) -> anyhow::Result<()> {
    info!(
        provider = state.services.provider_name(),
        "seer worker starting"
    );
    run_scheduler(state).await;
    Ok(())
}

async fn run_scheduler(state: AppState) {
    let mut fast_tick = interval(std::time::Duration::from_secs(3_600));   // 1 hour
    let mut arena_tick = interval(std::time::Duration::from_secs(3_600));   // 1 hour
    let mut prediction_tick = interval(std::time::Duration::from_secs(7_200)); // 2 hours
    let mut cohort_tick = interval(std::time::Duration::from_secs(14_400));  // 4 hours

    fast_tick.set_missed_tick_behavior(MissedTickBehavior::Skip);
    arena_tick.set_missed_tick_behavior(MissedTickBehavior::Skip);
    prediction_tick.set_missed_tick_behavior(MissedTickBehavior::Skip);
    cohort_tick.set_missed_tick_behavior(MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            _ = fast_tick.tick() => run_fast_jobs(&state).await,
            _ = arena_tick.tick() => run_arena_refresh_jobs(&state).await,
            _ = prediction_tick.tick() => run_prediction_generation_jobs(&state).await,
            _ = cohort_tick.tick() => run_cohort_jobs(&state).await,
        }
    }
}

async fn run_fast_jobs(state: &AppState) {
    let started_at = Utc::now();
    let provider = state.services.provider.provider().await;
    let active_intents = state.services.agent.active_executable_intents();
    let mut evaluated_intents = 0usize;
    let mut actionable_intents = 0usize;
    let mut delegated_ready_intents = 0usize;
    let mut signal_snapshots = 0usize;
    let mut errors = Vec::new();

    match state.services.signals.generate(provider).await {
        Ok(signals) => {
            signal_snapshots = signals.len();
            if let Err(err) =
                persist_signals(state.services.infra.postgres.as_ref(), &signals).await
            {
                tracing::warn!(error = %err, "failed to persist worker signal snapshots");
                errors.push(format!("signal persistence failed: {err}"));
            }
        }
        Err(err) => {
            tracing::warn!(error = %err, "failed to generate worker signal snapshots");
            errors.push(format!("signal generation failed: {err}"));
        }
    }

    for intent in active_intents {
        match state
            .services
            .execution
            .evaluate_stored_intent(provider, &intent)
            .await
        {
            Ok(proposal) => {
                if proposal.actionable {
                    actionable_intents += 1;
                }
                if let Some(policy) = state
                    .services
                    .agent
                    .active_session_policy_for_intent(intent.id)
                {
                    let result = state.services.execution.build_delegated_execution(
                        &intent,
                        &policy,
                        proposal.clone(),
                    );
                    let execution_log = state.services.agent.record_execution_log_with_policy(
                        &intent,
                        Some(policy.id),
                        proposal,
                    );
                    if let Err(err) = persist_agent_execution_log(
                        state.services.infra.postgres.as_ref(),
                        &execution_log,
                    )
                    .await
                    {
                        errors.push(format!(
                            "intent {} delegated execution log persistence failed: {err}",
                            intent.id
                        ));
                        tracing::warn!(
                            intent_id = %intent.id,
                            policy_id = %policy.id,
                            error = %err,
                            "failed to persist worker delegated execution log"
                        );
                    }
                    if result.executable {
                        delegated_ready_intents += 1;
                        if let Some(updated_policy) =
                            state.services.agent.mark_policy_used(policy.id)
                        {
                            if let Err(err) = persist_agent_execution_policy(
                                state.services.infra.postgres.as_ref(),
                                &updated_policy,
                            )
                            .await
                            {
                                errors.push(format!(
                                    "policy {} usage persistence failed: {err}",
                                    policy.id
                                ));
                                tracing::warn!(
                                    intent_id = %intent.id,
                                    policy_id = %policy.id,
                                    error = %err,
                                    "failed to persist worker policy usage"
                                );
                            }
                        }
                    }
                    tracing::info!(
                        intent_id = %intent.id,
                        policy_id = %policy.id,
                        executable = result.executable,
                        status = result.execution_status,
                        "worker evaluated delegated execution"
                    );
                } else {
                    let execution_log =
                        state.services.agent.record_execution_log(&intent, proposal);
                    if let Err(err) = persist_agent_execution_log(
                        state.services.infra.postgres.as_ref(),
                        &execution_log,
                    )
                    .await
                    {
                        errors.push(format!(
                            "intent {} execution log persistence failed: {err}",
                            intent.id
                        ));
                        tracing::warn!(
                            intent_id = %intent.id,
                            error = %err,
                            "failed to persist worker execution log"
                        );
                    }
                }
                evaluated_intents += 1;
            }
            Err(err) => {
                errors.push(format!("intent {} evaluation failed: {err}", intent.id));
                tracing::warn!(
                    intent_id = %intent.id,
                    wallet_address = intent.wallet_address,
                    error = %err,
                    "failed to evaluate active intent"
                );
            }
        }
    }

    let finished_at = Utc::now();
    let status = if errors.is_empty() {
        JobRunStatus::Success
    } else if evaluated_intents > 0 || signal_snapshots > 0 {
        JobRunStatus::PartialFailure
    } else {
        JobRunStatus::Failed
    };
    let job_run = JobRunRecord {
        job_name: "fast_jobs".to_string(),
        status,
        provider: state.services.provider_name().to_string(),
        summary: json!({
            "jobs": ["signals", "condition_triggers"],
            "signal_snapshots": signal_snapshots,
            "evaluated_intents": evaluated_intents,
            "actionable_intents": actionable_intents,
            "delegated_ready_intents": delegated_ready_intents,
        }),
        started_at,
        finished_at,
        error: (!errors.is_empty()).then(|| errors.join("; ")),
    };
    if let Err(err) = persist_job_run(state.services.infra.postgres.as_ref(), &job_run).await {
        tracing::warn!(error = %err, "failed to persist worker job run");
    }

    info!(
        provider = state.services.provider_name(),
        jobs = "signals,condition_triggers",
        signal_snapshots,
        evaluated_intents,
        actionable_intents,
        delegated_ready_intents,
        status = job_run_status_log_label(status),
        "job tick"
    );
}

fn job_run_status_log_label(status: JobRunStatus) -> &'static str {
    match status {
        JobRunStatus::Success => "success",
        JobRunStatus::PartialFailure => "partial_failure",
        JobRunStatus::Failed => "failed",
    }
}

async fn run_arena_refresh_jobs(state: &AppState) {
    info!(
        provider = state.services.provider_name(),
        jobs = "arena_metrics,resolve_due,leaderboard",
        "job tick"
    );
}

async fn run_prediction_generation_jobs(state: &AppState) {
    info!(
        provider = state.services.provider_name(),
        jobs = "arena_prediction_generation",
        "job tick"
    );
}

async fn run_cohort_jobs(state: &AppState) {
    info!(
        provider = state.services.provider_name(),
        jobs = "wallet_cohort_benchmarks",
        "job tick"
    );
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::{
        config::{AppRole, Settings},
        models::agent::{CreateIntentRequest, CreateSessionPolicyRequest, IntentStatus},
        services::AppServices,
        AppState,
    };

    use super::run_fast_jobs;

    fn test_settings() -> Settings {
        Settings {
            app_env: "test".to_string(),
            app_role: AppRole::Api,
            port: 10000,
            version: "test".to_string(),
            database_url: None,
            run_migrations: false,
            run_internal_jobs: true,
            redis_url: None,
            claude_api_key: None,
            claude_model: "claude-sonnet-4-20250514".to_string(),
            nansen_api_key: None,
            nansen_base_url: None,
            nansen_cli_path: "nansen".to_string(),
            nansen_smart_money_chains: vec![
                "ethereum".to_string(),
                "solana".to_string(),
                "base".to_string(),
            ],
            defillama_enabled: false,
            defillama_base_url: "https://api.llama.fi".to_string(),
            defillama_yields_base_url: "https://yields.llama.fi".to_string(),
            mantle_rpc_url: None,
            mantle_chain_id: 5003,
            aa_provider_stack: "safe-4337-relay-kit".to_string(),
            aa_bundler_url: None,
            aa_entry_point_address: None,
            aa_paymaster_url: None,
            backend_signer_private_key: None,
            mantle_usdc_address: Some("0x0000000000000000000000000000000000000001".to_string()),
            mantle_usdt_address: None,
            mantle_mnt_address: None,
            mantle_meth_address: None,
            mantle_usdy_address: None,
            mantle_wmnt_address: None,
            mantle_weth_address: None,
            mantle_cmeth_address: None,
            approved_strategy_address: None,
            approved_strategy_spender_address: None,
            strategy_deposit_function: "deposit(address,uint256)".to_string(),
            merchant_moe_strategy_address: None,
            merchant_moe_spender_address: None,
            merchant_moe_deposit_function: None,
            agni_strategy_address: None,
            agni_spender_address: None,
            agni_deposit_function: None,
            fluxion_strategy_address: None,
            fluxion_spender_address: None,
            fluxion_deposit_function: None,
            meth_strategy_address: Some("0x0000000000000000000000000000000000000002".to_string()),
            meth_spender_address: None,
            meth_deposit_function: None,
            ondo_usdy_strategy_address: None,
            ondo_usdy_spender_address: None,
            ondo_usdy_deposit_function: None,
            arena_points_address: None,
            prediction_registry_address: None,
            identity_sbt_address: None,
            intent_registry_address: None,
        }
    }

    #[tokio::test]
    async fn fast_job_uses_active_session_policy_for_actionable_intent() {
        let settings = test_settings();
        let services = Arc::new(AppServices::new(settings.clone()).await.unwrap());
        let state = AppState { settings, services };
        let intent = state.services.agent.create_intent(CreateIntentRequest {
            wallet_address: "0x00000000000000000000000000000000000000aa".to_string(),
            raw_intent: "When mETH TVL climbs above 40M, accumulate 25 USDC weekly into mETH"
                .to_string(),
        });
        let active_intent = state
            .services
            .agent
            .update_status(intent.id, IntentStatus::Active)
            .unwrap();
        let policy = state.services.agent.create_session_policy(
            &active_intent,
            CreateSessionPolicyRequest {
                smart_account_address: "0x00000000000000000000000000000000000000bb".to_string(),
                session_key_address: "0x00000000000000000000000000000000000000cc".to_string(),
                allowed_assets: vec!["mETH".to_string(), "USDC".to_string()],
                allowed_protocols: vec!["mETH Protocol".to_string()],
                allowed_contracts: vec!["0x0000000000000000000000000000000000000001".to_string()],
                max_spend_usd: Some(100.0),
                max_transaction_count: Some(2),
                expires_in_days: Some(7),
            },
        );

        run_fast_jobs(&state).await;

        let updated_policy = state.services.agent.get_policy(policy.id).unwrap();
        let logs = state
            .services
            .agent
            .execution_logs_for_intent(active_intent.id);
        assert_eq!(updated_policy.transactions_used, 1);
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].policy_id, Some(policy.id));
        assert_eq!(logs[0].execution_status, "delegated_proposal_ready");
    }
}
