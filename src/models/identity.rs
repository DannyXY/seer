use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PortfolioArchetype {
    YieldVampire,
    DiamondHand,
    Contrarian,
    Degen,
    Strategist,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioIdentity {
    pub id: Uuid,
    pub wallet_address: String,
    pub archetype: PortfolioArchetype,
    pub percentile: Option<u8>,
    pub stats: Value,
    pub insights: Value,
    pub metadata_uri: Option<String>,
    pub sbt_token_id: Option<u64>,
    pub created_at: DateTime<Utc>,
}
