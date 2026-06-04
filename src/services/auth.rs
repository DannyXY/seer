use std::collections::HashMap;
use std::str::FromStr;
use std::sync::RwLock;

use chrono::{Duration, Utc};
use ethers_core::types::{Address, Signature};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::models::auth::{AuthChallenge, AuthSession, AuthVerifyRequest};

pub struct AuthService {
    challenges: RwLock<HashMap<String, AuthChallenge>>,
    sessions: RwLock<HashMap<String, AuthSession>>,
}

impl AuthService {
    pub fn new() -> Self {
        Self {
            challenges: RwLock::new(HashMap::new()),
            sessions: RwLock::new(HashMap::new()),
        }
    }

    pub fn create_challenge(&self, wallet_address: String) -> anyhow::Result<AuthChallenge> {
        let normalized_wallet = normalize_wallet(&wallet_address)?;
        let nonce = Uuid::new_v4().to_string();
        let expires_at = Utc::now() + Duration::minutes(10);
        let message = format!(
            "Sign in to Seer\n\nWallet: {normalized_wallet}\nNonce: {nonce}\nExpires At: {}",
            expires_at.to_rfc3339()
        );
        let challenge = AuthChallenge {
            wallet_address: normalized_wallet.clone(),
            nonce: nonce.clone(),
            message,
            expires_at,
        };

        self.challenges
            .write()
            .expect("auth challenge store poisoned")
            .insert(challenge_key(&normalized_wallet, &nonce), challenge.clone());

        Ok(challenge)
    }

    pub fn verify(&self, request: AuthVerifyRequest) -> anyhow::Result<AuthSession> {
        let normalized_wallet = normalize_wallet(&request.wallet_address)?;
        let key = challenge_key(&normalized_wallet, &request.nonce);
        let challenge = self
            .challenges
            .write()
            .expect("auth challenge store poisoned")
            .remove(&key)
            .ok_or_else(|| anyhow::anyhow!("auth challenge not found or already used"))?;

        if challenge.expires_at < Utc::now() {
            anyhow::bail!("auth challenge expired");
        }
        if challenge.message != request.message {
            anyhow::bail!("auth message does not match issued challenge");
        }

        let recovered = recover_wallet(&request.message, &request.signature)?;
        if recovered != normalized_wallet {
            anyhow::bail!("signature does not match wallet");
        }

        let session = AuthSession {
            wallet_address: normalized_wallet,
            token: session_token(&request.signature, &request.nonce),
            expires_at: Utc::now() + Duration::hours(24),
        };

        self.sessions
            .write()
            .expect("auth session store poisoned")
            .insert(session.token.clone(), session.clone());

        Ok(session)
    }

    pub fn session_for_token(&self, token: &str) -> Option<AuthSession> {
        let session = self
            .sessions
            .read()
            .expect("auth session store poisoned")
            .get(token)
            .cloned()?;

        (session.expires_at >= Utc::now()).then_some(session)
    }

    #[cfg(test)]
    pub fn issue_test_session(&self, wallet_address: &str) -> AuthSession {
        let normalized_wallet = normalize_wallet(wallet_address).expect("valid test wallet");
        let token = session_token(&normalized_wallet, "test");
        let session = AuthSession {
            wallet_address: normalized_wallet,
            token,
            expires_at: Utc::now() + Duration::hours(1),
        };
        self.sessions
            .write()
            .expect("auth session store poisoned")
            .insert(session.token.clone(), session.clone());
        session
    }
}

pub fn normalize_wallet(wallet_address: &str) -> anyhow::Result<String> {
    let address =
        Address::from_str(wallet_address).map_err(|_| anyhow::anyhow!("invalid wallet address"))?;
    Ok(format!("{address:?}").to_lowercase())
}

fn recover_wallet(message: &str, signature: &str) -> anyhow::Result<String> {
    let signature = Signature::from_str(signature)
        .map_err(|_| anyhow::anyhow!("invalid Ethereum signature"))?;
    let recovered = signature
        .recover(message)
        .map_err(|err| anyhow::anyhow!("signature recovery failed: {err}"))?;
    Ok(format!("{recovered:?}").to_lowercase())
}

fn challenge_key(wallet_address: &str, nonce: &str) -> String {
    format!("{}:{}", wallet_address.to_lowercase(), nonce)
}

fn session_token(signature_or_wallet: &str, nonce: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(signature_or_wallet.as_bytes());
    hasher.update(nonce.as_bytes());
    hasher.update(Uuid::new_v4().as_bytes());
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ethers_signers::{LocalWallet, Signer};
    use std::str::FromStr;

    #[test]
    fn creates_challenge_for_normalized_wallet() {
        let service = AuthService::new();
        let challenge = service
            .create_challenge("0x0000000000000000000000000000000000000001".to_string())
            .unwrap();

        assert_eq!(
            challenge.wallet_address,
            "0x0000000000000000000000000000000000000001"
        );
        assert!(challenge.message.contains("Sign in to Seer"));
        assert!(challenge.message.contains(&challenge.nonce));
    }

    #[test]
    fn test_session_can_be_loaded_by_token() {
        let service = AuthService::new();
        let session = service.issue_test_session("0x0000000000000000000000000000000000000001");

        let loaded = service.session_for_token(&session.token).unwrap();
        assert_eq!(loaded.wallet_address, session.wallet_address);
    }

    #[tokio::test]
    async fn verifies_real_wallet_signature_and_issues_session() {
        let wallet = LocalWallet::from_str(
            "0x59c6995e998f97a5a0044966f094538e7b87c7f9d392d7dd05fefc7a8b63a6d8",
        )
        .unwrap();
        let service = AuthService::new();
        let challenge = service
            .create_challenge(format!("{:?}", wallet.address()))
            .unwrap();
        let signature = wallet
            .sign_message(challenge.message.clone())
            .await
            .unwrap();

        let session = service
            .verify(AuthVerifyRequest {
                wallet_address: format!("{:?}", wallet.address()),
                nonce: challenge.nonce,
                message: challenge.message,
                signature: signature.to_string(),
            })
            .unwrap();

        assert_eq!(
            session.wallet_address,
            normalize_wallet(&format!("{:?}", wallet.address())).unwrap()
        );
        assert!(service.session_for_token(&session.token).is_some());
    }
}
