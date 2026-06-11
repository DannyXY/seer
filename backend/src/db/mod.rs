use chrono::{DateTime, Utc};
use redis::Client as RedisClient;
use serde::Serialize;
use serde_json::Value;
use sqlx::{postgres::PgPoolOptions, types::BigDecimal, PgPool};

use crate::config::Settings;
use crate::models::{
    agent::{AgentExecutionLog, AgentIntent, ExecutionPolicy, IntentExecutionMode, IntentStatus},
    arena::{
        ArenaEntry, ArenaEntryStatus, ArenaPrediction, ArenaPosition, ComparisonOperator,
        PredictionStatus,
    },
    settings::UserSettings,
    signals::{Signal, SignalCategory},
};

#[derive(Clone)]
pub struct Infrastructure {
    pub postgres: Option<PgPool>,
    pub redis: Option<RedisClient>,
    pub migrations_enabled: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct InfrastructureStatus {
    pub postgres_configured: bool,
    pub redis_configured: bool,
    pub migrations_enabled: bool,
}

#[derive(Debug, Clone)]
pub struct JobRunRecord {
    pub job_name: String,
    pub status: JobRunStatus,
    pub provider: String,
    pub summary: Value,
    pub started_at: DateTime<Utc>,
    pub finished_at: DateTime<Utc>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobRunStatus {
    Success,
    PartialFailure,
    Failed,
}

impl Infrastructure {
    pub fn from_settings(settings: &Settings) -> anyhow::Result<Self> {
        let postgres = settings
            .database_url
            .as_ref()
            .map(|url| PgPoolOptions::new().max_connections(5).connect_lazy(url))
            .transpose()?;

        let redis = settings
            .redis_url
            .as_ref()
            .map(|url| RedisClient::open(url.as_str()))
            .transpose()?;

        Ok(Self {
            postgres,
            redis,
            migrations_enabled: settings.run_migrations,
        })
    }

    pub fn status(&self) -> InfrastructureStatus {
        InfrastructureStatus {
            postgres_configured: self.postgres.is_some(),
            redis_configured: self.redis.is_some(),
            migrations_enabled: self.migrations_enabled,
        }
    }

