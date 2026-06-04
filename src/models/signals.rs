use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SignalCategory {
    Alpha,
    Anomaly,
    Risk,
    Opportunity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Signal {
    pub id: Uuid,
    pub category: SignalCategory,
    pub headline: String,
    pub explanation: String,
    pub confidence_score: u8,
    pub related_wallet: Option<String>,
    pub related_protocol: Option<String>,
    pub related_asset: Option<String>,
    pub source_provider: String,
    pub source_data: Value,
    pub created_at: DateTime<Utc>,
}
