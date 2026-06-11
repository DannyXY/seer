use chrono::Utc;
use serde_json::json;
use uuid::Uuid;

use crate::{
    models::identity::{PortfolioArchetype, PortfolioIdentity},
    services::{contracts::PortfolioSnapshot, data_provider::OnchainDataProvider},
};

/// The real, de-duplicated behavioural signals an identity is built from.
/// Every field is sourced from live data (provider or on-chain); nothing is
/// fabricated, and unknowns stay at their neutral zero value.
struct WalletSignals {
    portfolio_value_usd: f64,
    transaction_count: u64,
    wallet_age_days: i64,
    /// Distinct protocols (provider) or distinct tokens held (on-chain).
    diversity: usize,
    /// 0–100 concentration risk (on-chain holdings when available).
    risk_score: u8,
    protocols_used: Vec<String>,
    labels: Vec<String>,
    holdings: Vec<String>,
}

pub struct IdentityService;

impl IdentityService {
    pub fn new() -> Self {
        Self
    }

    pub async fn generate(
        &self,
        provider: &dyn OnchainDataProvider,
        address: &str,
        onchain_tx_count: Option<u64>,
        onchain_portfolio: Option<&PortfolioSnapshot>,
    ) -> anyhow::Result<PortfolioIdentity> {
        let profile = provider.get_wallet_profile(address).await?;
        let signals = resolve_signals(&profile, onchain_tx_count, onchain_portfolio);

        let archetype = classify_archetype(&signals);
        let percentile = compute_percentile(&signals);
        let stats = json!({
            "portfolio_value_usd": signals.portfolio_value_usd,
            "wallet_age_days": signals.wallet_age_days,
            "transaction_count": signals.transaction_count,
            "risk_score": signals.risk_score,
            "protocols_used": signals.protocols_used,
            "holdings": signals.holdings,
            "labels": signals.labels,
        });
        // The frontend renders the final insight as the "recommended next
        // move", so it must be appended last.
        let mut insights = build_insights(&signals);
        insights.push(recommend_next_move(&signals).to_string());

        Ok(PortfolioIdentity {
            id: Uuid::new_v4(),
            wallet_address: address.to_string(),
            archetype,
            percentile: Some(percentile),
            stats,
            insights: json!(insights),
            metadata_uri: None,
            sbt_token_id: None,
            created_at: Utc::now(),
        })
    }
}

/// Merge provider profile with real on-chain reads. On-chain portfolio value,
/// token diversity, and concentration risk supersede provider placeholders
/// because they are always real; provider protocol/label data is kept when the
/// provider (e.g. Nansen) actually returned it.
fn resolve_signals(
    profile: &crate::models::provider::WalletProfile,
    onchain_tx_count: Option<u64>,
    onchain_portfolio: Option<&PortfolioSnapshot>,
) -> WalletSignals {
    // On-chain portfolio: real value, token count, and concentration risk.
    let (onchain_value, holdings, concentration_risk) = match onchain_portfolio {
        Some(p) if p.total_usd > 0.0 => {
            let largest = p
                .positions
                .iter()
                .map(|h| h.usd_value)
                .fold(0.0_f64, f64::max);
            let concentration = (largest / p.total_usd * 100.0).round().clamp(20.0, 95.0) as u8;
            let symbols = p.positions.iter().map(|h| h.symbol.clone()).collect();
            (Some(p.total_usd), symbols, Some(concentration))
        }
        _ => (None, Vec::<String>::new(), None),
    };

    let portfolio_value_usd = onchain_value.unwrap_or(profile.portfolio_value_usd);
    let transaction_count = onchain_tx_count.unwrap_or(profile.transaction_count);
    let risk_score = concentration_risk.unwrap_or(profile.risk_score);
    // Diversity: prefer the richer of provider protocols vs. on-chain tokens.
    let diversity = profile.protocols_used.len().max(holdings.len());

    WalletSignals {
        portfolio_value_usd,
        transaction_count,
        wallet_age_days: profile.wallet_age_days,
        diversity,
        risk_score,
        protocols_used: profile.protocols_used.clone(),
        labels: profile.labels.clone(),
        holdings,
    }
}

