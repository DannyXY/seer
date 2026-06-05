use async_trait::async_trait;
use std::time::Duration;

use chrono::{DateTime, Utc};
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
    defillama: Option<DefiLlamaProvider>,
}

impl ProviderRegistry {
    pub fn new(settings: Settings) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        let nansen = settings
            .nansen_api_key
            .clone()
            .map(|api_key| NansenProvider {
                client: client.clone(),
                api_key,
                base_url: settings
                    .nansen_base_url
                    .clone()
                    .unwrap_or_else(|| "https://api.nansen.ai/api/v1".to_string()),
                cli_path: settings.nansen_cli_path.clone(),
                smart_money_chains: settings.nansen_smart_money_chains.clone(),
            });
        let defillama = settings.defillama_enabled.then(|| DefiLlamaProvider {
            client,
            base_url: settings.defillama_base_url.clone(),
            yields_base_url: settings.defillama_yields_base_url.clone(),
        });
        Self {
            mock: MockProvider,
            nansen,
            defillama,
        }
    }

    pub fn active_name(&self) -> &'static str {
        if self.nansen.is_some() && self.defillama.is_some() {
            "nansen-defillama-mock-fallback"
        } else if self.nansen.is_some() {
            "nansen-or-mock-fallback"
        } else if self.defillama.is_some() {
            "defillama-or-mock-fallback"
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
                Err(err) => warn!("Nansen protocol metrics failed: {err}"),
            }
        }
        if let Some(defillama) = &self.defillama {
            match defillama.get_protocol_metrics(protocol).await {
                Ok(metrics) => return Ok(metrics),
                Err(err) => warn!("DefiLlama protocol metrics failed, using mock fallback: {err}"),
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
    smart_money_chains: Vec<String>,
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

    async fn smart_money_holdings(&self) -> Result<Vec<SmartMoneyMovement>, DataProviderError> {
        let payload = self
            .post_json(
                "/smart-money/holdings",
                json!({
                    "chains": self.smart_money_chains.clone(),
                    "pagination": {
                        "page": 1,
                        "per_page": 50
                    },
                    "order_by": [
                        {
                            "field": "value_usd",
                            "direction": "DESC"
                        }
                    ]
                }),
            )
            .await?;
        Ok(parse_smart_money_holdings(&payload))
    }
}

pub struct DefiLlamaProvider {
    client: reqwest::Client,
    base_url: String,
    yields_base_url: String,
}

impl DefiLlamaProvider {
    async fn get_json(&self, base_url: &str, path: &str) -> Result<Value, DataProviderError> {
        let url = format!(
            "{}/{}",
            base_url.trim_end_matches('/'),
            path.trim_start_matches('/')
        );
        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|err| DataProviderError::Unavailable(err.to_string()))?;
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(DataProviderError::Unavailable(format!(
                "DefiLlama HTTP {status}: {body}"
            )));
        }
        response
            .json::<Value>()
            .await
            .map_err(|err| DataProviderError::Unavailable(err.to_string()))
    }
}

#[async_trait]
impl OnchainDataProvider for DefiLlamaProvider {
    async fn get_wallet_profile(&self, address: &str) -> Result<WalletProfile, DataProviderError> {
        Err(DataProviderError::Unavailable(format!(
            "DefiLlama wallet profile not supported for {address}"
        )))
    }

    async fn get_wallet_positions(
        &self,
        address: &str,
    ) -> Result<Vec<PortfolioPosition>, DataProviderError> {
        Err(DataProviderError::Unavailable(format!(
            "DefiLlama wallet positions not supported for {address}"
        )))
    }

    async fn get_wallet_transactions(
        &self,
        address: &str,
    ) -> Result<Vec<WalletTransaction>, DataProviderError> {
        Err(DataProviderError::Unavailable(format!(
            "DefiLlama wallet transactions not supported for {address}"
        )))
    }

    async fn get_token_flows(&self, token: &str) -> Result<Vec<TokenFlow>, DataProviderError> {
        Err(DataProviderError::Unavailable(format!(
            "DefiLlama token flows not supported for {token}"
        )))
    }

