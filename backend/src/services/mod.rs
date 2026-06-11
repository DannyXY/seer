pub mod abi_encoder;
pub mod agent;
pub mod arena;
pub mod auth;
pub mod claude;
pub mod contracts;
pub mod data_provider;
pub mod execution;
pub mod identity;
pub mod notifier;
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
    pub notifier: notifier::NotifierService,
}

impl AppServices {
    pub async fn new(settings: Settings) -> anyhow::Result<Self> {
        let infra = Infrastructure::from_settings(&settings)?;
        infra.run_migrations_if_enabled().await?;
        let provider = data_provider::ProviderRegistry::new(settings.clone());
        let claude = claude::ClaudeService::new(settings.clone());
        let arena = arena::ArenaService::new();
        let contracts = contracts::ContractService::new(settings.clone());
        let notifier = notifier::NotifierService::new(
            settings.telegram_bot_token.clone(),
            settings.telegram_chat_id.clone(),
        );

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
            notifier,
            provider,
            claude,
        })
    }

    pub fn provider_name(&self) -> &'static str {
        self.provider.active_name()
    }
}
