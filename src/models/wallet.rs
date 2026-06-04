use serde::{Deserialize, Serialize};

use super::provider::PortfolioPosition;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletSummary {
    pub address: String,
    pub network: String,
    pub balances: Vec<PortfolioPosition>,
    pub risk_score: u8,
    pub wallet_age_days: i64,
    pub protocols_used: usize,
    pub transaction_count: u64,
}