    async fn get_protocol_metrics(
        &self,
        protocol: &str,
    ) -> Result<ProtocolMetrics, DataProviderError> {
        let captured_at = Utc::now();
        let protocols_payload = self.get_json(&self.base_url, "/protocols").await?;
        let yields_payload = self
            .get_json(&self.yields_base_url, "/pools")
            .await
            .unwrap_or_else(|err| {
                warn!("DefiLlama yields pools unavailable, continuing without APY: {err}");
                json!({})
            });
        protocol_metrics_from_defillama(protocol, &protocols_payload, &yields_payload, captured_at)
    }

    async fn get_smart_money_movements(
        &self,
        protocol: Option<&str>,
    ) -> Result<Vec<SmartMoneyMovement>, DataProviderError> {
        Err(DataProviderError::Unavailable(format!(
            "DefiLlama smart money not supported for {:?}",
            protocol
        )))
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
        let movements = self.smart_money_holdings().await?;
        if let Some(protocol) = protocol {
            let filtered: Vec<_> = movements
                .iter()
                .filter(|movement| {
                    normalize_protocol_name(&movement.protocol)
                        .contains(&normalize_protocol_name(protocol))
                        || normalize_protocol_name(protocol)
                            .contains(&normalize_protocol_name(&movement.protocol))
                })
                .cloned()
                .collect();
            if !filtered.is_empty() {
                return Ok(filtered);
            }
        }
        Ok(movements)
    }
}

fn parse_defi_holdings_positions(payload: &Value) -> Vec<PortfolioPosition> {
    candidate_rows(payload)
        .into_iter()
        .filter_map(position_from_value)
        .collect()
}

fn parse_smart_money_holdings(payload: &Value) -> Vec<SmartMoneyMovement> {
    candidate_rows(payload)
        .into_iter()
        .filter_map(smart_money_movement_from_value)
        .collect()
}

fn smart_money_movement_from_value(value: &Value) -> Option<SmartMoneyMovement> {
    let asset = first_string(
        value,
        &[
            &["token_symbol"],
            &["symbol"],
            &["token", "symbol"],
            &["asset_symbol"],
            &["asset", "symbol"],
            &["name"],
            &["token_name"],
        ],
    )?;
    let wallet = first_string(
        value,
        &[
            &["wallet"],
            &["wallet_address"],
            &["address"],
            &["entity"],
            &["entity_name"],
            &["smart_money_type"],
            &["label"],
        ],
    )
    .unwrap_or_else(|| "smart-money-cohort".to_string());
    let protocol = first_string(
        value,
        &[
            &["protocol"],
            &["protocol_name"],
            &["project"],
            &["chain"],
            &["token", "chain"],
        ],
    )
    .unwrap_or_else(|| "Smart Money Holdings".to_string());
    let usd_value = first_f64(
        value,
        &[
            &["value_usd"],
            &["usd_value"],
            &["value"],
            &["balance_usd"],
            &["amount_usd"],
            &["token", "value_usd"],
        ],
    )
    .unwrap_or(0.0);
    let holder_count = first_f64(
        value,
        &[
            &["smart_money_wallet_count"],
            &["wallet_count"],
            &["holders"],
            &["holder_count"],
        ],
    )
    .unwrap_or(1.0);
    let confidence = smart_money_confidence(usd_value, holder_count);

    Some(SmartMoneyMovement {
        wallet,
        protocol,
        asset,
        direction: "holding".to_string(),
        usd_value,
        confidence,
        captured_at: Utc::now(),
    })
}

fn smart_money_confidence(usd_value: f64, holder_count: f64) -> u8 {
    let value_component = if usd_value >= 10_000_000.0 {
        35.0
    } else if usd_value >= 1_000_000.0 {
        25.0
    } else if usd_value >= 100_000.0 {
        15.0
    } else {
        5.0
    };
    let holder_component = (holder_count.max(1.0).ln() * 12.0).clamp(5.0, 35.0);
    (45.0 + value_component + holder_component).clamp(50.0, 95.0) as u8
}

