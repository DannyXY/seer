use axum::http::HeaderMap;

use crate::{errors::ApiError, services::auth::normalize_wallet, AppState};

pub fn require_wallet(
    state: &AppState,
    headers: &HeaderMap,
    expected_wallet: &str,
) -> Result<String, ApiError> {
    let expected_wallet =
        normalize_wallet(expected_wallet).map_err(|err| ApiError::BadRequest(err.to_string()))?;
    let token = bearer_token(headers)?;
    let session = state
        .services
        .auth
        .session_for_token(token)
        .ok_or_else(|| {
            ApiError::Unauthorized("missing, invalid, or expired session".to_string())
        })?;

    if session.wallet_address != expected_wallet {
        return Err(ApiError::Unauthorized(
            "authenticated wallet does not match request wallet".to_string(),
        ));
    }

    Ok(session.wallet_address)
}

fn bearer_token(headers: &HeaderMap) -> Result<&str, ApiError> {
    let value = headers
        .get("authorization")
        .ok_or_else(|| ApiError::Unauthorized("missing Authorization header".to_string()))?
        .to_str()
        .map_err(|_| ApiError::Unauthorized("invalid Authorization header".to_string()))?;

    value
        .strip_prefix("Bearer ")
        .ok_or_else(|| ApiError::Unauthorized("Authorization must use Bearer token".to_string()))
}
