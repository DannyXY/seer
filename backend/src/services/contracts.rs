use ethers_core::{
    abi::{encode, Token},
    types::{transaction::eip2718::TypedTransaction, Address, Bytes, TransactionRequest, U256},
    utils::id,
};
use ethers::signers::{LocalWallet, Signer};
use serde_json::{json, Value};
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::config::Settings;
use crate::models::execution::{
    Erc20AllowanceRequest, Erc20AllowanceResponse, SendRawTransactionRequest,
    SendRawTransactionResponse, SendUserOperationRequest, SendUserOperationResponse,
    SimulateTransactionRequest, SimulateTransactionResponse, UserOperationReceiptRequest,
};

#[derive(Clone)]
pub struct ContractService {
    pub rpc_url: Option<String>,
    pub private_key: Option<String>,
    pub chain_id: u64,
    pub arena_points_address: Option<String>,
    pub prediction_registry_address: Option<String>,
    pub identity_sbt_address: Option<String>,
    pub intent_registry_address: Option<String>,
    pub aa_provider_stack: String,
    pub aa_bundler_url: Option<String>,
    pub aa_entry_point_address: Option<String>,
    pub aa_paymaster_url: Option<String>,
    client: reqwest::Client,
    /// Serialises all backend-signed transactions so each one gets a fresh,
    /// sequential nonce — prevents "nonce too low" errors under concurrent load.
    nonce_lock: Arc<Mutex<()>>,
}

impl ContractService {
    pub fn new(settings: Settings) -> Self {
        Self {
            rpc_url: settings.mantle_rpc_url,
            private_key: settings.backend_signer_private_key,
            chain_id: settings.mantle_chain_id,
            arena_points_address: settings.arena_points_address,
            prediction_registry_address: settings.prediction_registry_address,
            identity_sbt_address: settings.identity_sbt_address,
            intent_registry_address: settings.intent_registry_address,
            aa_provider_stack: settings.aa_provider_stack,
            aa_bundler_url: settings.aa_bundler_url,
            aa_entry_point_address: settings.aa_entry_point_address,
            aa_paymaster_url: settings.aa_paymaster_url,
            client: reqwest::Client::new(),
            nonce_lock: Arc::new(Mutex::new(())),
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

    // ── Intent Registry helpers ──────────────────────────────────────────────

    /// Returns (to, calldata) for user to sign `SeerIntentRegistry.registerIntent(bytes32, string)`.
    /// `intent_hash_hex` must be a 32-byte hex string (with or without 0x prefix).
    pub fn register_intent_calldata(&self, intent_hash_hex: &str, metadata_uri: &str) -> Option<(String, String)> {
        let to = self.intent_registry_address.clone()?;
        let bytes = ethers_core::utils::hex::decode(intent_hash_hex.trim_start_matches("0x")).ok()?;
        if bytes.len() != 32 { return None; }
        let mut fixed = [0u8; 32];
        fixed.copy_from_slice(&bytes);
        let mut data = id("registerIntent(bytes32,string)")[..4].to_vec();
        data.extend(encode(&[
            Token::FixedBytes(fixed.to_vec()),
            Token::String(metadata_uri.to_string()),
        ]));
        Some((to, format!("0x{}", hex_encode(&data))))
    }

    /// Fetch all IntentRegistered events for a wallet from SeerIntentRegistry.
    /// Returns (onchain_intent_id, intent_hash_hex, metadata_uri) tuples, oldest first.
    /// The updated contract emits metadataURI in the event so we can reconstruct
    /// intents from chain alone without a separate eth_call.
    pub async fn get_registered_intents_onchain(
        &self,
        wallet: &str,
    ) -> anyhow::Result<Vec<(u64, String, String)>> {
        let rpc_url = self
            .rpc_url
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("MANTLE_RPC_URL not configured"))?;
        let to = self
            .intent_registry_address
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("INTENT_REGISTRY_ADDRESS not configured"))?;

        // IntentRegistered(uint256 indexed intentId, address indexed user, bytes32 intentHash, string metadataURI)
        let topic0 = format!(
            "0x{}",
            hex_encode(&ethers_core::utils::keccak256(
                b"IntentRegistered(uint256,address,bytes32,string)"
            ))
        );
        let wallet_clean = wallet.trim_start_matches("0x").to_lowercase();
        let topic2 = format!("0x{:0>64}", wallet_clean);

        let response = self
            .rpc_call(
                rpc_url,
                "eth_getLogs",
                serde_json::json!([{
                    "address": to,
                    "topics": [topic0, null, topic2],
                    "fromBlock": "0x0",
                    "toBlock": "latest",
                }]),
            )
            .await?;

        let empty = vec![];
        let logs = response
            .get("result")
            .and_then(Value::as_array)
            .unwrap_or(&empty);

        let results: Vec<(u64, String, String)> = logs
            .iter()
            .filter_map(|log| {
                let topics = log.get("topics").and_then(Value::as_array)?;
                let id_hex = topics.get(1).and_then(Value::as_str)?;
                let id_bytes =
                    ethers_core::utils::hex::decode(id_hex.trim_start_matches("0x")).ok()?;
                let intent_id = U256::from_big_endian(&id_bytes)
                    .min(U256::from(u64::MAX))
                    .as_u64();

                // data = ABI-encoded (bytes32 intentHash, string metadataURI)
                // layout: [0..32] hash | [32..64] string_offset | [64..96] string_len | [96..] string_bytes
                let data_hex = log.get("data").and_then(Value::as_str).unwrap_or("0x");
                let data =
                    ethers_core::utils::hex::decode(data_hex.trim_start_matches("0x")).ok()?;

                // intentHash: first 32 bytes
                let hash_hex = if data.len() >= 32 {
                    hex_encode(&data[0..32])
                } else {
                    return None;
                };

                // metadataURI: decode dynamic string starting at byte 32
                let metadata_uri = decode_abi_string(&data, 32).unwrap_or_default();

                Some((intent_id, hash_hex, metadata_uri))
            })
            .collect();

        Ok(results)
    }