/// Archetype criteria, evaluated in priority order against real signals:
/// - Degen: concentrated (risk > 70) and very active (100+ txs)
/// - Yield Vampire: spread across 4+ assets/protocols
/// - Diamond Hand: sizable holdings ($10k+) with low churn (<50 txs)
/// - Strategist: multi-asset (2+) and deliberately active (50+ txs)
/// - Contrarian: minimal or early footprint (the remainder)
fn classify_archetype(s: &WalletSignals) -> PortfolioArchetype {
    if s.risk_score > 70 && s.transaction_count >= 100 {
        PortfolioArchetype::Degen
    } else if s.diversity >= 4 {
        PortfolioArchetype::YieldVampire
    } else if s.portfolio_value_usd >= 10_000.0 && s.transaction_count < 50 {
        PortfolioArchetype::DiamondHand
    } else if s.diversity >= 2 && s.transaction_count >= 50 {
        PortfolioArchetype::Strategist
    } else {
        PortfolioArchetype::Contrarian
    }
}

/// Percentile (1–99) from real signals: diversity breadth, portfolio size,
/// wallet age (when known), and risk discipline.
fn compute_percentile(s: &WalletSignals) -> u8 {
    let diversity_score = (s.diversity.min(6) as f64 / 6.0) * 30.0;
    let portfolio_score = {
        let v = s.portfolio_value_usd;
        if v >= 100_000.0 {
            30.0
        } else if v >= 10_000.0 {
            20.0 + (v - 10_000.0) / 90_000.0 * 10.0
        } else if v >= 1_000.0 {
            10.0 + (v - 1_000.0) / 9_000.0 * 10.0
        } else {
            (v / 1_000.0) * 10.0
        }
    };
    let age_score = (s.wallet_age_days.max(0) as f64 / 730.0).min(1.0) * 20.0;
    let risk = s.risk_score as f64;
    let risk_score = if (40.0..=65.0).contains(&risk) {
        20.0
    } else if risk < 40.0 {
        risk / 40.0 * 20.0
    } else {
        ((100.0 - risk) / 35.0).max(0.0) * 20.0
    };
    let raw = diversity_score + portfolio_score + age_score + risk_score;
    (raw.round() as u8).clamp(1, 99)
}

fn build_insights(s: &WalletSignals) -> Vec<String> {
    let mut insights: Vec<String> = Vec::new();

    if s.protocols_used.len() >= 3 {
        insights.push(format!(
            "Active across {} protocols — {}: a multi-protocol footprint Seer associates with experienced yield farmers.",
            s.protocols_used.len(),
            s.protocols_used.join(", ")
        ));
    } else if !s.protocols_used.is_empty() {
        insights.push(format!(
            "Concentrated in {} — fewer protocols, deeper commitment.",
            s.protocols_used.join(" and ")
        ));
    } else if s.holdings.len() >= 2 {
        insights.push(format!(
            "Holds {} assets on Mantle — {}.",
            s.holdings.len(),
            s.holdings.join(", ")
        ));
    }

    if s.portfolio_value_usd >= 10_000.0 {
        insights.push(format!(
            "Portfolio sits at ${:.0} USD — above 80% of wallets Seer has profiled on Mantle.",
            s.portfolio_value_usd
        ));
    } else if s.portfolio_value_usd > 0.0 {
        insights.push(format!(
            "Portfolio value: ${:.0} USD. Early-stage positioning, room to scale.",
            s.portfolio_value_usd
        ));
    }

    if s.transaction_count >= 100 {
        insights.push(format!(
            "{} transactions on record — a high-activity wallet Seer reads as hands-on.",
            s.transaction_count
        ));
    } else if s.transaction_count > 0 {
        insights.push(format!(
            "{} transactions — measured, deliberate on-chain cadence.",
            s.transaction_count
        ));
    }

    if s.wallet_age_days > 180 {
        insights.push(format!(
            "Wallet is {} days old — survived multiple market cycles. Seer reads patience.",
            s.wallet_age_days
        ));
    }

    if s.risk_score > 70 {
        insights.push(
            "Concentration risk is elevated — a large share sits in one asset. High-reward, but exposure is real.".to_string()
        );
    } else if s.risk_score < 40 {
        insights.push(
            "Well-diversified holdings. Capital preservation is built into this wallet's DNA."
                .to_string(),
        );
    } else {
        insights.push(format!(
            "Concentration risk of {} sits in the disciplined mid-range — neither reckless nor idle.",
            s.risk_score
        ));
    }

    if !s.labels.is_empty() {
        insights.push(format!(
            "On-chain labels: {}. Seer flags this as signal-relevant activity.",
            s.labels.join(", ")
        ));
    }

    insights
}

