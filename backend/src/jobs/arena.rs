use chrono::{Duration, Utc};
use serde_json::json;
use uuid::Uuid;

use crate::{
    db::{
        load_all_arena_entries, load_arena_predictions, persist_arena_entry,
        persist_arena_prediction,
    },
    models::arena::{ArenaPosition, ArenaPrediction, ComparisonOperator, PredictionStatus},
    AppState,
};

/// Contract enum order: 0 = Void, 1 = SeerCorrect, 2 = SeerIncorrect.
const OUTCOME_SEER_CORRECT: u8 = 1;
const OUTCOME_SEER_INCORRECT: u8 = 2;

pub struct ArenaResolutionSummary {
    pub resolved: Vec<Uuid>,
    pub onchain_resolved: usize,
    pub entries_settled: usize,
    pub errors: Vec<String>,
}

/// Hydrate arena state from the database so the in-memory store survives restarts
/// and the worker process sees predictions/entries created via the API process.
async fn hydrate_from_db(state: &AppState) {
    let pool = state.services.infra.postgres.as_ref();
    match load_arena_predictions(pool).await {
        Ok(predictions) if !predictions.is_empty() => {
            state.services.arena.seed_predictions(predictions)
        }
        Ok(_) => {}
        Err(err) => tracing::warn!(error = %err, "arena: failed to hydrate predictions from db"),
    }
    match load_all_arena_entries(pool).await {
        Ok(entries) if !entries.is_empty() => state.services.arena.seed_entries(entries),
        Ok(_) => {}
        Err(err) => tracing::warn!(error = %err, "arena: failed to hydrate entries from db"),
    }
}

/// Resolve every expired open prediction: fetch the final metric value, settle
/// entries in memory, persist results to the database, and mirror the
/// resolution on-chain (resolvePrediction + settleEntry per entrant).
pub async fn resolve_due_predictions(state: &AppState) -> ArenaResolutionSummary {
    hydrate_from_db(state).await;

    let provider = state.services.provider.provider().await;
    let now = Utc::now();
    let mut errors = Vec::new();

    // Fetch final metric values only for predictions that are due.
    let due: Vec<ArenaPrediction> = state
        .services
        .arena
        .predictions()
        .into_iter()
        .filter(|p| matches!(p.status, PredictionStatus::Open) && p.expiry_time <= now)
        .collect();

    let mut metric_values = std::collections::HashMap::new();
    for prediction in &due {
        if metric_values.contains_key(&prediction.metric) {
            continue;
        }
        // Metric format: "protocol.tvl_usd:<ProtocolName>" or a bare metric name.
        let protocol_name = prediction
            .metric
            .split(':')
            .nth(1)
            .unwrap_or(&prediction.metric);
        match provider.get_protocol_metrics(protocol_name).await {
            Ok(metrics) => {
                metric_values.insert(prediction.metric.clone(), metrics.tvl_usd);
            }
            Err(err) => {
                errors.push(format!(
                    "metric fetch failed for {}: {err}",
                    prediction.metric
                ));
            }
        }
    }

    let resolved_ids = state
        .services
        .arena
        .resolve_expired(|metric_key| metric_values.get(metric_key).copied());

    let pool = state.services.infra.postgres.as_ref();
    let mut onchain_resolved = 0usize;
    let mut entries_settled = 0usize;

    for prediction_id in &resolved_ids {
        let Some(prediction) = state.services.arena.get_prediction(*prediction_id) else {
            continue;
        };
        if let Err(err) = persist_arena_prediction(pool, &prediction).await {
            errors.push(format!(
                "prediction {prediction_id} persistence failed: {err}"
            ));
        }

        let entries = state.services.arena.entries_for_prediction(*prediction_id);
        for entry in &entries {
            if let Err(err) = persist_arena_entry(pool, entry).await {
                errors.push(format!("entry {} persistence failed: {err}", entry.id));
            }
        }

        // Mirror the resolution on-chain so locked points are released.
        let Some(onchain_id) = prediction.onchain_prediction_id else {
            continue;
        };
        if !state.services.contracts.is_configured() {
            continue;
        }
        let outcome = match prediction.result.as_deref() {
            Some("SeerCorrect") => OUTCOME_SEER_CORRECT,
            _ => OUTCOME_SEER_INCORRECT,
        };
        let final_value = prediction.final_value.unwrap_or(0.0).max(0.0) as u64;
        match state
            .services
            .contracts
            .resolve_prediction_on_chain(onchain_id, outcome, final_value)
            .await
        {
            Ok(tx_hash) => {
                onchain_resolved += 1;
                tracing::info!(
                    prediction_id = %prediction_id,
                    onchain_id,
                    tx_hash,
                    "arena: prediction resolved on-chain"
                );
                for entry in &entries {
                    match state
                        .services
                        .contracts
                        .settle_entry_on_chain(onchain_id, &entry.wallet_address)
                        .await
                    {
                        Ok(_) => entries_settled += 1,
                        Err(err) => {
                            errors.push(format!(
                                "settleEntry failed for {} on prediction {onchain_id}: {err}",
                                entry.wallet_address
                            ));
                        }
                    }
                }
            }
            Err(err) => {
                errors.push(format!(
                    "resolvePrediction failed for on-chain id {onchain_id}: {err}"
                ));
            }
        }
    }

    ArenaResolutionSummary {
        resolved: resolved_ids,
        onchain_resolved,
        entries_settled,
        errors,
    }
}

