use chrono::Utc;
use serde_json::json;
use uuid::Uuid;

use crate::{
    models::identity::{PortfolioArchetype, PortfolioIdentity},
    services::data_provider::OnchainDataProvider,
};

pub struct IdentityService;

impl IdentityService {
    pub fn new() -> Self {
        Self
    }

    pub async fn generate(
        &self,
        provider: &dyn OnchainDataProvider,
        address: &str,
    ) -> anyhow::Result<PortfolioIdentity> {
        let profile = provider.get_wallet_profile(address).await?;
        let archetype = if profile.risk_score > 75 {
            PortfolioArchetype::Degen
        } else if profile.protocols_used.len() >= 3 {
            PortfolioArchetype::Strategist
        } else {
            PortfolioArchetype::DiamondHand
        };

        Ok(PortfolioIdentity {
            id: Uuid::new_v4(),
            wallet_address: address.to_string(),
            archetype,
            percentile: Some(82),
            stats: json!({
                "portfolio_value_usd": profile.portfolio_value_usd,
                "wallet_age_days": profile.wallet_age_days,
                "risk_score": profile.risk_score,
                "labels": profile.labels,
            }),
            insights: json!({
                "summary": "This wallet behaves like a Mantle-native strategist with meaningful protocol spread.",
                "risk_note": "Identity is descriptive and does not imply future performance.",
            }),
            metadata_uri: None,
            sbt_token_id: None,
            created_at: Utc::now(),
        })
    }
}