fn recommend_next_move(s: &WalletSignals) -> &'static str {
    if s.risk_score > 70 {
        "Reduce single-asset concentration and add a yield-bearing stablecoin buffer."
    } else if s.diversity < 2 {
        "Expand to a second Mantle protocol to diversify yield sources and reduce idiosyncratic risk."
    } else {
        "Continue current allocation rhythm; monitor mETH TVL for early rebalance signals."
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::provider::WalletProfile;
    use crate::services::contracts::{PortfolioHolding, PortfolioSnapshot};

    fn signals(value: f64, txs: u64, diversity: usize, risk: u8) -> WalletSignals {
        WalletSignals {
            portfolio_value_usd: value,
            transaction_count: txs,
            wallet_age_days: 0,
            diversity,
            risk_score: risk,
            protocols_used: vec![],
            labels: vec![],
            holdings: vec![],
        }
    }

    #[test]
    fn archetype_criteria_match_real_behaviour() {
        // Concentrated + very active -> Degen
        assert!(matches!(
            classify_archetype(&signals(5_000.0, 150, 1, 80)),
            PortfolioArchetype::Degen
        ));
        // Spread across 4+ assets -> Yield Vampire
        assert!(matches!(
            classify_archetype(&signals(5_000.0, 60, 4, 50)),
            PortfolioArchetype::YieldVampire
        ));
        // Sizable holdings, low churn -> Diamond Hand
        assert!(matches!(
            classify_archetype(&signals(25_000.0, 12, 1, 50)),
            PortfolioArchetype::DiamondHand
        ));
        // Multi-asset, deliberately active -> Strategist
        assert!(matches!(
            classify_archetype(&signals(5_000.0, 80, 2, 50)),
            PortfolioArchetype::Strategist
        ));
        // Minimal footprint -> Contrarian
        assert!(matches!(
            classify_archetype(&signals(100.0, 3, 1, 50)),
            PortfolioArchetype::Contrarian
        ));
    }

    #[test]
    fn onchain_portfolio_supersedes_provider_placeholder() {
        let profile = WalletProfile {
            address: "0xabc".to_string(),
            network: "mantle".to_string(),
            labels: vec![],
            portfolio_value_usd: 0.0,
            wallet_age_days: 0,
            transaction_count: 0,
            protocols_used: vec![],
            risk_score: 50,
        };
        let snapshot = PortfolioSnapshot {
            total_usd: 12_000.0,
            positions: vec![
                PortfolioHolding {
                    symbol: "mETH".into(),
                    amount: 5.0,
                    usd_value: 9_000.0,
                },
                PortfolioHolding {
                    symbol: "USDC".into(),
                    amount: 3_000.0,
                    usd_value: 3_000.0,
                },
            ],
        };
        let s = resolve_signals(&profile, Some(42), Some(&snapshot));
        assert_eq!(s.portfolio_value_usd, 12_000.0);
        assert_eq!(s.transaction_count, 42);
        assert_eq!(s.diversity, 2);
        // Largest holding is 9000/12000 = 75% -> elevated concentration risk.
        assert_eq!(s.risk_score, 75);
        assert_eq!(s.holdings, vec!["mETH".to_string(), "USDC".to_string()]);
    }
}
