use std::env;

#[derive(Debug, Clone)]
pub struct Settings {
    pub app_env: String,
    pub app_role: AppRole,
    pub port: u16,
    pub version: String,
    pub database_url: Option<String>,
    pub run_migrations: bool,
    pub run_internal_jobs: bool,
    pub redis_url: Option<String>,
    pub claude_api_key: Option<String>,
    pub claude_model: String,
    pub nansen_api_key: Option<String>,
    pub nansen_base_url: Option<String>,
    pub nansen_cli_path: String,
    pub nansen_smart_money_chains: Vec<String>,
    pub defillama_enabled: bool,
    pub defillama_base_url: String,
    pub defillama_yields_base_url: String,
    pub mantle_rpc_url: Option<String>,
    /// Read-only RPC for user portfolio data (token balances, native balance,
    /// wallet activity). Defaults to Mantle mainnet, where real holdings live,
    /// while `mantle_rpc_url` stays on the chain where Seer's contracts are
    /// deployed (Sepolia testnet).
    pub mantle_data_rpc_url: Option<String>,
    pub mantle_chain_id: u64,
    pub aa_provider_stack: String,
    pub aa_bundler_url: Option<String>,
    pub aa_entry_point_address: Option<String>,
    pub aa_paymaster_url: Option<String>,
    pub backend_signer_private_key: Option<String>,
    pub mantle_usdc_address: Option<String>,
    pub mantle_usdt_address: Option<String>,
    pub mantle_mnt_address: Option<String>,
    pub mantle_meth_address: Option<String>,
    pub mantle_usdy_address: Option<String>,
    pub mantle_wmnt_address: Option<String>,
    pub mantle_weth_address: Option<String>,
    pub mantle_cmeth_address: Option<String>,
    /// Execution-chain token addresses ("SYMBOL:0xADDR,SYMBOL:0xADDR").
    /// When set, transaction drafts use these instead of MANTLE_*_ADDRESS,
    /// which points at mainnet for portfolio reads.
    pub exec_token_addresses: Vec<(String, String)>,
    pub approved_strategy_address: Option<String>,
    pub approved_strategy_spender_address: Option<String>,
    pub strategy_deposit_function: String,
    pub merchant_moe_strategy_address: Option<String>,
    pub merchant_moe_spender_address: Option<String>,
    pub merchant_moe_deposit_function: Option<String>,
    pub agni_strategy_address: Option<String>,
    pub agni_spender_address: Option<String>,
    pub agni_deposit_function: Option<String>,
    pub lendle_strategy_address: Option<String>,
    pub lendle_spender_address: Option<String>,
    pub lendle_deposit_function: Option<String>,
    pub meth_strategy_address: Option<String>,
    pub meth_spender_address: Option<String>,
    pub meth_deposit_function: Option<String>,
    pub ondo_usdy_strategy_address: Option<String>,
    pub ondo_usdy_spender_address: Option<String>,
    pub ondo_usdy_deposit_function: Option<String>,
    pub arena_points_address: Option<String>,
    pub prediction_registry_address: Option<String>,
    pub identity_sbt_address: Option<String>,
    pub intent_registry_address: Option<String>,
    pub telegram_bot_token: Option<String>,
    pub telegram_chat_id: Option<String>,
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
            run_internal_jobs: env_bool("RUN_INTERNAL_JOBS", true),
            redis_url: env_opt("REDIS_URL"),
            claude_api_key: env_opt("CLAUDE_API_KEY"),
            claude_model: env_or("CLAUDE_MODEL", "claude-sonnet-4-20250514"),
            nansen_api_key: env_opt("NANSEN_API_KEY"),
            nansen_base_url: env_opt("NANSEN_BASE_URL"),
            nansen_cli_path: env_or("NANSEN_CLI_PATH", "nansen"),
            nansen_smart_money_chains: env_csv(
                "NANSEN_SMART_MONEY_CHAINS",
                &["ethereum", "solana", "base"],
            ),
            defillama_enabled: env_bool("DEFILLAMA_ENABLED", true),
            defillama_base_url: env_or("DEFILLAMA_BASE_URL", "https://api.llama.fi"),
            defillama_yields_base_url: env_or(
                "DEFILLAMA_YIELDS_BASE_URL",
                "https://yields.llama.fi",
            ),
            mantle_rpc_url: env_opt("MANTLE_RPC_URL"),
            mantle_data_rpc_url: Some(env_or("MANTLE_DATA_RPC_URL", "https://rpc.mantle.xyz")),
            mantle_chain_id: env_or("MANTLE_CHAIN_ID", "5003").parse()?,
            aa_provider_stack: env_or("AA_PROVIDER_STACK", "safe-4337-relay-kit"),
            aa_bundler_url: env_opt("AA_BUNDLER_URL"),
            aa_entry_point_address: env_opt("AA_ENTRY_POINT_ADDRESS"),
            aa_paymaster_url: env_opt("AA_PAYMASTER_URL"),
            backend_signer_private_key: env_opt("BACKEND_SIGNER_PRIVATE_KEY"),
            mantle_usdc_address: env_opt("MANTLE_USDC_ADDRESS"),
            mantle_usdt_address: env_opt("MANTLE_USDT_ADDRESS"),
            mantle_mnt_address: env_opt("MANTLE_MNT_ADDRESS"),
            mantle_meth_address: env_opt("MANTLE_METH_ADDRESS"),
            mantle_usdy_address: env_opt("MANTLE_USDY_ADDRESS"),
            mantle_wmnt_address: env_opt("MANTLE_WMNT_ADDRESS"),
            mantle_weth_address: env_opt("MANTLE_WETH_ADDRESS"),
            mantle_cmeth_address: env_opt("MANTLE_CMETH_ADDRESS"),
            exec_token_addresses: env_symbol_address_pairs("SEER_EXEC_TOKEN_ADDRESSES"),
            approved_strategy_address: env_opt("SEER_APPROVED_STRATEGY_ADDRESS"),
            approved_strategy_spender_address: env_opt("SEER_APPROVED_STRATEGY_SPENDER_ADDRESS"),
            strategy_deposit_function: env_or(
                "SEER_STRATEGY_DEPOSIT_FUNCTION",
                "deposit(address,uint256)",
            ),
            merchant_moe_strategy_address: env_opt("SEER_MERCHANT_MOE_STRATEGY_ADDRESS"),
            merchant_moe_spender_address: env_opt("SEER_MERCHANT_MOE_SPENDER_ADDRESS"),
            merchant_moe_deposit_function: env_opt("SEER_MERCHANT_MOE_DEPOSIT_FUNCTION"),
            agni_strategy_address: env_opt("SEER_AGNI_STRATEGY_ADDRESS"),
            agni_spender_address: env_opt("SEER_AGNI_SPENDER_ADDRESS"),
            agni_deposit_function: env_opt("SEER_AGNI_DEPOSIT_FUNCTION"),
            lendle_strategy_address: env_opt("SEER_LENDLE_STRATEGY_ADDRESS"),
            lendle_spender_address: env_opt("SEER_LENDLE_SPENDER_ADDRESS"),
            lendle_deposit_function: env_opt("SEER_LENDLE_DEPOSIT_FUNCTION"),
            meth_strategy_address: env_opt("SEER_METH_STRATEGY_ADDRESS"),
            meth_spender_address: env_opt("SEER_METH_SPENDER_ADDRESS"),
            meth_deposit_function: env_opt("SEER_METH_DEPOSIT_FUNCTION"),
            ondo_usdy_strategy_address: env_opt("SEER_ONDO_USDY_STRATEGY_ADDRESS"),
            ondo_usdy_spender_address: env_opt("SEER_ONDO_USDY_SPENDER_ADDRESS"),
            ondo_usdy_deposit_function: env_opt("SEER_ONDO_USDY_DEPOSIT_FUNCTION"),
            arena_points_address: env_opt("SEER_ARENA_POINTS_ADDRESS"),
            prediction_registry_address: env_opt("SEER_PREDICTION_REGISTRY_ADDRESS"),
            identity_sbt_address: env_opt("SEER_IDENTITY_SBT_ADDRESS"),
            intent_registry_address: env_opt("SEER_INTENT_REGISTRY_ADDRESS"),
            telegram_bot_token: env_opt("TELEGRAM_BOT_TOKEN"),
            telegram_chat_id: env_opt("TELEGRAM_CHAT_ID"),
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

/// Parse "SYMBOL:0xADDR,SYMBOL:0xADDR" pairs; malformed entries are skipped.
fn env_symbol_address_pairs(key: &str) -> Vec<(String, String)> {
    env::var(key)
        .ok()
        .map(|value| {
            value
                .split(',')
                .filter_map(|pair| {
                    let (symbol, address) = pair.split_once(':')?;
                    let symbol = canonical_token_symbol(symbol.trim());
                    let address = address.trim();
                    (!symbol.is_empty() && address.starts_with("0x"))
                        .then(|| (symbol, address.to_string()))
                })
                .collect()
        })
        .unwrap_or_default()
}

fn canonical_token_symbol(symbol: &str) -> String {
    match symbol.trim().to_uppercase().as_str() {
        "METH" => "mETH".to_string(),
        "CMETH" => "cmETH".to_string(),
        "USDC" => "USDC".to_string(),
        "USDT" => "USDT".to_string(),
        "MNT" => "MNT".to_string(),
        "USDY" => "USDY".to_string(),
        "WMNT" => "WMNT".to_string(),
        "WETH" => "WETH".to_string(),
        other => other.to_string(),
    }
}

fn env_csv(key: &str, default: &[&str]) -> Vec<String> {
    env::var(key)
        .ok()
        .map(|value| {
            value
                .split(',')
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .filter(|values| !values.is_empty())
        .unwrap_or_else(|| default.iter().map(|value| value.to_string()).collect())
}
