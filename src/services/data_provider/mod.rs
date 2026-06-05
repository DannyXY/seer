use async_trait::async_trait;
use chrono::Utc;
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use serde_json::{json, Value};
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
                client: reqwest::Client::new(),
                api_key,
                base_url: settings
                    .nansen_base_url
                    .clone()
                    .unwrap_or_else(|| "https://api.nansen.ai/api/v1".to_string()),
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
    client: reqwest::Client,
    api_key: String,
    base_url: String,
    cli_path: String,
}

impl NansenProvider {
    async fn post_json(&self, path: &str, body: Value) -> Result<Value, DataProviderError> {
        let url = format!(
            "{}/{}",
            self.base_url.trim_end_matches('/'),
            path.trim_start_matches('/')
        );
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(
            "apiKey",
            HeaderValue::from_str(&self.api_key)
                .map_err(|err| DataProviderError::Unavailable(err.to_string()))?,
        );

        let response = self
            .client
            .post(url)
            .headers(headers)
            .json(&body)
            .send()
            .await
            .map_err(|err| DataProviderError::Unavailable(err.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(DataProviderError::Unavailable(format!(
                "Nansen HTTP {status}: {body}"
            )));
        }

        response
            .json::<Value>()
            .await
            .map_err(|err| DataProviderError::Unavailable(err.to_string()))
    }

    async fn defi_holdings(
        &self,
        address: &str,
    ) -> Result<Vec<PortfolioPosition>, DataProviderError> {
        let payload = self
            .post_json(
                "/portfolio/defi-holdings",
                json!({
                    "wallet_address": address
                }),
            )
            .await?;
        Ok(parse_defi_holdings_positions(&payload))
    }
}

#[async_trait]
impl OnchainDataProvider for NansenProvider {
    async fn get_wallet_profile(&self, address: &str) -> Result<WalletProfile, DataProviderError> {
        let positions = self.defi_holdings(address).await?;
        let protocols_used = unique_protocols(&positions);
        let portfolio_value_usd = positions.iter().map(|position| position.usd_value).sum();
        Ok(WalletProfile {
            address: address.to_string(),
            network: "mantle".to_string(),
            labels: vec!["nansen-defi-holdings".to_string()],
            portfolio_value_usd,
            wallet_age_days: 0,
            transaction_count: 0,
            protocols_used,
            risk_score: risk_score_from_positions(&positions),
        })
    }

    async fn get_wallet_positions(
        &self,
        address: &str,
    ) -> Result<Vec<PortfolioPosition>, DataProviderError> {
        self.defi_holdings(address).await
    }

