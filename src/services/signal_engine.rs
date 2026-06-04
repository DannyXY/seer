use chrono::Utc;
use serde_json::json;
use uuid::Uuid;

use crate::{
    models::signals::{Signal, SignalCategory},
    services::data_provider::OnchainDataProvider,
};

pub struct SignalEngine;

impl SignalEngine {
    pub fn new() -> Self {
        Self
    }

    pub async fn generate(
        &self,
        provider: &dyn OnchainDataProvider,
    ) -> anyhow::Result<Vec<Signal>> {
        let movements = provider
            .get_smart_money_movements(Some("mETH Protocol"))
            .await
            .unwrap_or_default();
        let metrics = provider.get_protocol_metrics("mETH Protocol").await.ok();
        let flows = provider.get_token_flows("mETH").await.unwrap_or_default();

        let mut signals = Vec::new();

        if let Some(movement) = movements.first() {
            signals.push(Signal {
                id: Uuid::from_u128(0x11111111111111111111111111111111),
                category: SignalCategory::Alpha,
                headline: format!("Smart money {} {}", movement.direction, movement.protocol),
                explanation: "Structured provider data shows high-confidence wallet movement into the protocol.".to_string(),
                confidence_score: movement.confidence,
                related_wallet: Some(movement.wallet.clone()),
                related_protocol: Some(movement.protocol.clone()),
                related_asset: Some(movement.asset.clone()),
                source_provider: "mock-or-nansen".to_string(),
                source_data: json!(movement),
                created_at: Utc::now(),
            });
        }

        if let Some(metrics) = metrics {
            if metrics.tvl_change_24h_pct >= 8.0 {
                signals.push(Signal {
                    id: Uuid::from_u128(0x22222222222222222222222222222222),
                    category: SignalCategory::Opportunity,
                    headline: format!("{} TVL climbed {:.1}% in 24h", metrics.protocol, metrics.tvl_change_24h_pct),
                    explanation: "The protocol crossed Seer's movement threshold and is eligible for Arena prediction generation.".to_string(),
                    confidence_score: 74,
                    related_wallet: None,
                    related_protocol: Some(metrics.protocol.clone()),
                    related_asset: None,
                    source_provider: "mock-or-nansen".to_string(),
                    source_data: json!(metrics),
                    created_at: Utc::now(),
                });
            }
        }

        if let Some(flow) = flows.first() {
            if flow.smart_money_wallet_count >= 5 {
                signals.push(Signal {
                    id: Uuid::from_u128(0x33333333333333333333333333333333),
                    category: SignalCategory::Anomaly,
                    headline: format!("{} smart-money flow intensified", flow.token),
                    explanation: "Multiple smart-money wallets moved into the same asset window."
                        .to_string(),
                    confidence_score: 81,
                    related_wallet: None,
                    related_protocol: flow.protocol.clone(),
                    related_asset: Some(flow.token.clone()),
                    source_provider: "mock-or-nansen".to_string(),
                    source_data: json!(flow),
                    created_at: Utc::now(),
                });
            }
        }

        Ok(signals)
    }
}
