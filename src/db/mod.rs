use redis::Client as RedisClient;
use serde::Serialize;
use sqlx::{postgres::PgPoolOptions, types::BigDecimal, PgPool};

use crate::config::Settings;
use crate::models::agent::{
    AgentExecutionLog, AgentIntent, ExecutionPolicy, IntentExecutionMode, IntentStatus,
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
        config::{AppRole, Settings},
        db::{
            execution_mode_label, intent_status_label, persist_agent_execution_log,
            persist_agent_execution_policy, persist_agent_intent, Infrastructure,
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

        let policy = service.create_policy(&intent);
        persist_agent_execution_policy(None, &policy).await.unwrap();
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
            redis_url: None,
            claude_api_key: None,
            claude_model: "claude-sonnet-4-20250514".to_string(),
            nansen_api_key: None,
            nansen_base_url: None,
            nansen_cli_path: "nansen".to_string(),
            mantle_rpc_url: None,
            mantle_chain_id: 5003,
            aa_bundler_url: None,
            backend_signer_private_key: None,
            mantle_usdc_address: None,
            mantle_usdt_address: None,
            mantle_mnt_address: None,
            mantle_meth_address: None,
            approved_strategy_address: None,
            approved_strategy_spender_address: None,
            strategy_deposit_function: "deposit(address,uint256)".to_string(),
            merchant_moe_strategy_address: None,
            merchant_moe_spender_address: None,
            merchant_moe_deposit_function: None,
            lendle_strategy_address: None,
            lendle_spender_address: None,
            lendle_deposit_function: None,
            agni_strategy_address: None,
            agni_spender_address: None,
            agni_deposit_function: None,
            meth_strategy_address: None,
            meth_spender_address: None,
            meth_deposit_function: None,
            arena_points_address: None,
            prediction_registry_address: None,
            identity_sbt_address: None,
            intent_registry_address: None,
        };
        let infra = Infrastructure::from_settings(&settings).unwrap();

        assert!(infra.status().migrations_enabled);
    }
}
