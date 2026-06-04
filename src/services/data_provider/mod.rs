use async_trait::async_trait;
use chrono::Utc;
use thiserror::Error;

use crate::{config::Settings, models::provider::*};

#[derive(Debug, Error)]
pub enum DataProviderError {
    #[error("provider unavailable: {0}")]
    Unavailable(String),
}

#[async_trait]
pub trait OnchainDataProvider: Send + Sync {
    async fn get_wallet_profile(&self, address: &str) -> Result<WalletProfile, DataProviderError>;
    async fn get_wallet_positions(
        &self,
        address: &str,
    ) -> Result<Vec<PortfolioPosition>, DataProviderError>;
    async fn get_wallet_transactions(
        &self,
        address: &str,
    ) -> Result<Vec<WalletTransaction>, DataProviderError>;
    async fn get_token_flows(&self, token: &str) -> Result<Vec<TokenFlow>, DataProviderError>;
    async fn get_protocol_metrics(
        &self,
        protocol: &str,
    ) -> Result<ProtocolMetrics, DataProviderError>;
    async fn get_smart_money_movements(
        &self,
        protocol: Option<&str>,
    ) -> Result<Vec<SmartMoneyMovement>, DataProviderError>;
}

pub struct ProviderRegistry {
    mock: MockProvider,
    nansen: Option<NansenProvider>,
}

impl ProviderRegistry {
    pub fn new(settings: Settings) -> Self {
        let nansen = settings
            .nansen_api_key
            .clone()
            .map(|api_key| NansenProvider {
                api_key,
                base_url: settings.nansen_base_url.clone(),
                cli_path: settings.nansen_cli_path.clone(),
            });
        Self {
            mock: MockProvider,
            nansen,
        }
    }

    pub fn active_name(&self) -> &'static str {
        if self.nansen.is_some() {
            "nansen-or-mock-fallback"
        } else {
            "mock"
        }
    }

    pub async fn provider(&self) -> &dyn OnchainDataProvider {
        self.nansen
            .as_ref()
            .map(|p| p as &dyn OnchainDataProvider)
            .unwrap_or(&self.mock)
    }
}

pub struct NansenProvider {
    api_key: String,
    base_url: Option<String>,
    cli_path: String,
}

#[async_trait]
impl OnchainDataProvider for NansenProvider {
    async fn get_wallet_profile(&self, address: &str) -> Result<WalletProfile, DataProviderError> {
        let _ = (&self.api_key, &self.base_url, &self.cli_path);
        Err(DataProviderError::Unavailable(format!(
            "Nansen provider scaffolded for {address}; wire nansen-cli/API output mapping next"
        )))
    }

    async fn get_wallet_positions(
        &self,
        address: &str,
    ) -> Result<Vec<PortfolioPosition>, DataProviderError> {
        Err(DataProviderError::Unavailable(format!(
            "Nansen positions not wired for {address}"
        )))
    }

    async fn get_wallet_transactions(
        &self,
        address: &str,
    ) -> Result<Vec<WalletTransaction>, DataProviderError> {
        Err(DataProviderError::Unavailable(format!(
            "Nansen transactions not wired for {address}"
        )))
    }

    async fn get_token_flows(&self, token: &str) -> Result<Vec<TokenFlow>, DataProviderError> {
        Err(DataProviderError::Unavailable(format!(
            "Nansen token flows not wired for {token}"
        )))
    }

    async fn get_protocol_metrics(
        &self,
        protocol: &str,
    ) -> Result<ProtocolMetrics, DataProviderError> {
        Err(DataProviderError::Unavailable(format!(
            "Nansen protocol metrics not wired for {protocol}"
        )))
    }

    async fn get_smart_money_movements(
        &self,
        protocol: Option<&str>,
    ) -> Result<Vec<SmartMoneyMovement>, DataProviderError> {
        Err(DataProviderError::Unavailable(format!(
            "Nansen smart money not wired for {:?}",
            protocol
        )))
    }
}

pub struct MockProvider;

#[async_trait]
impl OnchainDataProvider for MockProvider {
    async fn get_wallet_profile(&self, address: &str) -> Result<WalletProfile, DataProviderError> {
        Ok(WalletProfile {
            address: address.to_string(),
            network: "mantle".to_string(),
            labels: vec!["yield-seeker".to_string(), "early-mantle-user".to_string()],
            portfolio_value_usd: 8420.55,
            wallet_age_days: 184,
            transaction_count: 143,
            protocols_used: vec![
                "Agni Finance".to_string(),
                "Merchant Moe".to_string(),
                "mETH Protocol".to_string(),
            ],
            risk_score: 58,
        })
    }

    async fn get_wallet_positions(
        &self,
        _address: &str,
    ) -> Result<Vec<PortfolioPosition>, DataProviderError> {
        Ok(vec![
            PortfolioPosition {
                symbol: "MNT".to_string(),
                amount: "420.5".to_string(),
                usd_value: 512.31,
                protocol: None,
            },
            PortfolioPosition {
                symbol: "mETH".to_string(),
                amount: "2.1".to_string(),
                usd_value: 7230.02,
                protocol: Some("mETH Protocol".to_string()),
            },
            PortfolioPosition {
                symbol: "USDT".to_string(),
                amount: "678.22".to_string(),
                usd_value: 678.22,
                protocol: Some("Agni Finance".to_string()),
            },
        ])
    }

    async fn get_wallet_transactions(
        &self,
        _address: &str,
    ) -> Result<Vec<WalletTransaction>, DataProviderError> {
        Ok(vec![WalletTransaction {
            hash: "0xmocktx001".to_string(),
            timestamp: Utc::now(),
            protocol: Some("Agni Finance".to_string()),
            asset: Some("mETH".to_string()),
            direction: "in".to_string(),
            usd_value: Some(1250.0),
        }])
    }

    async fn get_token_flows(&self, token: &str) -> Result<Vec<TokenFlow>, DataProviderError> {
        Ok(vec![TokenFlow {
            token: token.to_string(),
            protocol: Some("mETH Protocol".to_string()),
            net_flow_usd: 1_250_000.0,
            wallet_count: 73,
            smart_money_wallet_count: 8,
            captured_at: Utc::now(),
        }])
    }

    async fn get_protocol_metrics(
        &self,
        protocol: &str,
    ) -> Result<ProtocolMetrics, DataProviderError> {
        Ok(ProtocolMetrics {
            protocol: protocol.to_string(),
            tvl_usd: 42_000_000.0,
            tvl_change_24h_pct: 9.4,
            apy: Some(12.8),
            risk_score: 46,
            captured_at: Utc::now(),
        })
    }

    async fn get_smart_money_movements(
        &self,
        protocol: Option<&str>,
    ) -> Result<Vec<SmartMoneyMovement>, DataProviderError> {
        Ok(vec![SmartMoneyMovement {
            wallet: "0xsmart000000000000000000000000000000000001".to_string(),
            protocol: protocol.unwrap_or("mETH Protocol").to_string(),
            asset: "mETH".to_string(),
            direction: "entered".to_string(),
            usd_value: 284_000.0,
            confidence: 86,
            captured_at: Utc::now(),
        }])
    }
}
