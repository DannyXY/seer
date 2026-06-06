use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletProfile {
    pub address: String,
    pub network: String,
    pub labels: Vec<String>,
    pub portfolio_value_usd: f64,
    pub wallet_age_days: i64,
    pub transaction_count: u64,
    pub protocols_used: Vec<String>,
    pub risk_score: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioPosition {
    pub symbol: String,
    pub amount: String,
    pub usd_value: f64,
    pub protocol: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletTransaction {
    pub hash: String,
    pub timestamp: DateTime<Utc>,
    pub protocol: Option<String>,
    pub asset: Option<String>,
    pub direction: String,
    pub usd_value: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenFlow {
    pub token: String,
    pub protocol: Option<String>,
    pub source_provider: String,
    pub net_flow_usd: f64,
    pub wallet_count: u32,
    pub smart_money_wallet_count: u32,
    pub captured_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolMetrics {
    pub protocol: String,
    pub source_provider: String,
    pub tvl_usd: f64,
    pub tvl_change_24h_pct: f64,
    pub apy: Option<f64>,
    pub risk_score: u8,
    pub captured_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmartMoneyMovement {
    pub wallet: String,
    pub protocol: String,
    pub asset: String,
    pub source_provider: String,
    pub direction: String,
    pub usd_value: f64,
    pub confidence: u8,
    pub captured_at: DateTime<Utc>,
}