    pub async fn run_migrations_if_enabled(&self) -> anyhow::Result<()> {
        if !self.migrations_enabled {
            return Ok(());
        }
        let Some(pool) = &self.postgres else {
            anyhow::bail!("RUN_MIGRATIONS=true requires DATABASE_URL");
        };
        sqlx::migrate!("./migrations").run(pool).await?;
        Ok(())
    }
}

pub async fn persist_agent_intent(
    pool: Option<&PgPool>,
    intent: &AgentIntent,
) -> anyhow::Result<()> {
    let Some(pool) = pool else {
        return Ok(());
    };

    sqlx::query(
        r#"
        INSERT INTO agent_intents (
            id,
            wallet_address,
            raw_intent,
            parsed_intent,
            execution_mode,
            status,
            intent_hash,
            onchain_intent_id,
            created_at,
            updated_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, NOW())
        ON CONFLICT (intent_hash) DO UPDATE SET
            wallet_address = EXCLUDED.wallet_address,
            raw_intent = EXCLUDED.raw_intent,
            parsed_intent = EXCLUDED.parsed_intent,
            execution_mode = EXCLUDED.execution_mode,
            status = EXCLUDED.status,
            onchain_intent_id = EXCLUDED.onchain_intent_id,
            updated_at = NOW()
        "#,
    )
    .bind(intent.id)
    .bind(&intent.wallet_address)
    .bind(&intent.raw_intent)
    .bind(serde_json::to_value(&intent.parsed_intent)?)
    .bind(execution_mode_label(&intent.parsed_intent.trigger.mode))
    .bind(intent_status_label(&intent.status))
    .bind(&intent.intent_hash)
    .bind(intent.onchain_intent_id.map(|id| id as i64))
    .bind(intent.created_at)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn load_intents_for_wallet(
    pool: Option<&PgPool>,
    wallet_address: &str,
) -> anyhow::Result<Vec<crate::models::agent::AgentIntent>> {
    use crate::models::agent::{AgentIntent, IntentStatus, ParsedIntent};
    use sqlx::Row;

    let Some(pool) = pool else {
        return Ok(Vec::new());
    };

    let rows = sqlx::query(
        r#"
        SELECT id, wallet_address, raw_intent, parsed_intent, status,
               intent_hash, onchain_intent_id, created_at
        FROM agent_intents
        WHERE LOWER(wallet_address) = LOWER($1)
        ORDER BY created_at DESC
        "#,
    )
    .bind(wallet_address)
    .fetch_all(pool)
    .await?;

    let mut intents = Vec::new();
    for row in rows {
        let id: uuid::Uuid = row.try_get("id")?;
        let wallet: String = row.try_get("wallet_address")?;
        let raw: String = row.try_get("raw_intent")?;
        let parsed_json: serde_json::Value = row.try_get("parsed_intent")?;
        let status_str: String = row.try_get("status")?;
        let hash: String = row.try_get("intent_hash")?;
        let onchain_id: Option<i64> = row.try_get("onchain_intent_id")?;
        let created_at: chrono::DateTime<Utc> = row.try_get("created_at")?;

        let Ok(parsed_intent) = serde_json::from_value::<ParsedIntent>(parsed_json) else {
            continue; // skip malformed rows
        };
        let status = match status_str.as_str() {
            "ACTIVE" => IntentStatus::Active,
            "PAUSED" => IntentStatus::Paused,
            "COMPLETED" => IntentStatus::Completed,
            "CANCELLED" => IntentStatus::Cancelled,
            _ => IntentStatus::Draft,
        };
        intents.push(AgentIntent {
            id,
            wallet_address: wallet,
            raw_intent: raw,
            parsed_intent,
            status,
            intent_hash: hash,
            onchain_intent_id: onchain_id.map(|v| v as u64),
            created_at,
        });
    }
    Ok(intents)
}

pub async fn load_intent_by_id(
    pool: Option<&PgPool>,
    intent_id: uuid::Uuid,
) -> anyhow::Result<Option<crate::models::agent::AgentIntent>> {
    use crate::models::agent::{AgentIntent, IntentStatus, ParsedIntent};
    use sqlx::Row;

    let Some(pool) = pool else {
        return Ok(None);
    };

    let row = sqlx::query(
        r#"
        SELECT id, wallet_address, raw_intent, parsed_intent, status,
               intent_hash, onchain_intent_id, created_at
        FROM agent_intents
        WHERE id = $1
        "#,
    )
    .bind(intent_id)
    .fetch_optional(pool)
    .await?;

    let Some(row) = row else { return Ok(None) };

    let id: uuid::Uuid = row.try_get("id")?;
    let wallet: String = row.try_get("wallet_address")?;
    let raw: String = row.try_get("raw_intent")?;
    let parsed_json: serde_json::Value = row.try_get("parsed_intent")?;
    let status_str: String = row.try_get("status")?;
    let hash: String = row.try_get("intent_hash")?;
    let onchain_id: Option<i64> = row.try_get("onchain_intent_id")?;
    let created_at: chrono::DateTime<Utc> = row.try_get("created_at")?;

    let Ok(parsed_intent) = serde_json::from_value::<ParsedIntent>(parsed_json) else {
        return Ok(None);
    };
    let status = match status_str.as_str() {
        "ACTIVE" => IntentStatus::Active,
        "PAUSED" => IntentStatus::Paused,
        "COMPLETED" => IntentStatus::Completed,
        "CANCELLED" => IntentStatus::Cancelled,
        _ => IntentStatus::Draft,
    };
    Ok(Some(AgentIntent {
        id,
        wallet_address: wallet,
        raw_intent: raw,
        parsed_intent,
        status,
        intent_hash: hash,
        onchain_intent_id: onchain_id.map(|v| v as u64),
        created_at,
    }))
}

pub async fn persist_agent_execution_log(
    pool: Option<&PgPool>,
    log: &AgentExecutionLog,
) -> anyhow::Result<()> {
    let Some(pool) = pool else {
        return Ok(());
    };

    sqlx::query(
        r#"
        INSERT INTO agent_execution_logs (
            id,
            intent_id,
            policy_id,
            action_type,
            proposed_action,
            execution_status,
            tx_hash,
            reasoning_hash,
            created_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, NULL, $7, $8)
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .bind(log.id)
    .bind(log.intent_id)
    .bind(log.policy_id)
    .bind(&log.action_type)
    .bind(serde_json::to_value(&log.proposal)?)
    .bind(&log.execution_status)
    .bind(&log.reasoning_hash)
    .bind(log.created_at)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn persist_agent_execution_policy(
    pool: Option<&PgPool>,
    policy: &ExecutionPolicy,
) -> anyhow::Result<()> {
    let Some(pool) = pool else {
        return Ok(());
    };

    sqlx::query(
        r#"
        INSERT INTO agent_execution_policies (
            id,
            intent_id,
            wallet_address,
            allowed_assets,
            allowed_protocols,
            max_spend_usd,
            max_transaction_count,
            expires_at,
            status,
            policy_hash,
            created_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, NOW())
        ON CONFLICT (policy_hash) DO UPDATE SET
            wallet_address = EXCLUDED.wallet_address,
            allowed_assets = EXCLUDED.allowed_assets,
            allowed_protocols = EXCLUDED.allowed_protocols,
            max_spend_usd = EXCLUDED.max_spend_usd,
            max_transaction_count = EXCLUDED.max_transaction_count,
            expires_at = EXCLUDED.expires_at,
            status = EXCLUDED.status
        "#,
    )
    .bind(policy.id)
    .bind(policy.intent_id)
    .bind(&policy.wallet_address)
    .bind(serde_json::to_value(&policy.allowed_assets)?)
    .bind(serde_json::to_value(&policy.allowed_protocols)?)
    .bind(
        policy
            .max_spend_usd
            .and_then(|value| BigDecimal::try_from(value).ok()),
    )
    .bind(policy.max_transaction_count.map(|value| value as i32))
    .bind(policy.expires_at)
    .bind(intent_status_label(&policy.status))
    .bind(&policy.policy_hash)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn persist_signals(pool: Option<&PgPool>, signals: &[Signal]) -> anyhow::Result<()> {
    let Some(pool) = pool else {
        return Ok(());
    };

    for signal in signals {
        sqlx::query(
            r#"
            INSERT INTO signals (
                id,
                category,
                headline,
                explanation,
                confidence_score,
                related_wallet,
                related_protocol,
                related_asset,
                source_provider,
                source_data,
                input_facts_hash,
                created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, NULL, $11)
            ON CONFLICT (id) DO UPDATE SET
                category = EXCLUDED.category,
                headline = EXCLUDED.headline,
                explanation = EXCLUDED.explanation,
                confidence_score = EXCLUDED.confidence_score,
                related_wallet = EXCLUDED.related_wallet,
                related_protocol = EXCLUDED.related_protocol,
                related_asset = EXCLUDED.related_asset,
                source_provider = EXCLUDED.source_provider,
                source_data = EXCLUDED.source_data,
                input_facts_hash = EXCLUDED.input_facts_hash,
                created_at = EXCLUDED.created_at
            "#,
        )
        .bind(signal.id)
        .bind(signal_category_label(&signal.category))
        .bind(&signal.headline)
        .bind(&signal.explanation)
        .bind(i32::from(signal.confidence_score))
        .bind(&signal.related_wallet)
        .bind(&signal.related_protocol)
        .bind(&signal.related_asset)
        .bind(&signal.source_provider)
        .bind(&signal.source_data)
        .bind(signal.created_at)
        .execute(pool)
        .await?;
    }

    Ok(())
}

pub async fn persist_job_run(pool: Option<&PgPool>, record: &JobRunRecord) -> anyhow::Result<()> {
    let Some(pool) = pool else {
        return Ok(());
    };

    sqlx::query(
        r#"
        INSERT INTO job_runs (
            job_name,
            status,
            provider,
            summary,
            started_at,
            finished_at,
            error
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        "#,
    )
    .bind(&record.job_name)
    .bind(job_run_status_label(record.status))
    .bind(&record.provider)
    .bind(&record.summary)
    .bind(record.started_at)
    .bind(record.finished_at)
    .bind(&record.error)
    .execute(pool)
    .await?;

    Ok(())
}

fn intent_status_label(status: &IntentStatus) -> &'static str {
    match status {
        IntentStatus::Draft => "DRAFT",
        IntentStatus::Active => "ACTIVE",
        IntentStatus::Paused => "PAUSED",
        IntentStatus::Completed => "COMPLETED",
        IntentStatus::Cancelled => "CANCELLED",
    }
}

fn execution_mode_label(mode: &IntentExecutionMode) -> &'static str {
    match mode {
        IntentExecutionMode::Instant => "INSTANT",
        IntentExecutionMode::Recurring => "RECURRING",
        IntentExecutionMode::Conditional => "CONDITIONAL",
        IntentExecutionMode::RecurringConditional => "RECURRING_CONDITIONAL",
    }
}

fn signal_category_label(category: &SignalCategory) -> &'static str {
    match category {
        SignalCategory::Alpha => "ALPHA",
        SignalCategory::Anomaly => "ANOMALY",
        SignalCategory::Risk => "RISK",
        SignalCategory::Opportunity => "OPPORTUNITY",
    }
}

fn job_run_status_label(status: JobRunStatus) -> &'static str {
    match status {
        JobRunStatus::Success => "SUCCESS",
        JobRunStatus::PartialFailure => "PARTIAL_FAILURE",
        JobRunStatus::Failed => "FAILED",
    }
}

pub async fn persist_lp_position(
    pool: Option<&PgPool>,
    position: &crate::models::lp_position::LpPosition,
) -> anyhow::Result<()> {
    let Some(pool) = pool else {
        tracing::debug!("skipping lp_position persistence: no postgres configured");
        return Ok(());
    };

    let protocol_str = match position.protocol {
        crate::models::lp_position::ProtocolType::AgniFinance => "AgniFinance",
        crate::models::lp_position::ProtocolType::MerchantMoe => "MerchantMoe",
    };

    let (agni_token_id, agni_token0, agni_token1, agni_fee, agni_tick_lower, agni_tick_upper, agni_liquidity) =
        if let Some(agni_pos) = &position.agni_position {
            (
                Some(agni_pos.token_id as i64),
                Some(agni_pos.token0.clone()),
                Some(agni_pos.token1.clone()),
                Some(agni_pos.fee as i32),
                Some(agni_pos.tick_lower as i32),
                Some(agni_pos.tick_upper as i32),
                Some(agni_pos.liquidity.clone()),
            )
        } else {
            (None, None, None, None, None, None, None)
        };

    let (moe_lb_pair, moe_token_x, moe_token_y, moe_bin_step, moe_bin_ids, moe_liquidity_minted) =
        if let Some(moe_pos) = &position.moe_position {
            (
                Some(moe_pos.lb_pair.clone()),
                Some(moe_pos.token_x.clone()),
                Some(moe_pos.token_y.clone()),
                Some(moe_pos.bin_step as i32),
                Some(
                    moe_pos
                        .bin_ids
                        .iter()
                        .map(|id| *id as i64)
                        .collect::<Vec<_>>(),
                ),
                Some(moe_pos.liquidity_minted.clone()),
            )
        } else {
            (None, None, None, None, None, None)
        };

    sqlx::query(
        r#"
        INSERT INTO lp_positions (
            id, wallet_address, protocol,
            agni_token_id, agni_token0, agni_token1, agni_fee, agni_tick_lower, agni_tick_upper, agni_liquidity,
            moe_lb_pair, moe_token_x, moe_token_y, moe_bin_step, moe_bin_ids, moe_liquidity_minted,
            amount_x_added, amount_y_added, intent_hash, tx_hash, created_at, updated_at
        ) VALUES (
            $1, $2, $3,
            $4, $5, $6, $7, $8, $9, $10,
            $11, $12, $13, $14, $15, $16,
            $17, $18, $19, $20, $21, $22
        )
        "#,
    )
    .bind(position.id.to_string())
    .bind(&position.wallet_address)
    .bind(protocol_str)
    .bind(agni_token_id)
    .bind(agni_token0)
    .bind(agni_token1)
    .bind(agni_fee)
    .bind(agni_tick_lower)
    .bind(agni_tick_upper)
    .bind(agni_liquidity)
    .bind(moe_lb_pair)
    .bind(moe_token_x)
    .bind(moe_token_y)
    .bind(moe_bin_step)
    .bind(moe_bin_ids)
    .bind(moe_liquidity_minted)
    .bind(&position.amount_x_added)
    .bind(&position.amount_y_added)
    .bind(&position.intent_hash)
    .bind(&position.tx_hash)
    .bind(position.created_at)
    .bind(position.updated_at)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn get_lp_positions(
    pool: Option<&PgPool>,
    wallet_address: &str,
) -> anyhow::Result<Vec<crate::models::lp_position::LpPosition>> {
    let Some(pool) = pool else {
        tracing::debug!("no postgres configured for lp_positions query");
        return Ok(Vec::new());
    };

    let rows = sqlx::query_as::<_, (String, String)>(
        "SELECT id, protocol FROM lp_positions WHERE wallet_address = $1 ORDER BY created_at DESC",
    )
    .bind(wallet_address)
    .fetch_all(pool)
    .await?;

    Ok(rows.iter().map(|(id, protocol)| {
        crate::models::lp_position::LpPosition {
            id: uuid::Uuid::parse_str(id).unwrap_or_else(|_| uuid::Uuid::new_v4()),
            wallet_address: wallet_address.to_string(),
            protocol: if protocol == "AgniFinance" {
                crate::models::lp_position::ProtocolType::AgniFinance
            } else {
                crate::models::lp_position::ProtocolType::MerchantMoe
            },
            agni_position: None,
            moe_position: None,
            amount_x_added: String::new(),
            amount_y_added: String::new(),
            intent_hash: None,
            tx_hash: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }).collect())
}

// ── Arena predictions ────────────────────────────────────────────────────────

pub async fn persist_arena_prediction(
    pool: Option<&PgPool>,
    prediction: &ArenaPrediction,
) -> anyhow::Result<()> {
    let Some(pool) = pool else { return Ok(()); };

    let op_str = match prediction.comparison_operator {
        ComparisonOperator::GreaterThanOrEqual => "GTE",
        ComparisonOperator::LessThanOrEqual => "LTE",
    };
    let pos_str = match prediction.seer_position {
        ArenaPosition::BackSeer => "BACK_SEER",
        ArenaPosition::ChallengeSeer => "CHALLENGE_SEER",
    };
    let status_str = match prediction.status {
        PredictionStatus::Open => "OPEN",
        PredictionStatus::Locked => "LOCKED",
        PredictionStatus::Resolved => "RESOLVED",
        PredictionStatus::Cancelled => "CANCELLED",
    };

    sqlx::query(r#"
        INSERT INTO arena_predictions (
            id, onchain_prediction_id, claim, metric, target_value,
            comparison_operator, expiry_time, seer_position, seer_confidence,
            reasoning, status, result, final_value, created_at
        )
        VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14)
        ON CONFLICT (id) DO UPDATE SET
            onchain_prediction_id = EXCLUDED.onchain_prediction_id,
            status                = EXCLUDED.status,
            result                = EXCLUDED.result,
            final_value           = EXCLUDED.final_value,
            expiry_time           = EXCLUDED.expiry_time
    "#)
    .bind(prediction.id)
    .bind(prediction.onchain_prediction_id.map(|v| v as i64))
    .bind(&prediction.claim)
    .bind(&prediction.metric)
    .bind(prediction.target_value)
    .bind(op_str)
    .bind(prediction.expiry_time)
    .bind(pos_str)
    .bind(prediction.seer_confidence as i32)
    .bind(&prediction.reasoning)
    .bind(status_str)
    .bind(&prediction.result)
    .bind(prediction.final_value)
    .bind(prediction.created_at)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn load_arena_predictions(
    pool: Option<&PgPool>,
) -> anyhow::Result<Vec<ArenaPrediction>> {
    use sqlx::Row;
    let Some(pool) = pool else { return Ok(Vec::new()); };

    let rows = sqlx::query(r#"
        SELECT id, onchain_prediction_id, claim, metric, target_value,
               comparison_operator, expiry_time, seer_position, seer_confidence,
               reasoning, status, result, final_value, created_at
        FROM arena_predictions
        ORDER BY created_at DESC
    "#)
    .fetch_all(pool)
    .await?;

    let mut out = Vec::new();
    for row in rows {
        let id: uuid::Uuid = row.try_get("id")?;
        let onchain_id: Option<i64> = row.try_get("onchain_prediction_id")?;
        let claim: String = row.try_get("claim")?;
        let metric: String = row.try_get("metric")?;
        let target_value: f64 = row.try_get("target_value")?;
        let op_str: String = row.try_get("comparison_operator")?;
        let expiry_time: DateTime<Utc> = row.try_get("expiry_time")?;
        let pos_str: String = row.try_get("seer_position")?;
        let seer_confidence: i32 = row.try_get("seer_confidence")?;
        let reasoning: String = row.try_get("reasoning")?;
        let status_str: String = row.try_get("status")?;
        let result: Option<String> = row.try_get("result")?;
        let final_value: Option<f64> = row.try_get("final_value")?;
        let created_at: DateTime<Utc> = row.try_get("created_at")?;

        let comparison_operator = match op_str.as_str() {
            "LTE" => ComparisonOperator::LessThanOrEqual,
            _ => ComparisonOperator::GreaterThanOrEqual,
        };
        let seer_position = match pos_str.as_str() {
            "CHALLENGE_SEER" => ArenaPosition::ChallengeSeer,
            _ => ArenaPosition::BackSeer,
        };
        let status = match status_str.as_str() {
            "LOCKED" => PredictionStatus::Locked,
            "RESOLVED" => PredictionStatus::Resolved,
            "CANCELLED" => PredictionStatus::Cancelled,
            _ => PredictionStatus::Open,
        };

        out.push(ArenaPrediction {
            id,
            onchain_prediction_id: onchain_id.map(|v| v as u64),
            claim,
            metric,
            target_value,
            comparison_operator,
            expiry_time,
            seer_position,
            seer_confidence: seer_confidence as u8,
            reasoning,
            status,
            result,
            final_value,
            created_at,
        });
    }
    Ok(out)
}

// ── Arena entries ─────────────────────────────────────────────────────────────

pub async fn persist_arena_entry(
    pool: Option<&PgPool>,
    entry: &ArenaEntry,
) -> anyhow::Result<()> {
    let Some(pool) = pool else { return Ok(()); };

    let pos_str = match entry.user_position {
        ArenaPosition::BackSeer => "BACK_SEER",
        ArenaPosition::ChallengeSeer => "CHALLENGE_SEER",
    };
    let status_str = match entry.status {
        ArenaEntryStatus::Active => "ACTIVE",
        ArenaEntryStatus::Resolved => "RESOLVED",
        ArenaEntryStatus::Cancelled => "CANCELLED",
    };

    sqlx::query(r#"
        INSERT INTO arena_entries (
            id, prediction_id, wallet_address, user_position,
            points_committed, status, points_delta, tx_hash,
            created_at, resolved_at
        )
        VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)
        ON CONFLICT (id) DO UPDATE SET
            status       = EXCLUDED.status,
            points_delta = EXCLUDED.points_delta,
            resolved_at  = EXCLUDED.resolved_at
    "#)
    .bind(entry.id)
    .bind(entry.prediction_id)
    .bind(&entry.wallet_address)
    .bind(pos_str)
    .bind(entry.points_committed as i32)
    .bind(status_str)
    .bind(entry.points_delta)
    .bind(&entry.tx_hash)
    .bind(entry.created_at)
    .bind(entry.resolved_at)
    .execute(pool)
    .await?;

    Ok(())
}

/// Load every arena entry. Used by the resolution job so entries created
/// before a restart still settle and persist correctly.
pub async fn load_all_arena_entries(pool: Option<&PgPool>) -> anyhow::Result<Vec<ArenaEntry>> {
    use sqlx::Row;
    let Some(pool) = pool else { return Ok(Vec::new()); };

    let rows = sqlx::query(r#"
        SELECT id, prediction_id, wallet_address, user_position,
               points_committed, status, points_delta, tx_hash,
               created_at, resolved_at
        FROM arena_entries
        ORDER BY created_at DESC
    "#)
    .fetch_all(pool)
    .await?;

    let mut out = Vec::new();
    for row in rows {
        let pos_str: String = row.try_get("user_position")?;
        let status_str: String = row.try_get("status")?;
        let points_committed: i32 = row.try_get("points_committed")?;
        out.push(ArenaEntry {
            id: row.try_get("id")?,
            prediction_id: row.try_get("prediction_id")?,
            wallet_address: row.try_get("wallet_address")?,
            user_position: match pos_str.as_str() {
                "CHALLENGE_SEER" => ArenaPosition::ChallengeSeer,
                _ => ArenaPosition::BackSeer,
            },
            points_committed: points_committed as u32,
            status: match status_str.as_str() {
                "RESOLVED" => ArenaEntryStatus::Resolved,
                "CANCELLED" => ArenaEntryStatus::Cancelled,
                _ => ArenaEntryStatus::Active,
            },
            points_delta: row.try_get("points_delta")?,
            tx_hash: row.try_get("tx_hash")?,
            created_at: row.try_get("created_at")?,
            resolved_at: row.try_get("resolved_at")?,
        });
    }
    Ok(out)
}

pub async fn load_entries_for_wallet(
    pool: Option<&PgPool>,
    wallet_address: &str,
) -> anyhow::Result<Vec<ArenaEntry>> {
    use sqlx::Row;
    let Some(pool) = pool else { return Ok(Vec::new()); };

    let rows = sqlx::query(r#"
        SELECT id, prediction_id, wallet_address, user_position,
               points_committed, status, points_delta, tx_hash,
               created_at, resolved_at
        FROM arena_entries
        WHERE LOWER(wallet_address) = LOWER($1)
        ORDER BY created_at DESC
    "#)
    .bind(wallet_address)
    .fetch_all(pool)
    .await?;

    let mut out = Vec::new();
    for row in rows {
        let id: uuid::Uuid = row.try_get("id")?;
        let prediction_id: uuid::Uuid = row.try_get("prediction_id")?;
        let wallet: String = row.try_get("wallet_address")?;
        let pos_str: String = row.try_get("user_position")?;
        let points_committed: i32 = row.try_get("points_committed")?;
        let status_str: String = row.try_get("status")?;
        let points_delta: Option<i32> = row.try_get("points_delta")?;
        let tx_hash: Option<String> = row.try_get("tx_hash")?;
        let created_at: DateTime<Utc> = row.try_get("created_at")?;
        let resolved_at: Option<DateTime<Utc>> = row.try_get("resolved_at")?;

        let user_position = match pos_str.as_str() {
            "CHALLENGE_SEER" => ArenaPosition::ChallengeSeer,
            _ => ArenaPosition::BackSeer,
        };
        let status = match status_str.as_str() {
            "RESOLVED" => ArenaEntryStatus::Resolved,
            "CANCELLED" => ArenaEntryStatus::Cancelled,
            _ => ArenaEntryStatus::Active,
        };

        out.push(ArenaEntry {
            id,
            prediction_id,
            wallet_address: wallet,
            user_position,
            points_committed: points_committed as u32,
            status,
            points_delta,
            tx_hash,
            created_at,
            resolved_at,
        });
    }
    Ok(out)
}

// ── Agent execution policies ──────────────────────────────────────────────────

pub async fn load_policies_for_wallet(
    pool: Option<&PgPool>,
    wallet_address: &str,
) -> anyhow::Result<Vec<ExecutionPolicy>> {
    use sqlx::Row;
    let Some(pool) = pool else { return Ok(Vec::new()); };

    let rows = sqlx::query(r#"
        SELECT id, intent_id, wallet_address, allowed_assets, allowed_protocols,
               max_spend_usd, max_transaction_count, expires_at, status,
               policy_hash, created_at
        FROM agent_execution_policies
        WHERE LOWER(wallet_address) = LOWER($1)
        ORDER BY created_at DESC
    "#)
    .bind(wallet_address)
    .fetch_all(pool)
    .await?;

    let mut out = Vec::new();
    for row in rows {
        let id: uuid::Uuid = row.try_get("id")?;
        let intent_id: uuid::Uuid = row.try_get("intent_id")?;
        let wallet: String = row.try_get("wallet_address")?;
        let assets_json: serde_json::Value = row.try_get("allowed_assets")?;
        let protocols_json: serde_json::Value = row.try_get("allowed_protocols")?;
        let max_spend: Option<BigDecimal> = row.try_get("max_spend_usd")?;
        let max_tx_count: Option<i32> = row.try_get("max_transaction_count")?;
        let expires_at: DateTime<Utc> = row.try_get("expires_at")?;
        let status_str: String = row.try_get("status")?;
        let policy_hash: String = row.try_get("policy_hash")?;

        let allowed_assets: Vec<String> =
            serde_json::from_value(assets_json).unwrap_or_default();
        let allowed_protocols: Vec<String> =
            serde_json::from_value(protocols_json).unwrap_or_default();
        let max_spend_usd = max_spend
            .and_then(|v| v.to_string().parse::<f64>().ok());
        let status = match status_str.as_str() {
            "ACTIVE" => IntentStatus::Active,
            "PAUSED" => IntentStatus::Paused,
            "COMPLETED" => IntentStatus::Completed,
            "CANCELLED" => IntentStatus::Cancelled,
            _ => IntentStatus::Draft,
        };

        out.push(ExecutionPolicy {
            id,
            intent_id,
            wallet_address: wallet,
            smart_account_address: None,
            session_key_address: None,
            allowed_assets,
            allowed_protocols,
            allowed_contracts: Vec::new(),
            max_spend_usd,
            max_transaction_count: max_tx_count.map(|v| v as u32),
            transactions_used: 0,
            revoked_at: None,
            expires_at,
            status,
            policy_hash,
        });
    }
    Ok(out)
}

// ── Agent execution logs ──────────────────────────────────────────────────────

pub async fn load_logs_for_intent(
    pool: Option<&PgPool>,
    intent_id: uuid::Uuid,
) -> anyhow::Result<Vec<AgentExecutionLog>> {
    use sqlx::Row;
    let Some(pool) = pool else { return Ok(Vec::new()); };

    let rows = sqlx::query(r#"
        SELECT l.id, l.intent_id, l.policy_id, l.action_type, l.proposed_action,
               l.execution_status, l.reasoning_hash, l.created_at,
               i.wallet_address
        FROM agent_execution_logs l
        JOIN agent_intents i ON i.id = l.intent_id
        WHERE l.intent_id = $1
        ORDER BY l.created_at ASC
    "#)
    .bind(intent_id)
    .fetch_all(pool)
    .await?;

    let mut out = Vec::new();
    for row in rows {
        let id: uuid::Uuid = row.try_get("id")?;
        let intent_id_val: uuid::Uuid = row.try_get("intent_id")?;
        let policy_id: Option<uuid::Uuid> = row.try_get("policy_id")?;
        let action_type: String = row.try_get("action_type")?;
        let proposed_json: serde_json::Value = row.try_get("proposed_action")?;
        let execution_status: String = row.try_get("execution_status")?;
        let reasoning_hash: Option<String> = row.try_get("reasoning_hash")?;
        let created_at: DateTime<Utc> = row.try_get("created_at")?;
        let wallet_address: String = row.try_get("wallet_address")?;

        let Ok(proposal) = serde_json::from_value(proposed_json) else {
            continue;
        };

        out.push(AgentExecutionLog {
            id,
            intent_id: intent_id_val,
            policy_id,
            wallet_address,
            action_type,
            proposal,
            execution_status,
            reasoning_hash: reasoning_hash.unwrap_or_default(),
            created_at,
        });
    }
    Ok(out)
}

// ── User settings ─────────────────────────────────────────────────────────────

pub async fn persist_user_settings(
    pool: Option<&PgPool>,
    wallet_address: &str,
    settings: &UserSettings,
) -> anyhow::Result<()> {
    let Some(pool) = pool else { return Ok(()); };

    sqlx::query(r#"
        INSERT INTO user_settings (wallet_address, settings, updated_at)
        VALUES ($1, $2, NOW())
        ON CONFLICT (wallet_address) DO UPDATE SET
            settings   = EXCLUDED.settings,
            updated_at = NOW()
    "#)
    .bind(wallet_address.to_lowercase())
    .bind(serde_json::to_value(settings)?)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn load_user_settings(
    pool: Option<&PgPool>,
    wallet_address: &str,
) -> anyhow::Result<Option<UserSettings>> {
    use sqlx::Row;
    let Some(pool) = pool else { return Ok(None); };

    let row = sqlx::query(
        "SELECT settings FROM user_settings WHERE wallet_address = LOWER($1)",
    )
    .bind(wallet_address)
    .fetch_optional(pool)
    .await?;

    let Some(row) = row else { return Ok(None); };
    let json: serde_json::Value = row.try_get("settings")?;
    Ok(serde_json::from_value(json).ok())
}

#[cfg(test)]
mod tests {
    use crate::{
        config::{AppRole, Settings},
        db::{
            execution_mode_label, intent_status_label, job_run_status_label,
            persist_agent_execution_log, persist_agent_execution_policy, persist_agent_intent,
            persist_job_run, persist_signals, signal_category_label, Infrastructure, JobRunRecord,
            JobRunStatus,
        },
        models::agent::{CreateIntentRequest, IntentExecutionMode, IntentStatus},
        models::signals::SignalCategory,
        services::{agent::AgentService, data_provider::MockProvider, signal_engine::SignalEngine},
    };
    use chrono::Utc;
    use serde_json::json;

    #[tokio::test]
    async fn persistence_helpers_noop_without_postgres() {
        let service = AgentService::new();
        let intent = service.create_intent(CreateIntentRequest {
            wallet_address: "0x123".to_string(),
            raw_intent: "When mETH TVL crosses 50M, buy 25 USDC weekly".to_string(),
        });
        persist_agent_intent(None, &intent).await.unwrap();

        let proposal = crate::models::execution::ExecutionProposal {
            actionable: false,
            action: intent.parsed_intent.action.clone(),
            wallet_address: intent.wallet_address.clone(),
            chain_id: 5003,
            network: "mantle-testnet".to_string(),
            conditions: Vec::new(),
            allowance_check: None,
            transaction_draft: None,
            required_authorization: "user-signed transaction".to_string(),
            protocol_operation: None,
        };
        let log = service.record_execution_log(&intent, proposal);
        persist_agent_execution_log(None, &log).await.unwrap();

        let policy = service.create_policy(&intent);
        persist_agent_execution_policy(None, &policy).await.unwrap();

        let signals = SignalEngine::new().generate(&MockProvider).await.unwrap();
        persist_signals(None, &signals).await.unwrap();

        let now = Utc::now();
        let job = JobRunRecord {
            job_name: "test_job".to_string(),
            status: JobRunStatus::Success,
            provider: "mock".to_string(),
            summary: json!({ "ok": true }),
            started_at: now,
            finished_at: now,
            error: None,
        };
        persist_job_run(None, &job).await.unwrap();
    }

    #[test]
    fn persistence_labels_match_migration_constraints() {
        assert_eq!(intent_status_label(&IntentStatus::Draft), "DRAFT");
        assert_eq!(intent_status_label(&IntentStatus::Active), "ACTIVE");
        assert_eq!(intent_status_label(&IntentStatus::Paused), "PAUSED");
        assert_eq!(intent_status_label(&IntentStatus::Completed), "COMPLETED");
        assert_eq!(intent_status_label(&IntentStatus::Cancelled), "CANCELLED");
        assert_eq!(
            execution_mode_label(&IntentExecutionMode::Instant),
            "INSTANT"
        );
        assert_eq!(
            execution_mode_label(&IntentExecutionMode::Recurring),
            "RECURRING"
        );
        assert_eq!(
            execution_mode_label(&IntentExecutionMode::Conditional),
            "CONDITIONAL"
        );
        assert_eq!(
            execution_mode_label(&IntentExecutionMode::RecurringConditional),
            "RECURRING_CONDITIONAL"
        );
        assert_eq!(signal_category_label(&SignalCategory::Alpha), "ALPHA");
        assert_eq!(signal_category_label(&SignalCategory::Anomaly), "ANOMALY");
        assert_eq!(signal_category_label(&SignalCategory::Risk), "RISK");
        assert_eq!(
            signal_category_label(&SignalCategory::Opportunity),
            "OPPORTUNITY"
        );
        assert_eq!(job_run_status_label(JobRunStatus::Success), "SUCCESS");
        assert_eq!(
            job_run_status_label(JobRunStatus::PartialFailure),
            "PARTIAL_FAILURE"
        );
        assert_eq!(job_run_status_label(JobRunStatus::Failed), "FAILED");
    }

    #[tokio::test]
    async fn migrations_noop_when_disabled_without_postgres() {
        let infra = Infrastructure {
            postgres: None,
            redis: None,
            migrations_enabled: false,
        };

        infra.run_migrations_if_enabled().await.unwrap();
        assert!(!infra.status().migrations_enabled);
    }

    #[test]
    fn infrastructure_status_reports_migration_flag() {
        let settings = Settings {
            app_env: "test".to_string(),
            app_role: AppRole::Api,
            port: 10000,
            version: "test".to_string(),
            database_url: None,
            run_migrations: true,
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
            defillama_enabled: true,
            defillama_base_url: "https://api.llama.fi".to_string(),
            defillama_yields_base_url: "https://yields.llama.fi".to_string(),
            mantle_rpc_url: None,
            mantle_chain_id: 5003,
            aa_provider_stack: "safe-4337-relay-kit".to_string(),
            aa_bundler_url: None,
            aa_entry_point_address: None,
            aa_paymaster_url: None,
            backend_signer_private_key: None,
            mantle_usdc_address: None,
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
            meth_strategy_address: None,
            meth_spender_address: None,
            meth_deposit_function: None,
            ondo_usdy_strategy_address: None,
            ondo_usdy_spender_address: None,
            ondo_usdy_deposit_function: None,
            arena_points_address: None,
            prediction_registry_address: None,
            identity_sbt_address: None,
            intent_registry_address: None,
        };
        let infra = Infrastructure::from_settings(&settings).unwrap();

        assert!(infra.status().migrations_enabled);
    }
}
