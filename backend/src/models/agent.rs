use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use super::execution::ExecutionProposal;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IntentStatus {
    Draft,
    Active,
    Paused,
    Completed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IntentExecutionMode {
    Instant,
    Recurring,
    Conditional,
    RecurringConditional,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentTrigger {
    pub mode: IntentExecutionMode,
    pub schedule: Option<String>,
    pub conditions: Vec<ExecutionCondition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionCondition {
    pub metric: String,
    pub subject: String,
    pub operator: String,
    pub value: Value,
    pub provider_hint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedIntent {
    pub action: String,
    pub target_assets: Vec<String>,
    pub target_protocols: Vec<String>,
    pub spend_amount: Option<SpendAmount>,
    pub trigger: IntentTrigger,
    pub constraints: Vec<String>,
    pub requires_user_signature: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpendAmount {
    pub amount: f64,
    pub asset: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentIntent {
    pub id: Uuid,
    pub wallet_address: String,
    pub raw_intent: String,
    pub parsed_intent: ParsedIntent,
    pub status: IntentStatus,
    pub intent_hash: String,
    pub onchain_intent_id: Option<u64>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPolicy {
    pub id: Uuid,
    pub intent_id: Uuid,
    pub wallet_address: String,
    pub smart_account_address: Option<String>,
    pub session_key_address: Option<String>,
    pub allowed_assets: Vec<String>,
    pub allowed_protocols: Vec<String>,
    pub allowed_contracts: Vec<String>,
    pub max_spend_usd: Option<f64>,
    pub max_transaction_count: Option<u32>,
    pub transactions_used: u32,
    pub revoked_at: Option<DateTime<Utc>>,
    pub expires_at: DateTime<Utc>,
    pub status: IntentStatus,
    pub policy_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateIntentRequest {
    pub wallet_address: String,
    pub raw_intent: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluateIntentWithAllowanceRequest {
    pub wallet_address: String,
    pub raw_intent: String,
    #[serde(default)]
    pub token_address: Option<String>,
    #[serde(default)]
    pub owner_address: Option<String>,
    #[serde(default)]
    pub spender_address: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSessionPolicyRequest {
    pub smart_account_address: String,
    pub session_key_address: String,
    pub allowed_assets: Vec<String>,
    pub allowed_protocols: Vec<String>,
    pub allowed_contracts: Vec<String>,
    pub max_spend_usd: Option<f64>,
    pub max_transaction_count: Option<u32>,
    pub expires_in_days: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentExecutionLog {
    pub id: Uuid,
    pub intent_id: Uuid,
    pub policy_id: Option<Uuid>,
    pub wallet_address: String,
    pub action_type: String,
    pub proposal: ExecutionProposal,
    pub execution_status: String,
    pub reasoning_hash: String,
    pub created_at: DateTime<Utc>,
}
