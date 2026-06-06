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
    SimulateTransactionRequest, SimulateTransactionResponse, UserOperationReceiptRequest,
};

pub struct ContractService {
    pub rpc_url: Option<String>,
    pub private_key: Option<String>,
    pub arena_points_address: Option<String>,
    pub prediction_registry_address: Option<String>,
    pub identity_sbt_address: Option<String>,
    pub intent_registry_address: Option<String>,
    pub aa_provider_stack: String,
    pub aa_bundler_url: Option<String>,
    pub aa_entry_point_address: Option<String>,
    pub aa_paymaster_url: Option<String>,
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
            aa_provider_stack: settings.aa_provider_stack,
            aa_bundler_url: settings.aa_bundler_url,
            aa_entry_point_address: settings.aa_entry_point_address,
            aa_paymaster_url: settings.aa_paymaster_url,
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

    pub fn entry_point_configured(&self) -> bool {
        self.aa_entry_point_address.is_some()
    }

    pub fn paymaster_configured(&self) -> bool {
        self.aa_paymaster_url.is_some()
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
        if let Some(configured_entry_point) = &self.aa_entry_point_address {
            if !request
                .entry_point
                .eq_ignore_ascii_case(configured_entry_point)
            {
                anyhow::bail!("entry_point does not match configured AA_ENTRY_POINT_ADDRESS");
            }
        }
        validate_user_operation_shape(&request.user_operation)?;
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

    pub async fn simulate_transaction(
        &self,
        request: SimulateTransactionRequest,
    ) -> anyhow::Result<SimulateTransactionResponse> {
        let rpc_url = self
            .rpc_url
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("MANTLE_RPC_URL is not configured"))?;

        if let Some(from) = &request.from {
            validate_hex_address(from, "from")?;
        }
        validate_hex_address(&request.to, "to")?;
        if let Some(data) = &request.data {
            validate_hex_data(data, "data")?;
        }
        let value = request
            .value
            .as_deref()
            .map(normalize_rpc_quantity)
            .transpose()?;

        let mut call = serde_json::Map::new();
        if let Some(from) = request.from {
            call.insert("from".to_string(), json!(from));
        }
        call.insert("to".to_string(), json!(request.to));
        if let Some(value) = value {
            call.insert("value".to_string(), json!(value));
        }
        if let Some(data) = request.data {
            call.insert("data".to_string(), json!(data));
        }

        let block_tag = "latest".to_string();
        let response = self
            .rpc_call(
                rpc_url,
                "eth_call",
                json!([Value::Object(call), block_tag.clone()]),
            )
            .await?;

        if let Some(error) = response.get("error") {
            return Ok(SimulateTransactionResponse {
                success: false,
                block_tag,
                return_data: None,
                error: Some(error.to_string()),
            });
        }

        let return_data = response
            .get("result")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow::anyhow!("eth_call simulation response missing result"))?;

        Ok(SimulateTransactionResponse {
            success: true,
            block_tag,
            return_data: Some(return_data.to_string()),
            error: None,
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
    let Some(hex) = value.strip_prefix("0x") else {
        anyhow::bail!("{name} must be a 0x-prefixed address");
    };
    if hex.len() != 40 || !hex.chars().all(|character| character.is_ascii_hexdigit()) {
        anyhow::bail!("{name} must be a 0x-prefixed address");
    }
    Ok(())
}

fn validate_required_hex_field(
    object: &serde_json::Map<String, Value>,
    key: &str,
) -> anyhow::Result<()> {
    let value = object
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("user_operation.{key} is required"))?;
    validate_hex_data(value, &format!("user_operation.{key}"))
}

fn validate_required_quantity_field(
    object: &serde_json::Map<String, Value>,
    key: &str,
) -> anyhow::Result<()> {
    let value = object
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("user_operation.{key} is required"))?;
    normalize_rpc_quantity(value)?;
    Ok(())
}

