use std::env;

#[derive(Debug, Clone)]
pub struct Settings {
    pub app_env: String,
    pub app_role: AppRole,
    pub port: u16,
    pub version: String,
    pub database_url: Option<String>,
    pub run_migrations: bool,
    pub redis_url: Option<String>,
    pub claude_api_key: Option<String>,
    pub claude_model: String,
    pub nansen_api_key: Option<String>,
    pub nansen_base_url: Option<String>,
    pub nansen_cli_path: String,
    pub defillama_enabled: bool,
    pub defillama_base_url: String,
    pub defillama_yields_base_url: String,
    pub mantle_rpc_url: Option<String>,
    pub mantle_chain_id: u64,
    pub aa_bundler_url: Option<String>,
    pub backend_signer_private_key: Option<String>,
    pub mantle_usdc_address: Option<String>,
    pub mantle_usdt_address: Option<String>,
    pub mantle_mnt_address: Option<String>,
    pub mantle_meth_address: Option<String>,
    pub approved_strategy_address: Option<String>,
    pub approved_strategy_spender_address: Option<String>,
    pub strategy_deposit_function: String,
    pub merchant_moe_strategy_address: Option<String>,
    pub merchant_moe_spender_address: Option<String>,
    pub merchant_moe_deposit_function: Option<String>,
    pub lendle_strategy_address: Option<String>,
    pub lendle_spender_address: Option<String>,
    pub lendle_deposit_function: Option<String>,
    pub agni_strategy_address: Option<String>,
    pub agni_spender_address: Option<String>,
    pub agni_deposit_function: Option<String>,
    pub meth_strategy_address: Option<String>,
    pub meth_spender_address: Option<String>,
    pub meth_deposit_function: Option<String>,
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
            run_migrations: env_bool("RUN_MIGRATIONS", false),
            redis_url: env_opt("REDIS_URL"),
            claude_api_key: env_opt("CLAUDE_API_KEY"),
            claude_model: env_or("CLAUDE_MODEL", "claude-sonnet-4-20250514"),
            nansen_api_key: env_opt("NANSEN_API_KEY"),
            nansen_base_url: env_opt("NANSEN_BASE_URL"),
            nansen_cli_path: env_or("NANSEN_CLI_PATH", "nansen"),
            defillama_enabled: env_bool("DEFILLAMA_ENABLED", true),
            defillama_base_url: env_or("DEFILLAMA_BASE_URL", "https://api.llama.fi"),
            defillama_yields_base_url: env_or(
                "DEFILLAMA_YIELDS_BASE_URL",
                "https://yields.llama.fi",
            ),
            mantle_rpc_url: env_opt("MANTLE_RPC_URL"),
            mantle_chain_id: env_or("MANTLE_CHAIN_ID", "5003").parse()?,
            aa_bundler_url: env_opt("AA_BUNDLER_URL"),
            backend_signer_private_key: env_opt("BACKEND_SIGNER_PRIVATE_KEY"),
            mantle_usdc_address: env_opt("MANTLE_USDC_ADDRESS"),
            mantle_usdt_address: env_opt("MANTLE_USDT_ADDRESS"),
            mantle_mnt_address: env_opt("MANTLE_MNT_ADDRESS"),
            mantle_meth_address: env_opt("MANTLE_METH_ADDRESS"),
            approved_strategy_address: env_opt("SEER_APPROVED_STRATEGY_ADDRESS"),
            approved_strategy_spender_address: env_opt("SEER_APPROVED_STRATEGY_SPENDER_ADDRESS"),
            strategy_deposit_function: env_or(
                "SEER_STRATEGY_DEPOSIT_FUNCTION",
                "deposit(address,uint256)",
            ),
            merchant_moe_strategy_address: env_opt("SEER_MERCHANT_MOE_STRATEGY_ADDRESS"),
            merchant_moe_spender_address: env_opt("SEER_MERCHANT_MOE_SPENDER_ADDRESS"),
            merchant_moe_deposit_function: env_opt("SEER_MERCHANT_MOE_DEPOSIT_FUNCTION"),
            lendle_strategy_address: env_opt("SEER_LENDLE_STRATEGY_ADDRESS"),
            lendle_spender_address: env_opt("SEER_LENDLE_SPENDER_ADDRESS"),
            lendle_deposit_function: env_opt("SEER_LENDLE_DEPOSIT_FUNCTION"),
            agni_strategy_address: env_opt("SEER_AGNI_STRATEGY_ADDRESS"),
            agni_spender_address: env_opt("SEER_AGNI_SPENDER_ADDRESS"),
            agni_deposit_function: env_opt("SEER_AGNI_DEPOSIT_FUNCTION"),
            meth_strategy_address: env_opt("SEER_METH_STRATEGY_ADDRESS"),
            meth_spender_address: env_opt("SEER_METH_SPENDER_ADDRESS"),
            meth_deposit_function: env_opt("SEER_METH_DEPOSIT_FUNCTION"),
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

fn env_bool(key: &str, default: bool) -> bool {
    env::var(key)
        .ok()
        .map(|value| {
            matches!(
                value.trim().to_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(default)
}
