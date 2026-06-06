use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProtocolType {
    AgniFinance,
    MerchantMoe,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgniPosition {
    pub token_id: u64,
    pub token0: String,
    pub token1: String,
    pub fee: u32,
    pub tick_lower: i32,
    pub tick_upper: i32,
    pub liquidity: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerchantMoePosition {
    pub lb_pair: String,
    pub token_x: String,
    pub token_y: String,
    pub bin_step: u32,
    pub bin_ids: Vec<u64>,
    pub liquidity_minted: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LpPosition {
    pub id: Uuid,
    pub wallet_address: String,
    pub protocol: ProtocolType,
    pub agni_position: Option<AgniPosition>,
    pub moe_position: Option<MerchantMoePosition>,
    pub amount_x_added: String,
    pub amount_y_added: String,
    pub intent_hash: Option<String>,
    pub tx_hash: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl LpPosition {
    pub fn new_agni(
        wallet_address: String,
        agni_position: AgniPosition,
        amount_x_added: String,
        amount_y_added: String,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            wallet_address,
            protocol: ProtocolType::AgniFinance,
            agni_position: Some(agni_position),
            moe_position: None,
            amount_x_added,
            amount_y_added,
            intent_hash: None,
            tx_hash: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    pub fn new_merchant_moe(
        wallet_address: String,
        moe_position: MerchantMoePosition,
        amount_x_added: String,
        amount_y_added: String,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            wallet_address,
            protocol: ProtocolType::MerchantMoe,
            agni_position: None,
            moe_position: Some(moe_position),
            amount_x_added,
            amount_y_added,
            intent_hash: None,
            tx_hash: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}
