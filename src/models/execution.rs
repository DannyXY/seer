use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::models::agent::ExecutionCondition;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConditionEvaluation {
    pub condition: ExecutionCondition,
    pub observed_value: Option<Value>,
    pub passed: bool,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionProposal {
    pub actionable: bool,
    pub action: String,
    pub wallet_address: String,
    pub chain_id: u64,
    pub network: String,
    pub conditions: Vec<ConditionEvaluation>,
    pub transaction_draft: Option<TransactionDraft>,
    pub required_authorization: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionDraft {
    pub kind: String,
    pub to: Option<String>,
    pub value: String,
    pub data: Option<String>,
    pub chain_id: u64,
    pub human_summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserOperationDraft {
    pub sender: String,
    pub call_data: Option<String>,
    pub target: Option<String>,
    pub value: String,
    pub chain_id: u64,
    pub policy_hash: String,
    pub human_summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegatedExecutionResult {
    pub executable: bool,
    pub execution_status: String,
    pub policy_hash: String,
    pub proposal: ExecutionProposal,
    pub user_operation: Option<UserOperationDraft>,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendRawTransactionRequest {
    pub signed_transaction: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendRawTransactionResponse {
    pub tx_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendUserOperationRequest {
    pub entry_point: String,
    pub user_operation: Map<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendUserOperationResponse {
    pub user_operation_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserOperationReceiptRequest {
    pub user_operation_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Erc20AllowanceRequest {
    pub token_address: String,
    pub owner_address: String,
    pub spender_address: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Erc20AllowanceResponse {
    pub token_address: String,
    pub owner_address: String,
    pub spender_address: String,
    pub allowance: String,
}
