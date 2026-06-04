use async_trait::async_trait;
use chrono::Utc;
use thiserror::Error;
use tracing::warn;

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
        self
    }
}

#[async_trait]
impl OnchainDataProvider for ProviderRegistry {
    async fn get_wallet_profile(&self, address: &str) -> Result<WalletProfile, DataProviderError> {
        if let Some(nansen) = &self.nansen {
            match nansen.get_wallet_profile(address).await {
                Ok(profile) => return Ok(profile),
                Err(err) => warn!("Nansen wallet profile failed, using mock fallback: {err}"),
            }
        }
        self.mock.get_wallet_profile(address).await
    }

    async fn get_wallet_positions(
        &self,
        address: &str,
    ) -> Result<Vec<PortfolioPosition>, DataProviderError> {
        if let Some(nansen) = &self.nansen {
            match nansen.get_wallet_positions(address).await {
                Ok(positions) => return Ok(positions),
                Err(err) => warn!("Nansen wallet positions failed, using mock fallback: {err}"),
            }
        }
        self.mock.get_wallet_positions(address).await
    }

    async fn get_wallet_transactions(
        &self,
        address: &str,
    ) -> Result<Vec<WalletTransaction>, DataProviderError> {
        if let Some(nansen) = &self.nansen {
            match nansen.get_wallet_transactions(address).await {
                Ok(transactions) => return Ok(transactions),
                Err(err) => warn!("Nansen wallet transactions failed, using mock fallback: {err}"),
            }
        }
        self.mock.get_wallet_transactions(address).await
    }

    async fn get_token_flows(&self, token: &str) -> Result<Vec<TokenFlow>, DataProviderError> {
        if let Some(nansen) = &self.nansen {
            match nansen.get_token_flows(token).await {
                Ok(flows) => return Ok(flows),
                Err(err) => warn!("Nansen token flows failed, using mock fallback: {err}"),
            }
        }
        self.mock.get_token_flows(token).await
    }

    async fn get_protocol_metrics(
        &self,
        protocol: &str,
    ) -> Result<ProtocolMetrics, DataProviderError> {
        if let Some(nansen) = &self.nansen {
            match nansen.get_protocol_metrics(protocol).await {
                Ok(metrics) => return Ok(metrics),
                Err(err) => warn!("Nansen protocol metrics failed, using mock fallback: {err}"),
            }
        }
        self.mock.get_protocol_metrics(protocol).await
    }

    async fn get_smart_money_movements(
        &self,
        protocol: Option<&str>,
    ) -> Result<Vec<SmartMoneyMovement>, DataProviderError> {
        if let Some(nansen) = &self.nansen {
            match nansen.get_smart_money_movements(protocol).await {
                Ok(movements) => return Ok(movements),
                Err(err) => warn!("Nansen smart money failed, using mock fallback: {err}"),
            }
        }
        self.mock.get_smart_money_movements(protocol).await
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

#[cfg(test)]
mod tests {
    use crate::{
        config::{AppRole, Settings},
        services::data_provider::{OnchainDataProvider, ProviderRegistry},
    };

    fn settings_with_nansen() -> Settings {
        Settings {
            app_env: "test".to_string(),
            app_role: AppRole::Api,
            port: 10000,
            version: "test".to_string(),
            database_url: None,
            redis_url: None,
            claude_api_key: None,
            claude_model: "claude-sonnet-4-20250514".to_string(),
            nansen_api_key: Some("test-key".to_string()),
            nansen_base_url: None,
            nansen_cli_path: "nansen".to_string(),
            mantle_rpc_url: None,
            mantle_chain_id: 5003,
            aa_bundler_url: None,
            backend_signer_private_key: None,
            mantle_usdc_address: None,
            mantle_usdt_address: None,
            mantle_mnt_address: None,
            mantle_meth_address: None,
            approved_strategy_address: None,
            strategy_deposit_function: "deposit(address,uint256)".to_string(),
            arena_points_address: None,
            prediction_registry_address: None,
            identity_sbt_address: None,
            intent_registry_address: None,
        }
    }

    #[tokio::test]
    async fn registry_falls_back_when_nansen_wallet_profile_is_unavailable() {
        let registry = ProviderRegistry::new(settings_with_nansen());
        let profile = registry
            .get_wallet_profile("0x1234567890123456789012345678901234567890")
            .await
            .unwrap();

        assert_eq!(registry.active_name(), "nansen-or-mock-fallback");
        assert_eq!(profile.network, "mantle");
        assert!(profile
            .protocols_used
            .contains(&"mETH Protocol".to_string()));
    }

    #[tokio::test]
    async fn registry_falls_back_when_nansen_protocol_metrics_are_unavailable() {
        let registry = ProviderRegistry::new(settings_with_nansen());
        let metrics = registry
            .get_protocol_metrics("mETH Protocol")
            .await
            .unwrap();

        assert_eq!(metrics.protocol, "mETH Protocol");
        assert_eq!(metrics.risk_score, 46);
    }
}
