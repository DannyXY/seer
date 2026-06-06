use axum::{extract::State, Json};
use serde_json::{json, Value};

use crate::{
    errors::ApiError,
    models::auth::{AuthChallengeRequest, AuthVerifyRequest},
    AppState,
};

pub async fn challenge(
    State(state): State<AppState>,
    Json(request): Json<AuthChallengeRequest>,
) -> Result<Json<Value>, ApiError> {
    let challenge = state
        .services
        .auth
        .create_challenge(request.wallet_address)
        .map_err(|err| ApiError::BadRequest(err.to_string()))?;
    Ok(Json(json!(challenge)))
}

pub async fn verify(
    State(state): State<AppState>,
    Json(request): Json<AuthVerifyRequest>,
) -> Result<Json<Value>, ApiError> {
    let session = state
        .services
        .auth
        .verify(request)
        .map_err(|err| ApiError::Unauthorized(err.to_string()))?;
    Ok(Json(json!(session)))
}