fn validate_user_operation_shape(
    user_operation: &serde_json::Map<String, Value>,
) -> anyhow::Result<()> {
    let sender = user_operation
        .get("sender")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("user_operation.sender is required"))?;
    validate_hex_address(sender, "user_operation.sender")?;

    for key in [
        "nonce",
        "callGasLimit",
        "verificationGasLimit",
        "preVerificationGas",
        "maxFeePerGas",
        "maxPriorityFeePerGas",
    ] {
        validate_required_quantity_field(user_operation, key)?;
    }

    validate_required_hex_field(user_operation, "callData")?;
    validate_required_hex_field(user_operation, "signature")?;

    for key in [
        "initCode",
        "factory",
        "factoryData",
        "paymasterAndData",
        "paymaster",
        "paymasterData",
    ] {
        if let Some(value) = user_operation.get(key).and_then(Value::as_str) {
            validate_hex_data(value, &format!("user_operation.{key}"))?;
        }
    }

    Ok(())
}

fn validate_hex_data(value: &str, name: &str) -> anyhow::Result<()> {
    if !value.starts_with("0x") || value.len() % 2 != 0 {
        anyhow::bail!("{name} must be even-length 0x-prefixed hex data");
    }
    if !value[2..].chars().all(|ch| ch.is_ascii_hexdigit()) {
        anyhow::bail!("{name} contains non-hex characters");
    }
    Ok(())
}

fn normalize_rpc_quantity(value: &str) -> anyhow::Result<String> {
    let trimmed = value.trim();
    if trimmed.starts_with("0x") {
        if trimmed == "0x" {
            anyhow::bail!("value must not be empty hex");
        }
        return Ok(trimmed.to_string());
    }
    let parsed = trimmed.parse::<u128>()?;
    Ok(format!("0x{parsed:x}"))
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

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{
        normalize_rpc_quantity, validate_hex_address, validate_hex_data,
        validate_user_operation_shape,
    };

    #[test]
    fn normalizes_decimal_rpc_quantity_to_hex() {
        assert_eq!(normalize_rpc_quantity("0").unwrap(), "0x0");
        assert_eq!(normalize_rpc_quantity("25").unwrap(), "0x19");
        assert_eq!(normalize_rpc_quantity("0x2a").unwrap(), "0x2a");
    }

    #[test]
    fn rejects_invalid_hex_data_for_simulation() {
        assert!(validate_hex_data("095ea7b3", "data").is_err());
        assert!(validate_hex_data("0x123", "data").is_err());
        assert!(validate_hex_data("0xzz", "data").is_err());
        assert!(validate_hex_data("0x095ea7b3", "data").is_ok());
    }

    #[test]
    fn rejects_non_hex_addresses() {
        assert!(
            validate_hex_address("0xnotanaddress000000000000000000000000000", "sender").is_err()
        );
        assert!(
            validate_hex_address("0x00000000000000000000000000000000000000aa", "sender").is_ok()
        );
    }

    #[test]
    fn validates_provider_built_user_operation_shape() {
        let user_operation = json!({
            "sender": "0x00000000000000000000000000000000000000aa",
            "nonce": "0x0",
            "callData": "0x1234",
            "callGasLimit": "0x5208",
            "verificationGasLimit": "0x10000",
            "preVerificationGas": "0x5208",
            "maxFeePerGas": "0x1",
            "maxPriorityFeePerGas": "0x1",
            "signature": "0xabcd"
        })
        .as_object()
        .unwrap()
        .clone();

        assert!(validate_user_operation_shape(&user_operation).is_ok());
    }

    #[test]
    fn rejects_user_operation_missing_signature() {
        let user_operation = json!({
            "sender": "0x00000000000000000000000000000000000000aa",
            "nonce": "0x0",
            "callData": "0x1234",
            "callGasLimit": "0x5208",
            "verificationGasLimit": "0x10000",
            "preVerificationGas": "0x5208",
            "maxFeePerGas": "0x1",
            "maxPriorityFeePerGas": "0x1"
        })
        .as_object()
        .unwrap()
        .clone();

        assert!(validate_user_operation_shape(&user_operation).is_err());
    }
}
