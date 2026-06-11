use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::{
    api::auth_guard::require_wallet,
    db::{load_arena_predictions, load_entries_for_wallet, persist_arena_entry, persist_arena_prediction},
    errors::ApiError,
    models::arena::ArenaEntryRequest,
    AppState,
};

pub async fn predictions(State(state): State<AppState>) -> Json<Value> {
    // Merge from DB on every read so predictions created by the worker process
    // or a previous run are always visible (seeding never overwrites in-memory state).
    if let Ok(db_preds) = load_arena_predictions(state.services.infra.postgres.as_ref()).await {
        if !db_preds.is_empty() {
            state.services.arena.seed_predictions(db_preds);
        }
    }

    let predictions = state
        .services
        .arena
        .predictions()
        .into_iter()
        .map(|prediction| {
            json!({
                "claim": prediction.claim,
                "comparison_operator": prediction.comparison_operator,
                "created_at": prediction.created_at,
                "expiry_time": prediction.expiry_time,
                "final_value": prediction.final_value,
                "id": prediction.id,
                "metric": prediction.metric,
                "onchain_prediction_id": prediction.onchain_prediction_id,
                "pool_points": state.services.arena.prediction_pool(prediction.id),
                "reasoning": prediction.reasoning,
                "result": prediction.result,
                "seer_confidence": prediction.seer_confidence,
                "seer_position": prediction.seer_position,
                "status": prediction.status,
                "target_value": prediction.target_value
            })
        })
        .collect::<Vec<_>>();
    Json(json!({ "predictions": predictions }))
}

