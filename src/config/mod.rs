use std::env;

#[derive(Debug, Clone)]
pub struct Settings {
    pub app_env: String,
    pub app_role: AppRole,
    pub port: u16,
    pub version: String,
    pub database_url: Option<String>,
    pub redis_url: Option<String>,
    pub claude_api_key: Option<String>,
    pub claude_model: String,
    pub nansen_api_key: Option<String>,
    pub nansen_base_url: Option<String>,
    pub nansen_cli_path: String,
    pub mantle_rpc_url: Option<String>,
    pub mantle_chain_id: u64,
    pub aa_bundler_url: Option<String>,
    pub backend_signer_private_key: Option<String>,
    pub mantle_usdc_address: Option<String>,
    pub mantle_usdt_address: Option<String>,
    pub mantle_mnt_address: Option<String>,
    pub mantle_meth_address: Option<String>,
    pub approved_strategy_address: Option<String>,
    pub arena_points_address: Option<String>,
    pub prediction_registry_address: Option<String>,
    pub identity_sbt_address: Option<String>,
    pub intent_registry_address: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppRole {
    Api,
    Worker,
}

impl AppRole {
    fn parse(value: &str) -> anyhow::Result<Self> {
        match value.trim().to_lowercase().as_str() {
            "api" => Ok(Self::Api),
            "worker" => Ok(Self::Worker),
            other => anyhow::bail!("APP_ROLE must be 'api' or 'worker', got '{other}'"),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Api => "api",
            Self::Worker => "worker",
        }
    }
}

impl Settings {
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            app_env: env_or("APP_ENV", "development"),
            app_role: AppRole::parse(&env_or("APP_ROLE", "api"))?,
            port: env_or("PORT", "10000").parse()?,
            version: env_or("SEER_VERSION", env!("CARGO_PKG_VERSION")),
            database_url: env_opt("DATABASE_URL"),
            redis_url: env_opt("REDIS_URL"),
            claude_api_key: env_opt("CLAUDE_API_KEY"),
            claude_model: env_or("CLAUDE_MODEL", "claude-3-5-sonnet-latest"),
            nansen_api_key: env_opt("NANSEN_API_KEY"),
            nansen_base_url: env_opt("NANSEN_BASE_URL"),
            nansen_cli_path: env_or("NANSEN_CLI_PATH", "nansen"),
            mantle_rpc_url: env_opt("MANTLE_RPC_URL"),
            mantle_chain_id: env_or("MANTLE_CHAIN_ID", "5003").parse()?,
            aa_bundler_url: env_opt("AA_BUNDLER_URL"),
            backend_signer_private_key: env_opt("BACKEND_SIGNER_PRIVATE_KEY"),
            mantle_usdc_address: env_opt("MANTLE_USDC_ADDRESS"),
            mantle_usdt_address: env_opt("MANTLE_USDT_ADDRESS"),
            mantle_mnt_address: env_opt("MANTLE_MNT_ADDRESS"),
            mantle_meth_address: env_opt("MANTLE_METH_ADDRESS"),
            approved_strategy_address: env_opt("SEER_APPROVED_STRATEGY_ADDRESS"),
            arena_points_address: env_opt("SEER_ARENA_POINTS_ADDRESS"),
            prediction_registry_address: env_opt("SEER_PREDICTION_REGISTRY_ADDRESS"),
            identity_sbt_address: env_opt("SEER_IDENTITY_SBT_ADDRESS"),
            intent_registry_address: env_opt("SEER_INTENT_REGISTRY_ADDRESS"),
        })
    }
}

fn env_or(key: &str, default: &str) -> String {
    env::var(key).unwrap_or_else(|_| default.to_string())
}

fn env_opt(key: &str) -> Option<String> {
    env::var(key).ok().filter(|value| !value.trim().is_empty())
}