fn protocol_metrics_from_defillama(
    protocol: &str,
    protocols_payload: &Value,
    yields_payload: &Value,
    captured_at: DateTime<Utc>,
) -> Result<ProtocolMetrics, DataProviderError> {
    let protocol_row = candidate_rows(protocols_payload)
        .into_iter()
        .find(|row| defillama_protocol_matches(protocol, row))
        .ok_or_else(|| {
            DataProviderError::Unavailable(format!("DefiLlama protocol not found: {protocol}"))
        })?;
    let tvl_usd = first_f64(
        protocol_row,
        &[&["tvl"], &["chainTvls", "Mantle"], &["chainTvls", "mantle"]],
    )
    .ok_or_else(|| {
        DataProviderError::Unavailable(format!("DefiLlama TVL missing for {protocol}"))
    })?;
    let tvl_change_24h_pct = first_f64(
        protocol_row,
        &[
            &["change_1d"],
            &["change1d"],
            &["tvlPrev1dChange"],
            &["change_24h"],
        ],
    )
    .unwrap_or(0.0);
    let apy = defillama_average_apy(protocol, yields_payload);

    Ok(ProtocolMetrics {
        protocol: canonical_protocol_name(protocol_row).unwrap_or_else(|| protocol.to_string()),
        source_provider: "defillama".to_string(),
        tvl_usd,
        tvl_change_24h_pct,
        apy,
        risk_score: risk_score_from_protocol_metrics(tvl_usd, tvl_change_24h_pct, apy),
        captured_at,
    })
}

fn defillama_average_apy(protocol: &str, payload: &Value) -> Option<f64> {
    let pools: Vec<f64> = candidate_rows(payload)
        .into_iter()
        .filter(|row| defillama_yield_pool_matches(protocol, row))
        .filter_map(|row| first_f64(row, &[&["apy"], &["apyBase"], &["apyReward"]]))
        .filter(|apy| apy.is_finite() && *apy >= 0.0)
        .collect();
    if pools.is_empty() {
        None
    } else {
        Some(pools.iter().sum::<f64>() / pools.len() as f64)
    }
}

fn defillama_protocol_matches(requested: &str, row: &Value) -> bool {
    let aliases = protocol_aliases(requested);
    for path in [&["name"][..], &["slug"][..], &["displayName"][..]] {
        if let Some(value) = first_string(row, &[path]) {
            let normalized = normalize_protocol_name(&value);
            if aliases.iter().any(|alias| normalized == *alias) {
                return true;
            }
        }
    }
    false
}

fn defillama_yield_pool_matches(requested: &str, row: &Value) -> bool {
    let aliases = protocol_aliases(requested);
    let is_mantle = first_string(row, &[&["chain"]])
        .map(|chain| normalize_protocol_name(&chain) == "mantle")
        .unwrap_or(false);
    if !is_mantle {
        return false;
    }
    for path in [
        &["project"][..],
        &["protocol"][..],
        &["poolMeta"][..],
        &["url"][..],
    ] {
        if let Some(value) = first_string(row, &[path]) {
            let normalized = normalize_protocol_name(&value);
            if aliases.iter().any(|alias| normalized.contains(alias)) {
                return true;
            }
        }
    }
    false
}

fn protocol_aliases(protocol: &str) -> Vec<String> {
    let normalized = normalize_protocol_name(protocol);
    let mut aliases = vec![normalized.clone()];
    match normalized.as_str() {
        "methprotocol" | "meth" => {
            aliases.extend(["methprotocol", "mantlestakedether", "meth"].map(str::to_string));
        }
        "merchantmoe" => {
            aliases.extend(["merchantmoe", "moe"].map(str::to_string));
        }
        "agnifinance" => {
            aliases.extend(["agnifinance", "agni"].map(str::to_string));
        }
        "lendle" => {
            aliases.push("lendle".to_string());
        }
        _ => {}
    }
    aliases.sort();
    aliases.dedup();
    aliases
}

