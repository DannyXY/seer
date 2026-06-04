use tokio::time::{interval, MissedTickBehavior};
use tracing::info;

use crate::AppState;

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
    let mut fast_tick = interval(std::time::Duration::from_secs(30));
    let mut arena_tick = interval(std::time::Duration::from_secs(300));
    let mut prediction_tick = interval(std::time::Duration::from_secs(900));
    let mut cohort_tick = interval(std::time::Duration::from_secs(3600));

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
    let provider = state.services.provider.provider().await;
    let active_intents = state.services.agent.active_executable_intents();
    let mut evaluated_intents = 0usize;
    let mut actionable_intents = 0usize;

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
                state.services.agent.record_execution_log(&intent, proposal);
                evaluated_intents += 1;
            }
            Err(err) => {
                tracing::warn!(
                    intent_id = %intent.id,
                    wallet_address = intent.wallet_address,
                    error = %err,
                    "failed to evaluate active intent"
                );
            }
        }
    }

    info!(
        provider = state.services.provider_name(),
        jobs = "signals,condition_triggers",
        evaluated_intents,
        actionable_intents,
        "job tick"
    );
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
