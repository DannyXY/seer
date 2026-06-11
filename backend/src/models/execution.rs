use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::models::agent::ExecutionCondition;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProtocolOperation {
    AgniSwap {
        token_in: String,
        token_out: String,
        amount_in: String,
        fee_tier: u32,
        amount_out_minimum: String,
    },
    AgniAddLiquidity {
        token0: String,
        token1: String,
        fee: u32,
        tick_lower: i32,
        tick_upper: i32,
        amount0_desired: String,
        amount1_desired: String,
        amount0_min: String,
        amount1_min: String,
    },
    AgniRemoveLiquidity {
        token_id: u64,
        liquidity: String,
        amount0_min: String,
        amount1_min: String,
    },
    AgniCollectFees {
        token_id: u64,
    },
    MerchantMoeSwap {
        token_path: Vec<String>,
        amount_in: String,
        amount_out_minimum: String,
        bin_steps: Vec<u32>,
    },
    MerchantMoeAddLiquidity {
        token_x: String,
        token_y: String,
        bin_step: u32,
        amount_x: String,
        amount_y: String,
        amount_x_min: String,
        amount_y_min: String,
        active_id_desired: u32,
        id_slippage: u32,
    },
    MerchantMoeRemoveLiquidity {
        token_x: String,
        token_y: String,
        bin_step: u32,
        bin_ids: Vec<u64>,
    },
    MethStake {
        amount_eth: String,
    },
    MethUnstake {
        amount_meth: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConditionEvaluation {
    pub condition: ExecutionCondition,
    pub observed_value: Option<Value>,
    pub source_provider: Option<String>,
    pub source_captured_at: Option<DateTime<Utc>>,
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
    pub allowance_check: Option<Erc20AllowanceRequest>,
    pub transaction_draft: Option<TransactionDraft>,
    pub required_authorization: String,
    pub protocol_operation: Option<ProtocolOperation>,
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
pub struct ProtocolExecutionReadiness {
    pub protocol: String,
    pub strategy_address: Option<String>,
    pub approval_spender_address: Option<String>,
    pub deposit_function: Option<String>,
    pub adapter_kind: String,
    /// Whether the configured deposit function is one the transaction builder
    /// can actually encode. A protocol configured with an unsupported
    /// signature (e.g. a swap) is "configured", not runnable.
    pub signature_supported: bool,
    pub ready_for_strategy_draft: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionReadinessResponse {
    pub chain_id: u64,
    pub configured_token_symbols: Vec<String>,
    pub generic_strategy_address: Option<String>,
    pub generic_approval_spender_address: Option<String>,
    pub generic_deposit_function: String,
    pub protocols: Vec<ProtocolExecutionReadiness>,
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
pub struct SimulateTransactionRequest {
    pub from: Option<String>,
    pub to: String,
    pub value: Option<String>,
    pub data: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulateTransactionResponse {
    pub success: bool,
    pub block_tag: String,
    pub return_data: Option<String>,
    pub error: Option<String>,
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