fn normalize_protocol_name(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn canonical_protocol_name(row: &Value) -> Option<String> {
    first_string(row, &[&["name"], &["slug"], &["displayName"]])
}

fn risk_score_from_protocol_metrics(tvl_usd: f64, tvl_change_24h_pct: f64, apy: Option<f64>) -> u8 {
    let mut risk: f64 = 45.0;
    if tvl_usd < 5_000_000.0 {
        risk += 25.0;
    } else if tvl_usd < 25_000_000.0 {
        risk += 12.0;
    } else if tvl_usd > 100_000_000.0 {
        risk -= 8.0;
    }
    if tvl_change_24h_pct.abs() > 20.0 {
        risk += 12.0;
    } else if tvl_change_24h_pct.abs() > 10.0 {
        risk += 6.0;
    }
    if apy.is_some_and(|apy| apy > 50.0) {
        risk += 10.0;
    }
    risk.clamp(15.0, 95.0) as u8
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
            source_provider: "mock".to_string(),
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
            parse_defi_holdings_positions, parse_smart_money_holdings,
            protocol_metrics_from_defillama, risk_score_from_positions, OnchainDataProvider,
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
            nansen_smart_money_chains: vec![
                "ethereum".to_string(),
                "solana".to_string(),
                "base".to_string(),
            ],
            defillama_enabled: false,
            defillama_base_url: "https://api.llama.fi".to_string(),
            defillama_yields_base_url: "https://yields.llama.fi".to_string(),
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
    fn parses_nansen_smart_money_holdings_rows() {
        let payload = json!({
            "data": [
                {
                    "token": {
                        "symbol": "MNT",
                        "chain": "mantle"
                    },
                    "entity_name": "Top Fund",
                    "value_usd": 2500000.0,
                    "smart_money_wallet_count": 12
                },
                {
                    "token_symbol": "mETH",
                    "chain": "mantle",
                    "wallet_count": 8,
                    "balance_usd": "750000"
                }
            ]
        });

        let movements = parse_smart_money_holdings(&payload);

        assert_eq!(movements.len(), 2);
        assert_eq!(movements[0].asset, "MNT");
        assert_eq!(movements[0].wallet, "Top Fund");
        assert_eq!(movements[0].protocol, "mantle");
        assert_eq!(movements[0].direction, "holding");
        assert!(movements[0].confidence >= 70);
        assert_eq!(movements[1].asset, "mETH");
        assert_eq!(movements[1].usd_value, 750000.0);
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

    #[test]
    fn parses_defillama_protocol_metrics_with_mantle_yield_apy() {
        let protocols = json!([
            {
                "name": "Lendle",
                "slug": "lendle",
                "tvl": 42500000.0,
                "change_1d": 3.5,
                "chains": ["Mantle"]
            }
        ]);
        let yields = json!({
            "data": [
                {
                    "project": "lendle",
                    "chain": "Mantle",
                    "symbol": "USDC",
                    "tvlUsd": 1200000.0,
                    "apy": 8.4
                },
                {
                    "project": "lendle",
                    "chain": "Arbitrum",
                    "symbol": "USDC",
                    "tvlUsd": 1200000.0,
                    "apy": 50.0
                }
            ]
        });

        let metrics =
            protocol_metrics_from_defillama("Lendle", &protocols, &yields, chrono::Utc::now())
                .unwrap();

        assert_eq!(metrics.protocol, "Lendle");
        assert_eq!(metrics.source_provider, "defillama");
        assert_eq!(metrics.tvl_usd, 42500000.0);
        assert_eq!(metrics.tvl_change_24h_pct, 3.5);
        assert_eq!(metrics.apy, Some(8.4));
    }

    #[test]
    fn defillama_protocol_alias_matches_meth_protocol() {
        let protocols = json!([
            {
                "name": "Mantle Staked Ether",
                "slug": "mantle-staked-ether",
                "tvl": 150000000.0,
                "change_1d": -1.2
            }
        ]);
        let metrics = protocol_metrics_from_defillama(
            "mETH Protocol",
            &protocols,
            &json!({ "data": [] }),
            chrono::Utc::now(),
        )
        .unwrap();

        assert_eq!(metrics.protocol, "Mantle Staked Ether");
        assert_eq!(metrics.source_provider, "defillama");
        assert!(metrics.risk_score < 50);
    }
}