/// Keep the arena alive: when no open, unexpired prediction exists, generate a
/// new one from live protocol metrics, register it on-chain, and persist it.
pub async fn generate_prediction_if_needed(
    state: &AppState,
) -> anyhow::Result<Option<ArenaPrediction>> {
    hydrate_from_db(state).await;

    let now = Utc::now();
    let has_open = state
        .services
        .arena
        .predictions()
        .iter()
        .any(|p| matches!(p.status, PredictionStatus::Open) && p.expiry_time > now);
    if has_open {
        return Ok(None);
    }

    let provider = state.services.provider.provider().await;
    let candidates = ["mETH Protocol", "Merchant Moe", "Agni Finance", "Lendle"];
    let mut metrics = None;
    for protocol in candidates {
        match provider.get_protocol_metrics(protocol).await {
            Ok(m) if m.tvl_usd > 0.0 => {
                metrics = Some(m);
                break;
            }
            Ok(_) => {}
            Err(err) => {
                tracing::debug!(protocol, error = %err, "arena generation: metrics unavailable")
            }
        }
    }
    let Some(metrics) = metrics else {
        anyhow::bail!("no protocol metrics available to generate a prediction");
    };

    // Seer backs momentum: rising TVL -> "stays above" slightly below current,
    // falling TVL -> "stays below" slightly above current.
    let rising = metrics.tvl_change_24h_pct >= 0.0;
    let (comparison_operator, target_value, direction_label) = if rising {
        (
            ComparisonOperator::GreaterThanOrEqual,
            round_to_significant(metrics.tvl_usd * 0.97),
            "stay above",
        )
    } else {
        (
            ComparisonOperator::LessThanOrEqual,
            round_to_significant(metrics.tvl_usd * 1.03),
            "stay below",
        )
    };

    let metric_key = format!("protocol.tvl_usd:{}", metrics.protocol);
    let claim = format!(
        "{} TVL will {} {} for the next 24 hours",
        metrics.protocol,
        direction_label,
        format_usd(target_value),
    );
    let confidence = (60.0 + metrics.tvl_change_24h_pct.abs().min(5.0) * 4.0) as u8;

    let reasoning = match state
        .services
        .claude
        .explain_prediction(&metric_key, target_value)
        .await
    {
        Ok(reasoning) => reasoning,
        Err(err) => {
            tracing::warn!(error = %err, "arena generation: claude reasoning unavailable");
            format!(
                "{} TVL is {} with a 24h change of {:+.1}%. Seer expects the trend to hold through expiry.",
                metrics.protocol,
                format_usd(metrics.tvl_usd),
                metrics.tvl_change_24h_pct,
            )
        }
    };

    let mut prediction = ArenaPrediction {
        id: Uuid::new_v4(),
        onchain_prediction_id: None,
        claim,
        metric: metric_key,
        target_value,
        comparison_operator,
        expiry_time: now + Duration::hours(24),
        seer_position: ArenaPosition::BackSeer,
        seer_confidence: confidence.clamp(55, 90),
        reasoning,
        status: PredictionStatus::Open,
        result: None,
        final_value: None,
        created_at: now,
    };

    // Register on-chain first so the entry calldata path works immediately.
    // An RPC failure still leaves a usable off-chain prediction.
    if state.services.contracts.is_configured() {
        let op = match prediction.comparison_operator {
            ComparisonOperator::GreaterThanOrEqual => 0u8,
            ComparisonOperator::LessThanOrEqual => 1u8,
        };
        let data_key = ethers_core::utils::keccak256(prediction.metric.as_bytes());
        match state
            .services
            .contracts
            .create_prediction_on_chain(
                &prediction.claim,
                data_key,
                prediction.target_value as u64,
                prediction.expiry_time.timestamp() as u64,
                op,
                0, // BackSeer
            )
            .await
        {
            Ok(onchain_id) => prediction.onchain_prediction_id = Some(onchain_id),
            Err(err) => {
                tracing::warn!(error = %err, "arena generation: on-chain registration failed")
            }
        }
    }

    state.services.arena.seed_predictions(vec![prediction.clone()]);
    if let Err(err) =
        persist_arena_prediction(state.services.infra.postgres.as_ref(), &prediction).await
    {
        tracing::warn!(error = %err, "arena generation: failed to persist prediction");
    }

    tracing::info!(
        prediction_id = %prediction.id,
        onchain_id = ?prediction.onchain_prediction_id,
        claim = %prediction.claim,
        "arena: generated new prediction"
    );

    Ok(Some(prediction))
}

impl ArenaResolutionSummary {
    pub fn to_json(&self) -> serde_json::Value {
        json!({
            "resolved": self.resolved,
            "onchain_resolved": self.onchain_resolved,
            "entries_settled": self.entries_settled,
            "errors": self.errors,
        })
    }
}

fn round_to_significant(value: f64) -> f64 {
    if value <= 0.0 {
        return 0.0;
    }
    let magnitude = 10f64.powf(value.log10().floor() - 2.0);
    (value / magnitude).round() * magnitude
}

fn format_usd(value: f64) -> String {
    if value >= 1_000_000_000.0 {
        format!("${:.1}B", value / 1_000_000_000.0)
    } else if value >= 1_000_000.0 {
        format!("${:.1}M", value / 1_000_000.0)
    } else if value >= 1_000.0 {
        format!("${:.0}K", value / 1_000.0)
    } else {
        format!("${value:.0}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rounds_to_three_significant_digits() {
        assert_eq!(round_to_significant(41_237_894.0), 41_200_000.0);
        assert_eq!(round_to_significant(987.6), 988.0);
        assert_eq!(round_to_significant(0.0), 0.0);
    }

    #[test]
    fn formats_usd_scales() {
        assert_eq!(format_usd(41_200_000.0), "$41.2M");
        assert_eq!(format_usd(450_000.0), "$450K");
        assert_eq!(format_usd(1_300_000_000.0), "$1.3B");
    }
}
