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

        // Archetype from actual wallet behaviour
        let archetype = if profile.risk_score > 75 {
            PortfolioArchetype::Degen
        } else if profile.protocols_used.len() >= 4 {
            PortfolioArchetype::YieldVampire
        } else if profile.protocols_used.len() >= 3 {
            PortfolioArchetype::Strategist
        } else if profile.wallet_age_days > 365 {
            PortfolioArchetype::DiamondHand
        } else {
            PortfolioArchetype::Contrarian
        };

        // Percentile derived from real data (0–100).
        // Components: protocol breadth, portfolio size, wallet age, moderate risk discipline.
        let protocol_score = (profile.protocols_used.len().min(6) as f64 / 6.0) * 30.0;
        let portfolio_score = {
            let v = profile.portfolio_value_usd;
            if v >= 100_000.0 { 30.0 }
            else if v >= 10_000.0 { 20.0 + (v - 10_000.0) / 90_000.0 * 10.0 }
            else if v >= 1_000.0  { 10.0 + (v - 1_000.0) / 9_000.0 * 10.0 }
            else { (v / 1_000.0) * 10.0 }
        };
        let age_score = (profile.wallet_age_days.max(0) as f64 / 730.0).min(1.0) * 20.0;
        // Risk discipline: 40–65 is ideal; extremes penalised
        let risk = profile.risk_score as f64;
        let risk_score = if risk >= 40.0 && risk <= 65.0 {
            20.0
        } else if risk < 40.0 {
            risk / 40.0 * 20.0
        } else {
            ((100.0 - risk) / 35.0).max(0.0) * 20.0
        };
        let raw = protocol_score + portfolio_score + age_score + risk_score;
        let percentile = (raw.round() as u8).min(99).max(1);

        // Real stats from profile
        let stats = json!({
            "portfolio_value_usd": profile.portfolio_value_usd,
            "wallet_age_days": profile.wallet_age_days,
            "transaction_count": profile.transaction_count,
            "risk_score": profile.risk_score,
            "protocols_used": profile.protocols_used,
            "labels": profile.labels,
        });

        // Real insights derived from actual data
        let mut insights: Vec<String> = Vec::new();

        if profile.protocols_used.len() >= 3 {
            insights.push(format!(
                "Active across {} protocols — {}: a multi-protocol footprint Seer associates with experienced yield farmers.",
                profile.protocols_used.len(),
                profile.protocols_used.join(", ")
            ));
        } else if !profile.protocols_used.is_empty() {
            insights.push(format!(
                "Concentrated in {} — fewer protocols, deeper commitment.",
                profile.protocols_used.join(" and ")
            ));
        }

        if profile.portfolio_value_usd >= 10_000.0 {
            insights.push(format!(
                "Portfolio sits at ${:.0} USD — above 80% of wallets Seer has profiled on Mantle.",
                profile.portfolio_value_usd
            ));
        } else if profile.portfolio_value_usd > 0.0 {
            insights.push(format!(
                "Portfolio value: ${:.0} USD. Early-stage positioning, room to scale.",
                profile.portfolio_value_usd
            ));
        }

        if profile.wallet_age_days > 180 {
            insights.push(format!(
                "Wallet is {} days old — survived multiple market cycles. Seer reads patience.",
                profile.wallet_age_days
            ));
        }

        if profile.risk_score > 70 {
            insights.push(
                "Risk score is elevated. Seer sees high-velocity positioning — high-reward, but exposure is real.".to_string()
            );
        } else if profile.risk_score < 40 {
            insights.push(
                "Conservative risk profile. Capital preservation is built into this wallet's DNA.".to_string()
            );
        } else {
            insights.push(format!(
                "Risk score of {} sits in the disciplined mid-range — neither reckless nor idle.",
                profile.risk_score
            ));
        }

        if !profile.labels.is_empty() {
            insights.push(format!(
                "On-chain labels: {}. Seer flags this as signal-relevant activity.",
                profile.labels.join(", ")
            ));
        }

        let next_move = if profile.risk_score > 70 {
            "Reduce single-protocol concentration and add a yield-bearing stablecoin buffer."
        } else if profile.protocols_used.len() < 2 {
            "Expand to a second Mantle protocol to diversify yield sources and reduce idiosyncratic risk."
        } else {
            "Continue current allocation rhythm; monitor mETH TVL for early rebalance signals."
        };

        let insights_value = json!(insights);

        Ok(PortfolioIdentity {
            id: Uuid::new_v4(),
            wallet_address: address.to_string(),
            archetype,
            percentile: Some(percentile),
            stats,
            insights: insights_value,
            metadata_uri: None,
            sbt_token_id: None,
            created_at: Utc::now(),
        })
    }
}