    /// Fetch all PredictionEntered events for a wallet from SeerPredictionRegistry.
    /// Returns (prediction_id, position, points_amount) tuples.
    pub async fn get_entries_for_user_onchain(
        &self,
        wallet: &str,
    ) -> anyhow::Result<Vec<(u64, u8, u64)>> {
        let rpc_url = self
            .rpc_url
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("MANTLE_RPC_URL not configured"))?;
        let to = self
            .prediction_registry_address
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("PREDICTION_REGISTRY_ADDRESS not configured"))?;

        // PredictionEntered(uint256 indexed predictionId, address indexed user, uint8 position, uint256 pointsAmount)
        let topic0 = format!(
            "0x{}",
            hex_encode(&ethers_core::utils::keccak256(
                b"PredictionEntered(uint256,address,uint8,uint256)"
            ))
        );
        let wallet_clean = wallet.trim_start_matches("0x").to_lowercase();
        let topic2 = format!("0x{:0>64}", wallet_clean);

        let response = self
            .rpc_call(
                rpc_url,
                "eth_getLogs",
                serde_json::json!([{
                    "address": to,
                    "topics": [topic0, null, topic2],
                    "fromBlock": "0x0",
                    "toBlock": "latest",
                }]),
            )
            .await?;

        let empty = vec![];
        let logs = response
            .get("result")
            .and_then(Value::as_array)
            .unwrap_or(&empty);

        let results: Vec<(u64, u8, u64)> = logs
            .iter()
            .filter_map(|log| {
                let topics = log.get("topics").and_then(Value::as_array)?;
                let pred_id_hex = topics.get(1).and_then(Value::as_str)?;
                let pred_id_bytes =
                    ethers_core::utils::hex::decode(pred_id_hex.trim_start_matches("0x")).ok()?;
                let prediction_id = U256::from_big_endian(&pred_id_bytes)
                    .min(U256::from(u64::MAX))
                    .as_u64();

                // data = ABI-encoded (uint8 position, uint256 pointsAmount)
                // layout: [0..32] position (padded) | [32..64] pointsAmount
                let data_hex = log.get("data").and_then(Value::as_str).unwrap_or("0x");
                let data =
                    ethers_core::utils::hex::decode(data_hex.trim_start_matches("0x")).ok()?;
                if data.len() < 64 {
                    return None;
                }
                let position = data[31]; // last byte of first 32-byte slot
                let points =
                    U256::from_big_endian(&data[32..64]).min(U256::from(u64::MAX)).as_u64();

                Some((prediction_id, position, points))
            })
            .collect();

        Ok(results)
    }

    /// Build calldata to call `pauseIntent(uint256)` on SeerIntentRegistry.
    pub fn pause_intent_calldata(&self, onchain_intent_id: u64) -> Option<(String, String)> {
        let to = self.intent_registry_address.as_ref()?;
        let mut data = id("pauseIntent(uint256)")[..4].to_vec();
        data.extend(encode(&[Token::Uint(U256::from(onchain_intent_id))]));
        Some((to.clone(), format!("0x{}", hex_encode(&data))))
    }

    /// Build calldata to call `resumeIntent(uint256)` on SeerIntentRegistry.
    pub fn resume_intent_calldata(&self, onchain_intent_id: u64) -> Option<(String, String)> {
        let to = self.intent_registry_address.as_ref()?;
        let mut data = id("resumeIntent(uint256)")[..4].to_vec();
        data.extend(encode(&[Token::Uint(U256::from(onchain_intent_id))]));
        Some((to.clone(), format!("0x{}", hex_encode(&data))))
    }

    /// Build calldata to call `cancelIntent(uint256)` on SeerIntentRegistry.
    pub fn cancel_intent_calldata(&self, onchain_intent_id: u64) -> Option<(String, String)> {
        let to = self.intent_registry_address.as_ref()?;
        let mut data = id("cancelIntent(uint256)")[..4].to_vec();
        data.extend(encode(&[Token::Uint(U256::from(onchain_intent_id))]));
        Some((to.clone(), format!("0x{}", hex_encode(&data))))
    }

    // ── Identity SBT helpers ─────────────────────────────────────────────────

    /// Read `SeerIdentitySBT.tokenOfOwner(wallet)` — returns 0 if not minted.
    pub async fn identity_token_of_owner(&self, wallet: &str) -> anyhow::Result<u64> {
        let rpc_url = self.rpc_url.as_ref().ok_or_else(|| anyhow::anyhow!("MANTLE_RPC_URL not configured"))?;
        let to = self.identity_sbt_address.as_ref().ok_or_else(|| anyhow::anyhow!("IDENTITY_SBT_ADDRESS not configured"))?;
        let addr = Address::from_str(wallet).map_err(|e| anyhow::anyhow!("bad wallet address: {e}"))?;
        let mut data = id("tokenOfOwner(address)")[..4].to_vec();
        data.extend(encode(&[Token::Address(addr)]));
        let result = self.eth_call(rpc_url, to, &data).await?;
        Ok(u256_from_return_data(&result).min(U256::from(u64::MAX)).as_u64())
    }

    /// Backend-signed: call `SeerIdentitySBT.mintIdentity(user, uri)`.
    /// Returns the minted token ID.
    pub async fn mint_identity_on_chain(&self, user: &str, uri: &str) -> anyhow::Result<u64> {
        let to = self.identity_sbt_address.as_ref()
            .ok_or_else(|| anyhow::anyhow!("IDENTITY_SBT_ADDRESS not configured"))?;
        let to_addr = Address::from_str(to)?;
        let user_addr = Address::from_str(user)?;

        let mut calldata = id("mintIdentity(address,string)")[..4].to_vec();
        calldata.extend(encode(&[
            Token::Address(user_addr),
            Token::String(uri.to_string()),
        ]));

        let tx_hash = self.sign_and_send_tx(to_addr, calldata).await?;
        let token_id = self.wait_for_identity_minted(&tx_hash).await?;
        Ok(token_id)
    }

    /// Poll for receipt and extract IdentityMinted(user, tokenId, uri) event.
    async fn wait_for_identity_minted(&self, tx_hash: &str) -> anyhow::Result<u64> {
        let rpc_url = self.rpc_url.as_ref().ok_or_else(|| anyhow::anyhow!("no rpc"))?;
        let event_sig = "IdentityMinted(address,uint256,string)";
        let topic0 = format!("0x{}", hex_encode(&ethers_core::utils::keccak256(event_sig.as_bytes())));

        for _ in 0..30 {
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            let response = self.rpc_call(rpc_url, "eth_getTransactionReceipt", json!([tx_hash])).await?;
            let Some(receipt) = response.get("result").filter(|v| !v.is_null()) else {
                continue;
            };
            if receipt.get("status").and_then(Value::as_str) == Some("0x0") {
                anyhow::bail!("mintIdentity transaction reverted");
            }
            let empty_logs = vec![];
            let logs = receipt.get("logs").and_then(Value::as_array).unwrap_or(&empty_logs);
            for log in logs {
                let empty_topics = vec![];
                let topics = log.get("topics").and_then(Value::as_array).unwrap_or(&empty_topics);
                if topics.first().and_then(Value::as_str) == Some(&topic0) {
                    if let Some(id_hex) = topics.get(2).and_then(Value::as_str) {
                        let id_bytes = ethers_core::utils::hex::decode(id_hex.trim_start_matches("0x")).unwrap_or_default();
                        let id_val = U256::from_big_endian(&id_bytes);
                        return Ok(id_val.min(U256::from(u64::MAX)).as_u64());
                    }
                }
            }
            break;
        }
        anyhow::bail!("timed out waiting for IdentityMinted event in tx {tx_hash}")
    }

    // ── Arena contract helpers ────────────────────────────────────────────────

    /// Returns (to, calldata) for user to sign `SeerArenaPoints.claimStarterPoints()`.
    pub fn claim_starter_points_calldata(&self) -> Option<(String, String)> {
        let to = self.arena_points_address.clone()?;
        let calldata = format!("0x{}", hex_encode(&id("claimStarterPoints()")[..4]));
        Some((to, calldata))
    }

    /// Returns (to, calldata) for user to sign `SeerPredictionRegistry.enterPrediction()`.
    pub fn enter_prediction_calldata(&self, onchain_id: u64, position: u8, amount: u64) -> Option<(String, String)> {
        let to = self.prediction_registry_address.clone()?;
        let mut data = id("enterPrediction(uint256,uint8,uint256)")[..4].to_vec();
        data.extend(encode(&[
            Token::Uint(U256::from(onchain_id)),
            Token::Uint(U256::from(position)),
            Token::Uint(U256::from(amount)),
        ]));
        Some((to, format!("0x{}", hex_encode(&data))))
    }

    /// Read `SeerArenaPoints.getAvailablePoints(wallet)` via eth_call.
    pub async fn read_available_points(&self, wallet: &str) -> anyhow::Result<u64> {
        let rpc_url = self.rpc_url.as_ref().ok_or_else(|| anyhow::anyhow!("MANTLE_RPC_URL not configured"))?;
        let to = self.arena_points_address.as_ref().ok_or_else(|| anyhow::anyhow!("SEER_ARENA_POINTS_ADDRESS not configured"))?;
        let addr = Address::from_str(wallet).map_err(|e| anyhow::anyhow!("bad wallet address: {e}"))?;
        let mut data = id("getAvailablePoints(address)")[..4].to_vec();
        data.extend(encode(&[Token::Address(addr)]));
        let result = self.eth_call(rpc_url, to, &data).await?;
        Ok(u256_from_return_data(&result).min(U256::from(u64::MAX)).as_u64())
    }

    /// Check `SeerArenaPoints.claimedStarterPoints(wallet)` via eth_call.
    pub async fn has_claimed_starter_points(&self, wallet: &str) -> anyhow::Result<bool> {
        let rpc_url = self.rpc_url.as_ref().ok_or_else(|| anyhow::anyhow!("MANTLE_RPC_URL not configured"))?;
        let to = self.arena_points_address.as_ref().ok_or_else(|| anyhow::anyhow!("SEER_ARENA_POINTS_ADDRESS not configured"))?;
        let addr = Address::from_str(wallet).map_err(|e| anyhow::anyhow!("bad wallet address: {e}"))?;
        let mut data = id("claimedStarterPoints(address)")[..4].to_vec();
        data.extend(encode(&[Token::Address(addr)]));
        let result = self.eth_call(rpc_url, to, &data).await?;
        let bytes = ethers_core::utils::hex::decode(result.trim_start_matches("0x")).unwrap_or_default();
        Ok(bytes.last().copied().unwrap_or(0) != 0)
    }

    /// Fetch the native MNT balance for `wallet` via `eth_getBalance` on Mantle RPC.
    /// Returns (raw_wei_as_string, human_readable_mnt, usd_value_placeholder).
    pub async fn get_native_balance(&self, wallet: &str) -> anyhow::Result<(String, f64)> {
        let rpc_url = self
            .rpc_url
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("MANTLE_RPC_URL not configured"))?;
        let response = self
            .rpc_call(rpc_url, "eth_getBalance", serde_json::json!([wallet, "latest"]))
            .await?;
        let hex = response
            .get("result")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("eth_getBalance missing result"))?;
        let wei = u128::from_str_radix(hex.trim_start_matches("0x"), 16)
            .unwrap_or(0);
        // MNT has 18 decimals
        let mnt = wei as f64 / 1e18;
        Ok((hex.to_string(), mnt))
    }

    /// Backend-signed: call `SeerPredictionRegistry.createPrediction()`, return on-chain prediction ID.
    pub async fn create_prediction_on_chain(
        &self,
        claim: &str,
        data_key: [u8; 32],
        target_value: u64,
        expiry_unix: u64,
        comparison_op: u8,  // 0 = Gte, 1 = Lte
        seer_position: u8,  // 0 = BackSeer, 1 = ChallengeSeer
    ) -> anyhow::Result<u64> {
        let to = self.prediction_registry_address.as_ref()
            .ok_or_else(|| anyhow::anyhow!("SEER_PREDICTION_REGISTRY_ADDRESS not configured"))?;
        let to_addr = Address::from_str(to)?;

        let mut calldata = id("createPrediction(string,bytes32,uint256,uint256,uint8,uint8)")[..4].to_vec();
        calldata.extend(encode(&[
            Token::String(claim.to_string()),
            Token::FixedBytes(data_key.to_vec()),
            Token::Uint(U256::from(target_value)),
            Token::Uint(U256::from(expiry_unix)),
            Token::Uint(U256::from(comparison_op)),
            Token::Uint(U256::from(seer_position)),
        ]));

        let tx_hash = self.sign_and_send_tx(to_addr, calldata).await?;
        let prediction_id = self.wait_for_prediction_id(&tx_hash).await?;
        Ok(prediction_id)
    }

    /// Sign and send a backend-authorized transaction. Returns tx hash.
    async fn sign_and_send_tx(&self, to: Address, calldata: Vec<u8>) -> anyhow::Result<String> {
        let rpc_url = self.rpc_url.as_ref().ok_or_else(|| anyhow::anyhow!("MANTLE_RPC_URL not configured"))?;
        let private_key = self.private_key.as_ref().ok_or_else(|| anyhow::anyhow!("BACKEND_SIGNER_PRIVATE_KEY not configured"))?;

        let wallet: LocalWallet = private_key.parse::<LocalWallet>()?.with_chain_id(self.chain_id);
        let from = wallet.address();

        // Hold the lock for the entire sign+send cycle so concurrent calls
        // don't fetch the same pending nonce and collide.
        let _guard = self.nonce_lock.lock().await;

        let nonce = self.get_nonce(rpc_url, &format!("{from:?}")).await?;
        let gas_price = self.get_gas_price(rpc_url).await?;

        let tx = TransactionRequest::new()
            .to(to)
            .from(from)
            .data(Bytes::from(calldata))
            .gas(500_000u64)
            .gas_price(gas_price)
            .nonce(nonce)
            .chain_id(self.chain_id);

        let typed: TypedTransaction = tx.into();
        let sig = wallet.sign_transaction(&typed).await?;
        let rlp_bytes = typed.rlp_signed(&sig);
        let hex = format!("0x{}", hex_encode(&rlp_bytes));

        let response = self.rpc_call(rpc_url, "eth_sendRawTransaction", json!([hex])).await?;
        if let Some(err) = response.get("error") {
            anyhow::bail!("rpc error sending tx: {err}");
        }
        let tx_hash = response.get("result").and_then(Value::as_str)
            .ok_or_else(|| anyhow::anyhow!("eth_sendRawTransaction missing result"))?;
        Ok(tx_hash.to_string())
    }

    /// Poll for a receipt and extract the PredictionCreated event's predictionId.
    async fn wait_for_prediction_id(&self, tx_hash: &str) -> anyhow::Result<u64> {
        let rpc_url = self.rpc_url.as_ref().ok_or_else(|| anyhow::anyhow!("no rpc"))?;
        // PredictionCreated(uint256 indexed predictionId, bytes32 dataKey, uint256 expiryTime)
        let event_sig = "PredictionCreated(uint256,bytes32,uint256)";
        let topic0 = format!("0x{}", hex_encode(&ethers_core::utils::keccak256(event_sig.as_bytes())));

        for _ in 0..30 {
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            let response = self.rpc_call(rpc_url, "eth_getTransactionReceipt", json!([tx_hash])).await?;
            let Some(receipt) = response.get("result").filter(|v| !v.is_null()) else {
                continue;
            };
            let empty_logs = vec![];
            let logs = receipt.get("logs").and_then(Value::as_array).unwrap_or(&empty_logs);
            for log in logs {
                let empty_topics = vec![];
                let topics = log.get("topics").and_then(Value::as_array).unwrap_or(&empty_topics);
                if topics.first().and_then(Value::as_str) == Some(&topic0) {
                    if let Some(id_hex) = topics.get(1).and_then(Value::as_str) {
                        let id_bytes = ethers_core::utils::hex::decode(id_hex.trim_start_matches("0x")).unwrap_or_default();
                        let id_val = U256::from_big_endian(&id_bytes);
                        return Ok(id_val.min(U256::from(u64::MAX)).as_u64());
                    }
                }
            }
            // Receipt found but no matching log — tx reverted or wrong contract
            if receipt.get("status").and_then(Value::as_str) == Some("0x0") {
                anyhow::bail!("createPrediction transaction reverted");
            }
            break;
        }
        anyhow::bail!("timed out waiting for PredictionCreated event in tx {tx_hash}")
    }

    async fn eth_call(&self, rpc_url: &str, to: &str, calldata: &[u8]) -> anyhow::Result<String> {
        let response = self.rpc_call(rpc_url, "eth_call", json!([{
            "to": to,
            "data": format!("0x{}", hex_encode(calldata)),
        }, "latest"])).await?;
        if let Some(err) = response.get("error") {
            anyhow::bail!("eth_call error: {err}");
        }
        Ok(response.get("result").and_then(Value::as_str).unwrap_or("0x").to_string())
    }

    async fn get_nonce(&self, rpc_url: &str, address: &str) -> anyhow::Result<U256> {
        let response = self.rpc_call(rpc_url, "eth_getTransactionCount", json!([address, "pending"])).await?;
        let hex = response.get("result").and_then(Value::as_str).unwrap_or("0x0");
        Ok(U256::from_str_radix(hex.trim_start_matches("0x"), 16)?)
    }

    async fn get_gas_price(&self, rpc_url: &str) -> anyhow::Result<U256> {
        let response = self.rpc_call(rpc_url, "eth_gasPrice", json!([])).await?;
        let hex = response.get("result").and_then(Value::as_str).unwrap_or("0x3B9ACA00");
        Ok(U256::from_str_radix(hex.trim_start_matches("0x"), 16)?)
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

fn u256_from_return_data(hex: &str) -> U256 {
    let bytes = ethers_core::utils::hex::decode(hex.trim_start_matches("0x")).unwrap_or_default();
    if bytes.len() < 32 { return U256::zero(); }
    U256::from_big_endian(&bytes[bytes.len() - 32..])
}

/// Decode a Solidity ABI-encoded `string` from a byte slice.
/// `offset_slot` is the byte index of the 32-byte word that holds the
/// offset to the string's length prefix (typically 32 for the second slot).
fn decode_abi_string(data: &[u8], offset_slot: usize) -> Option<String> {
    if data.len() < offset_slot + 32 { return None; }
    // Read the offset value (points to where the length word lives)
    let offset = U256::from_big_endian(&data[offset_slot..offset_slot + 32])
        .min(U256::from(usize::MAX))
        .as_usize();
    if data.len() < offset + 32 { return None; }
    let len = U256::from_big_endian(&data[offset..offset + 32])
        .min(U256::from(usize::MAX))
        .as_usize();
    let start = offset + 32;
    if data.len() < start + len { return None; }
    String::from_utf8(data[start..start + len].to_vec()).ok()
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
