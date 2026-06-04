use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthChallengeRequest {
    pub wallet_address: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthChallenge {
    pub wallet_address: String,
    pub nonce: String,
    pub message: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthVerifyRequest {
    pub wallet_address: String,
    pub nonce: String,
    pub message: String,
    pub signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthSession {
    pub wallet_address: String,
    pub token: String,
    pub expires_at: DateTime<Utc>,
}
