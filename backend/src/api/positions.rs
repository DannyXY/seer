use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use serde_json::json;

use crate::{
    api::auth_guard::require_wallet,
    db::persist_lp_position,
    errors::ApiError,
    models::lp_position::{AgniPosition, LpPosition, MerchantMoePosition},
    AppState,
};

#[derive(Debug, serde::Deserialize)]
pub struct CreateAgniPositionRequest {
    pub wallet_address: String,
    pub token_id: u64,
    pub token0: String,
    pub token1: String,
    pub fee: u32,
    pub tick_lower: i32,
    pub tick_upper: i32,
    pub liquidity: String,
    pub amount0_added: String,
    pub amount1_added: String,
    pub tx_hash: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
pub struct CreateMerchantMoePositionRequest {
    pub wallet_address: String,
    pub lb_pair: String,
    pub token_x: String,
    pub token_y: String,
    pub bin_step: u32,
    pub bin_ids: Vec<u64>,
    pub liquidity_minted: Vec<String>,
    pub amount_x_added: String,
    pub amount_y_added: String,
    pub tx_hash: Option<String>,
}

pub async fn create_agni_position(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<CreateAgniPositionRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_wallet(&state, &headers, &req.wallet_address)?;

    let agni_pos = AgniPosition {
        token_id: req.token_id,
        token0: req.token0,
        token1: req.token1,
        fee: req.fee,
        tick_lower: req.tick_lower,
        tick_upper: req.tick_upper,
        liquidity: req.liquidity,
    };

    let position = LpPosition::new_agni(
        req.wallet_address.clone(),
        agni_pos,
        req.amount0_added,
        req.amount1_added,
    );

    let position_id = position.id;
    let mut position = position;
    position.tx_hash = req.tx_hash;

    persist_lp_position(state.services.infra.postgres.as_ref(), &position)
        .await
        .map_err(|err| ApiError::Service(err.to_string()))?;

    Ok(Json(json!({
        "position_id": position_id,
        "protocol": "Agni Finance",
        "token_id": req.token_id,
        "status": "recorded"
    })))
}

pub async fn create_merchant_moe_position(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<CreateMerchantMoePositionRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_wallet(&state, &headers, &req.wallet_address)?;

    let moe_pos = MerchantMoePosition {
        lb_pair: req.lb_pair,
        token_x: req.token_x,
        token_y: req.token_y,
        bin_step: req.bin_step,
        bin_ids: req.bin_ids,
        liquidity_minted: req.liquidity_minted,
    };

    let position = LpPosition::new_merchant_moe(
        req.wallet_address.clone(),
        moe_pos,
        req.amount_x_added,
        req.amount_y_added,
    );

    let position_id = position.id;
    let mut position = position;
    position.tx_hash = req.tx_hash;

    persist_lp_position(state.services.infra.postgres.as_ref(), &position)
        .await
        .map_err(|err| ApiError::Service(err.to_string()))?;

    Ok(Json(json!({
        "position_id": position_id,
        "protocol": "Merchant Moe",
        "bin_ids": position.moe_position.map(|p| p.bin_ids),
        "status": "recorded"
    })))
}

pub async fn list_user_positions(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(wallet_address): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_wallet(&state, &headers, &wallet_address)?;

    let positions =
        crate::db::get_lp_positions(state.services.infra.postgres.as_ref(), &wallet_address)
            .await
            .map_err(|err| ApiError::Service(err.to_string()))?;

    Ok(Json(json!({
        "wallet_address": wallet_address,
        "positions": positions,
        "count": positions.len()
    })))
}
