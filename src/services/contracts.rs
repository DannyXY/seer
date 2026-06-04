use ethers_core::{
    abi::{encode, Token},
    types::Address,
    utils::id,
};
use serde_json::{json, Value};
use std::str::FromStr;

use crate::config::Settings;
use crate::models::execution::{
    Erc20AllowanceRequest, Erc20AllowanceResponse, SendRawTransactionRequest,
    SendRawTransactionResponse, SendUserOperationRequest, SendUserOperationResponse,
    UserOperationReceiptRequest,
};

pub struct ContractService {
    pub rpc_url: Option<String>,
    pub private_key: Option<String>,
    pub arena_points_address: Option<String>,
    pub prediction_registry_address: Option<String>,
    pub identity_sbt_address: Option<String>,
    pub intent_registry_address: Option<String>,
    pub aa_bundler_url: Option<String>,
    client: reqwest::Client,
}

impl ContractService {
    pub fn new(settings: Settings) -> Self {
        Self {
            rpc_url: settings.mantle_rpc_url,
            private_key: settings.backend_signer_private_key,
            arena_points_address: settings.arena_points_address,
            prediction_registry_address: settings.prediction_registry_address,
            identity_sbt_address: settings.identity_sbt_address,
            intent_registry_address: settings.intent_registry_address,
            aa_bundler_url: settings.aa_bundler_url,
            client: reqwest::Client::new(),
        }
    }

    pub fn is_configured(&self) -> bool {
        self.rpc_url.is_some() && self.private_key.is_some()
    }

    pub fn rpc_configured(&self) -> bool {
        self.rpc_url.is_some()
    }

    pub fn bundler_configured(&self) -> bool {
        self.aa_bundler_url.is_some()
    }

    pub async fn chain_id(&self) -> anyhow::Result<Option<u64>> {
        let Some(rpc_url) = &self.rpc_url else {
            return Ok(None);
        };
        let response = self.rpc_call(rpc_url, "eth_chainId", json!([])).await?;
        let chain_id_hex = response
            .get("result")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow::anyhow!("eth_chainId response missing result"))?;
        let chain_id = u64::from_str_radix(chain_id_hex.trim_start_matches("0x"), 16)?;
        Ok(Some(chain_id))
    }

    pub async fn send_raw_transaction(
        &self,
        request: SendRawTransactionRequest,
    ) -> anyhow::Result<SendRawTransactionResponse> {
        let rpc_url = self
            .rpc_url
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("MANTLE_RPC_URL is not configured"))?;

        if !request.signed_transaction.starts_with("0x") {
            anyhow::bail!("signed_transaction must be 0x-prefixed");
        }

        let response = self
            .rpc_call(
                rpc_url,
                "eth_sendRawTransaction",
                json!([request.signed_transaction]),
            )
            .await?;

        if let Some(error) = response.get("error") {
            anyhow::bail!("rpc error: {error}");
        }

        let tx_hash = response
            .get("result")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow::anyhow!("eth_sendRawTransaction response missing result"))?;

        Ok(SendRawTransactionResponse {
            tx_hash: tx_hash.to_string(),
        })
    }

    pub async fn send_user_operation(
        &self,
        request: SendUserOperationRequest,
    ) -> anyhow::Result<SendUserOperationResponse> {
        let bundler_url = self
            .aa_bundler_url
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("AA_BUNDLER_URL is not configured"))?;

        validate_hex_address(&request.entry_point, "entry_point")?;
        let response = self
            .rpc_call(
                bundler_url,
                "eth_sendUserOperation",
                json!([request.user_operation, request.entry_point]),
            )
            .await?;

        if let Some(error) = response.get("error") {
            anyhow::bail!("bundler error: {error}");
        }

        let user_operation_hash = response
            .get("result")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow::anyhow!("eth_sendUserOperation response missing result"))?;

        Ok(SendUserOperationResponse {
            user_operation_hash: user_operation_hash.to_string(),
        })
    }

    pub async fn user_operation_receipt(
        &self,
        request: UserOperationReceiptRequest,
    ) -> anyhow::Result<Value> {
        let bundler_url = self
            .aa_bundler_url
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("AA_BUNDLER_URL is not configured"))?;

        if !request.user_operation_hash.starts_with("0x") {
            anyhow::bail!("user_operation_hash must be 0x-prefixed");
        }

        let response = self
            .rpc_call(
                bundler_url,
                "eth_getUserOperationReceipt",
                json!([request.user_operation_hash]),
            )
            .await?;

        if let Some(error) = response.get("error") {
            anyhow::bail!("bundler error: {error}");
        }

        Ok(response.get("result").cloned().unwrap_or(Value::Null))
    }

    pub async fn erc20_allowance(
        &self,
        request: Erc20AllowanceRequest,
    ) -> anyhow::Result<Erc20AllowanceResponse> {
        let rpc_url = self
            .rpc_url
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("MANTLE_RPC_URL is not configured"))?;

        validate_hex_address(&request.token_address, "token_address")?;
        validate_hex_address(&request.owner_address, "owner_address")?;
        validate_hex_address(&request.spender_address, "spender_address")?;

        let owner = Address::from_str(&request.owner_address)?;
        let spender = Address::from_str(&request.spender_address)?;
        let mut data = id("allowance(address,address)")[..4].to_vec();
        data.extend(encode(&[Token::Address(owner), Token::Address(spender)]));

        let response = self
            .rpc_call(
                rpc_url,
                "eth_call",
                json!([{
                    "to": request.token_address,
                    "data": format!("0x{}", hex_encode(&data)),
                }, "latest"]),
            )
            .await?;

        if let Some(error) = response.get("error") {
            anyhow::bail!("rpc error: {error}");
        }

        let allowance = response
            .get("result")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow::anyhow!("eth_call allowance response missing result"))?;

        Ok(Erc20AllowanceResponse {
            token_address: request.token_address,
            owner_address: request.owner_address,
            spender_address: request.spender_address,
            allowance: allowance.to_string(),
        })
    }

    async fn rpc_call(&self, rpc_url: &str, method: &str, params: Value) -> anyhow::Result<Value> {
        let response = self
            .client
            .post(rpc_url)
            .json(&json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": method,
                "params": params,
            }))
            .send()
            .await?
            .error_for_status()?
            .json::<Value>()
            .await?;

        Ok(response)
    }
}

fn validate_hex_address(value: &str, name: &str) -> anyhow::Result<()> {
    if !value.starts_with("0x") || value.len() != 42 {
        anyhow::bail!("{name} must be a 0x-prefixed address");
    }
    Ok(())
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}