pub async fn prediction(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>, ApiError> {
    let prediction = state
        .services
        .arena
        .get_prediction(id)
        .ok_or(ApiError::NotFound)?;
    Ok(Json(json!(prediction)))
}

pub async fn enter(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(entry): Json<ArenaEntryRequest>,
) -> Result<Json<Value>, ApiError> {
    require_wallet(&state, &headers, &entry.wallet_address)?;

    // Sync on-chain points into the in-memory store before the balance check,
    // so a wallet that claimed on-chain is never rejected for insufficient balance.
    if let Ok(onchain_pts) = state.services.contracts.read_available_points(&entry.wallet_address).await {
        state.services.arena.sync_points(&entry.wallet_address, onchain_pts);
    }

    let entry = state
        .services
        .arena
        .enter_prediction(id, entry)
        .map_err(|err| ApiError::BadRequest(err.to_string()))?;

    // Persist entry and updated prediction state to DB.
    let _ = persist_arena_entry(state.services.infra.postgres.as_ref(), &entry).await;
    if let Some(pred) = state.services.arena.get_prediction(id) {
        let _ = persist_arena_prediction(state.services.infra.postgres.as_ref(), &pred).await;
    }

    let user_points = state.services.arena.user_points(&entry.wallet_address);

    // Build calldata for the user to sign the on-chain enterPrediction tx
    let onchain = state.services.arena.get_prediction(id)
        .and_then(|p| p.onchain_prediction_id)
        .and_then(|onchain_id| {
            let pos = match entry.user_position {
                crate::models::arena::ArenaPosition::BackSeer => 0u8,
                crate::models::arena::ArenaPosition::ChallengeSeer => 1u8,
            };
            state.services.contracts.enter_prediction_calldata(onchain_id, pos, entry.points_committed as u64)
                .map(|(to, data)| json!({ "to": to, "data": data, "chain_id": state.services.contracts.chain_id }))
        });

    let starter_calldata = state.services.contracts.claim_starter_points_calldata()
        .map(|(to, data)| json!({ "to": to, "data": data, "chain_id": state.services.contracts.chain_id }));

    Ok(Json(json!({
        "prediction_id": id,
        "entry": entry,
        "user_points": user_points,
        "status": "active",
        "contract_configured": state.services.contracts.is_configured(),
        "entry_calldata": onchain,
        "claim_starter_calldata": starter_calldata,
    })))
}

pub async fn entries(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(address): Path<String>,
) -> Result<Json<Value>, ApiError> {
    require_wallet(&state, &headers, &address)?;

    // 1. Restore entries from DB on cache miss.
    if state.services.arena.entries_for_wallet(&address).is_empty() {
        if let Ok(db_entries) = load_entries_for_wallet(
            state.services.infra.postgres.as_ref(), &address
        ).await {
            state.services.arena.seed_entries(db_entries);
        }
    }

    // 2. Seed any entries found on-chain that are still missing locally.
    //    Queries PredictionEntered events filtered by this wallet address.
    if state.services.contracts.rpc_configured() {
        if let Ok(onchain_entries) = state
            .services
            .contracts
            .get_entries_for_user_onchain(&address)
            .await
        {
            for (onchain_pred_id, position_u8, points) in onchain_entries {
                // Find the local prediction by its on-chain ID.
                let prediction = state
                    .services
                    .arena
                    .get_prediction_by_onchain_id(onchain_pred_id);

                let prediction_uuid = match prediction {
                    Some(p) => p.id,
                    // Prediction not yet seeded locally — skip this entry for now;
                    // it will appear once the predictions endpoint is called.
                    None => continue,
                };

                // Skip if we already have an entry for this prediction.
                let already_has = state
                    .services
                    .arena
                    .entries_for_wallet(&address)
                    .iter()
                    .any(|e| e.prediction_id == prediction_uuid);
                if already_has {
                    continue;
                }

                let user_position = if position_u8 == 0 {
                    crate::models::arena::ArenaPosition::BackSeer
                } else {
                    crate::models::arena::ArenaPosition::ChallengeSeer
                };

                let entry = crate::models::arena::ArenaEntry {
                    id: uuid::Uuid::new_v4(),
                    prediction_id: prediction_uuid,
                    wallet_address: address.clone(),
                    user_position,
                    points_committed: points as u32,
                    status: crate::models::arena::ArenaEntryStatus::Active,
                    points_delta: None,
                    tx_hash: None,
                    created_at: chrono::Utc::now(),
                    resolved_at: None,
                };

                state.services.arena.seed_entries(vec![entry]);
            }
        }
    }

    // 3. Sync on-chain points so balance is accurate even after restart.
    if let Ok(onchain_pts) = state.services.contracts.read_available_points(&address).await {
        state.services.arena.sync_points(&address, onchain_pts);
    }

    Ok(Json(json!({
        "wallet_address": address,
        "entries": state.services.arena.entries_for_wallet(&address),
        "user_points": state.services.arena.user_points(&address)
    })))
}

pub async fn leaderboard(State(state): State<AppState>) -> Json<Value> {
    Json(json!({
        "leaderboard": state.services.arena.leaderboard()
    }))
}

pub async fn seer_record(State(state): State<AppState>) -> Json<Value> {
    let (total, correct) = state.services.arena.seer_record();
    let accuracy_rate = if total > 0 {
        correct as f64 / total as f64
    } else {
        0.0
    };
    Json(json!({
        "accuracy_rate": accuracy_rate,
        "resolved_predictions": total,
        "correct_predictions": correct,
    }))
}

pub async fn on_chain_points(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(address): Path<String>,
) -> Result<Json<Value>, ApiError> {
    require_wallet(&state, &headers, &address)?;
    let (available, claimed) = tokio::join!(
        state.services.contracts.read_available_points(&address),
        state.services.contracts.has_claimed_starter_points(&address),
    );
    let claim_calldata = state.services.contracts.claim_starter_points_calldata()
        .map(|(to, data)| json!({ "to": to, "data": data, "chain_id": state.services.contracts.chain_id }));
    Ok(Json(json!({
        "wallet_address": address,
        "available_points": available.unwrap_or(0),
        "has_claimed_starter_points": claimed.unwrap_or(false),
        "claim_starter_calldata": claim_calldata,
        "contract_configured": state.services.contracts.rpc_configured(),
    })))
}

pub async fn resolve_due(State(state): State<AppState>) -> Json<Value> {
    let summary = crate::jobs::arena::resolve_due_predictions(&state).await;
    let mut body = summary.to_json();
    body["contract_configured"] = json!(state.services.contracts.is_configured());
    Json(body)
}
