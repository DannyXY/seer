pub mod abi_encoder;
pub mod agent;
pub mod arena;
pub mod auth;
pub mod claude;
pub mod contracts;
pub mod data_provider;
pub mod execution;
pub mod identity;
pub mod quoter;
pub mod settings;
pub mod signal_engine;
pub mod wallet;

use crate::config::Settings;
use crate::db::Infrastructure;

pub struct AppServices {
    pub infra: Infrastructure,
    pub auth: auth::AuthService,
    pub provider: data_provider::ProviderRegistry,
    pub claude: claude::ClaudeService,
    pub wallet: wallet::WalletService,
    pub signals: signal_engine::SignalEngine,
    pub identity: identity::IdentityService,
    pub arena: arena::ArenaService,
    pub agent: agent::AgentService,
    pub settings: settings::SettingsService,
    pub execution: execution::ExecutionService,
    pub contracts: contracts::ContractService,
}

impl AppServices {
    pub async fn new(settings: Settings) -> anyhow::Result<Self> {
        let infra = Infrastructure::from_settings(&settings)?;
        infra.run_migrations_if_enabled().await?;
        let provider = data_provider::ProviderRegistry::new(settings.clone());
        let claude = claude::ClaudeService::new(settings.clone());
        let arena = arena::ArenaService::new();
        let contracts = contracts::ContractService::new(settings.clone());

        // Spawn background task: register the seed prediction on Mantle Sepolia.
        // If the contract isn't configured this is a no-op (the error is just logged).
        {
            use chrono::Utc;
            let arena_ref = arena.predictions();
            if let Some(seed) = arena_ref.into_iter().find(|p| p.onchain_prediction_id.is_none()) {
                let seed_id = seed.id;
                let claim = seed.claim.clone();
                let target = seed.target_value as u64;
                let expiry = seed.expiry_time.timestamp() as u64;
                let op = match seed.comparison_operator {
                    crate::models::arena::ComparisonOperator::GreaterThanOrEqual => 0u8,
                    crate::models::arena::ComparisonOperator::LessThanOrEqual => 1u8,
                };
                let pos = match seed.seer_position {
                    crate::models::arena::ArenaPosition::BackSeer => 0u8,
                    crate::models::arena::ArenaPosition::ChallengeSeer => 1u8,
                };
                // data_key: keccak256 of the metric string, truncated to 32 bytes
                let data_key_bytes = ethers_core::utils::keccak256(seed.metric.as_bytes());
                let arena_svc = arena.clone();
                let contracts_svc = contracts.clone();
                tokio::spawn(async move {
                    match contracts_svc.create_prediction_on_chain(&claim, data_key_bytes, target, expiry, op, pos).await {
                        Ok(onchain_id) => {
                            tracing::info!("Seed prediction registered on-chain with id={onchain_id}");
                            arena_svc.register_onchain_prediction_id(seed_id, onchain_id);
                        }
                        Err(err) => tracing::warn!("Failed to register seed prediction on-chain: {err}"),
                    }
                });
            }
        }

        Ok(Self {
            infra,
            auth: auth::AuthService::new(),
            wallet: wallet::WalletService::new(),
            signals: signal_engine::SignalEngine::new(),
            identity: identity::IdentityService::new(),
            arena,
            agent: agent::AgentService::new(),
            settings: settings::SettingsService::new(),
            execution: execution::ExecutionService::new(settings.clone()),
            contracts,
            provider,
            claude,
        })
    }

    pub fn provider_name(&self) -> &'static str {
        self.provider.active_name()
    }
}
