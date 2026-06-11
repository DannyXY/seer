use serde::{Deserialize, Serialize};

use super::provider::PortfolioPosition;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletSummary {
    pub address: String,
    pub network: String,
    pub balances: Vec<PortfolioPosition>,
    pub mainnet_balances: Vec<PortfolioPosition>,
    pub testnet_balances: Vec<PortfolioPosition>,
    pub seer_token_faucet_calldata: Option<FaucetCalldata>,
    pub risk_score: u8,
    pub wallet_age_days: i64,
    pub protocols_used: usize,
    pub transaction_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaucetCalldata {
    pub label: String,
    pub token_symbol: String,
    pub token_address: String,
    pub amount: String,
    pub to: String,
    pub data: String,
    pub chain_id: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionCalldata {
    pub label: String,
    pub to: String,
    pub data: String,
    pub chain_id: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletApproval {
    pub id: String,
    pub token_symbol: String,
    pub token_address: String,
    pub spender_label: String,
    pub spender_address: String,
    pub allowance: String,
    pub allowance_display: String,
    pub active: bool,
    pub revoke_calldata: Option<TransactionCalldata>,
    pub read_error: Option<String>,
}
