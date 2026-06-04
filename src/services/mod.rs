pub mod agent;
pub mod arena;
pub mod auth;
pub mod claude;
pub mod contracts;
pub mod data_provider;
pub mod execution;
pub mod identity;
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
    pub execution: execution::ExecutionService,
    pub contracts: contracts::ContractService,
}

impl AppServices {
    pub async fn new(settings: Settings) -> anyhow::Result<Self> {
        let infra = Infrastructure::from_settings(&settings)?;
        let provider = data_provider::ProviderRegistry::new(settings.clone());
        let claude = claude::ClaudeService::new(settings.clone());
        Ok(Self {
            infra,
            auth: auth::AuthService::new(),
            wallet: wallet::WalletService::new(),
            signals: signal_engine::SignalEngine::new(),
            identity: identity::IdentityService::new(),
            arena: arena::ArenaService::new(),
            agent: agent::AgentService::new(),
            execution: execution::ExecutionService::new(settings.clone()),
            contracts: contracts::ContractService::new(settings),
            provider,
            claude,
        })
    }

    pub fn provider_name(&self) -> &'static str {
        self.provider.active_name()
    }
}