    async fn get_wallet_transactions(
        &self,
        address: &str,
    ) -> Result<Vec<WalletTransaction>, DataProviderError> {
        let _ = &self.cli_path;
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

fn parse_defi_holdings_positions(payload: &Value) -> Vec<PortfolioPosition> {
    candidate_rows(payload)
        .into_iter()
        .filter_map(position_from_value)
        .collect()
}

fn candidate_rows(payload: &Value) -> Vec<&Value> {
    if let Some(array) = payload.as_array() {
        return array.iter().collect();
    }
    for key in [
        "data",
        "result",
        "results",
        "holdings",
        "items",
        "rows",
        "positions",
    ] {
        if let Some(value) = payload.get(key) {
            let rows = candidate_rows(value);
            if !rows.is_empty() {
                return rows;
            }
        }
    }
    Vec::new()
}

fn position_from_value(value: &Value) -> Option<PortfolioPosition> {
    let symbol = first_string(
        value,
        &[
            &["symbol"],
            &["token_symbol"],
            &["asset_symbol"],
            &["token", "symbol"],
            &["asset", "symbol"],
        ],
    )?;
    let amount = first_string(
        value,
        &[&["amount"], &["balance"], &["quantity"], &["token_amount"]],
    )
    .or_else(|| {
        first_f64(
            value,
            &[&["amount"], &["balance"], &["quantity"], &["token_amount"]],
        )
        .map(|value| value.to_string())
    })
    .unwrap_or_else(|| "0".to_string());
    let usd_value = first_f64(
        value,
        &[
            &["usd_value"],
            &["value_usd"],
            &["value"],
            &["balance_usd"],
            &["token", "value_usd"],
        ],
    )
    .unwrap_or(0.0);
    let protocol = first_string(
        value,
        &[
            &["protocol"],
            &["protocol_name"],
            &["project"],
            &["project_name"],
            &["app"],
            &["app_name"],
            &["protocol", "name"],
        ],
    );

    Some(PortfolioPosition {
        symbol,
        amount,
        usd_value,
        protocol,
    })
}

fn first_string(value: &Value, paths: &[&[&str]]) -> Option<String> {
    paths.iter().find_map(|path| {
        path_value(value, path).and_then(|value| {
            value
                .as_str()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
        })
    })
}

fn first_f64(value: &Value, paths: &[&[&str]]) -> Option<f64> {
    paths.iter().find_map(|path| {
        path_value(value, path).and_then(|value| {
            value.as_f64().or_else(|| {
                value
                    .as_str()
                    .and_then(|raw| raw.replace(',', "").parse().ok())
            })
        })
    })
}

fn path_value<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Value> {
    path.iter()
        .try_fold(value, |current, key| current.get(*key))
}

fn unique_protocols(positions: &[PortfolioPosition]) -> Vec<String> {
    let mut protocols = Vec::new();
    for position in positions {
        if let Some(protocol) = &position.protocol {
            if !protocols.iter().any(|seen| seen == protocol) {
                protocols.push(protocol.clone());
            }
        }
    }
    protocols
}

fn risk_score_from_positions(positions: &[PortfolioPosition]) -> u8 {
    let total: f64 = positions.iter().map(|position| position.usd_value).sum();
    if total <= 0.0 {
        return 50;
    }
    let largest = positions
        .iter()
        .map(|position| position.usd_value)
        .fold(0.0, f64::max);
    let concentration = (largest / total * 100.0).round();
    concentration.clamp(20.0, 95.0) as u8
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
    use serde_json::json;

    use crate::{
        config::{AppRole, Settings},
        services::data_provider::{
            parse_defi_holdings_positions, risk_score_from_positions, OnchainDataProvider,
            ProviderRegistry,
        },
    };

    fn settings_with_nansen() -> Settings {
        Settings {
            app_env: "test".to_string(),
            app_role: AppRole::Api,
            port: 10000,
            version: "test".to_string(),
            database_url: None,
            run_migrations: false,
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
            approved_strategy_spender_address: None,
            strategy_deposit_function: "deposit(address,uint256)".to_string(),
            merchant_moe_strategy_address: None,
            merchant_moe_spender_address: None,
            merchant_moe_deposit_function: None,
            lendle_strategy_address: None,
            lendle_spender_address: None,
            lendle_deposit_function: None,
            agni_strategy_address: None,
            agni_spender_address: None,
            agni_deposit_function: None,
            meth_strategy_address: None,
            meth_spender_address: None,
            meth_deposit_function: None,
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

    #[test]
    fn parses_nansen_defi_holdings_positions_from_data_array() {
        let payload = json!({
            "data": [
                {
                    "token": { "symbol": "USDC" },
                    "balance": "25.5",
                    "value_usd": 25.5,
                    "protocol": { "name": "Merchant Moe" }
                },
                {
                    "asset_symbol": "mETH",
                    "amount": 1.2,
                    "usd_value": "4200.50",
                    "protocol_name": "mETH Protocol"
                }
            ]
        });

        let positions = parse_defi_holdings_positions(&payload);

        assert_eq!(positions.len(), 2);
        assert_eq!(positions[0].symbol, "USDC");
        assert_eq!(positions[0].amount, "25.5");
        assert_eq!(positions[0].protocol.as_deref(), Some("Merchant Moe"));
        assert_eq!(positions[1].symbol, "mETH");
        assert_eq!(positions[1].usd_value, 4200.50);
    }

    #[test]
    fn derives_position_concentration_risk_score() {
        let positions = parse_defi_holdings_positions(&json!({
            "holdings": [
                { "symbol": "USDC", "amount": "50", "value_usd": 50 },
                { "symbol": "MNT", "amount": "50", "value_usd": 50 }
            ]
        }));

        assert_eq!(risk_score_from_positions(&positions), 50);
    }
}
