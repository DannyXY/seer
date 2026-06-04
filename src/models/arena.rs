use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComparisonOperator {
    GreaterThanOrEqual,
    LessThanOrEqual,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ArenaPosition {
    BackSeer,
    ChallengeSeer,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PredictionStatus {
    Open,
    Locked,
    Resolved,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArenaPrediction {
    pub id: Uuid,
    pub onchain_prediction_id: Option<u64>,
    pub claim: String,
    pub metric: String,
    pub target_value: f64,
    pub comparison_operator: ComparisonOperator,
    pub expiry_time: DateTime<Utc>,
    pub seer_position: ArenaPosition,
    pub seer_confidence: u8,
    pub reasoning: String,
    pub status: PredictionStatus,
    pub result: Option<String>,
    pub final_value: Option<f64>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArenaEntryRequest {
    pub wallet_address: String,
    pub user_position: ArenaPosition,
    pub points_committed: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ArenaEntryStatus {
    Active,
    Resolved,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArenaEntry {
    pub id: Uuid,
    pub prediction_id: Uuid,
    pub wallet_address: String,
    pub user_position: ArenaPosition,
    pub points_committed: u32,
    pub status: ArenaEntryStatus,
    pub points_delta: Option<i32>,
    pub tx_hash: Option<String>,
    pub created_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaderboardRow {
    pub rank: u32,
    pub wallet_address: String,
    pub total_points: i32,
    pub weekly_gain: i32,
    pub accuracy_rate: Option<f64>,
    pub entries_count: u32,
}
