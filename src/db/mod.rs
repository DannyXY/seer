use redis::Client as RedisClient;
use serde::Serialize;
use sqlx::{postgres::PgPoolOptions, PgPool};

use crate::config::Settings;
use crate::models::agent::{AgentExecutionLog, AgentIntent, IntentExecutionMode, IntentStatus};

#[derive(Clone)]
pub struct Infrastructure {
    pub postgres: Option<PgPool>,
    pub redis: Option<RedisClient>,
}

#[derive(Debug, Clone, Serialize)]
pub struct InfrastructureStatus {
    pub postgres_configured: bool,
    pub redis_configured: bool,
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

        Ok(Self { postgres, redis })
    }

    pub fn status(&self) -> InfrastructureStatus {
        InfrastructureStatus {
            postgres_configured: self.postgres.is_some(),
            redis_configured: self.redis.is_some(),
        }
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
        VALUES ($1, $2, NULL, $3, $4, $5, NULL, $6, $7)
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .bind(log.id)
    .bind(log.intent_id)
    .bind(&log.action_type)
    .bind(serde_json::to_value(&log.proposal)?)
    .bind(&log.execution_status)
    .bind(&log.reasoning_hash)
    .bind(log.created_at)
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

#[cfg(test)]
mod tests {
    use crate::{
        db::{
            execution_mode_label, intent_status_label, persist_agent_execution_log,
            persist_agent_intent,
        },
        models::agent::{CreateIntentRequest, IntentExecutionMode, IntentStatus},
        services::agent::AgentService,
    };

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
            transaction_draft: None,
            required_authorization: "user-signed transaction".to_string(),
        };
        let log = service.record_execution_log(&intent, proposal);
        persist_agent_execution_log(None, &log).await.unwrap();
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
    }
}
