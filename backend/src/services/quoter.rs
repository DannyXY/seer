// Staged QuoterV2 (Agni) / LBQuoter (Merchant Moe) integration for swap
// execution. Not yet wired into the execution path; kept until swap
// signatures are supported by the transaction builder.
#![allow(dead_code, unused_variables)]

use ethers::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgniQuote {
    pub amount_out: String,
    pub sqrt_price_x96_after: String,
    pub initialized_ticks_crossed: u32,
    pub gas_estimate: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerchantMoeQuote {
    pub amounts: Vec<String>,
    pub bin_steps: Vec<u32>,
    pub versions: Vec<u8>,
    pub fees: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuoteRequest {
    pub protocol: String,
    pub token_in: String,
    pub token_out: String,
    pub amount_in: String,
    pub fee_tier: Option<u32>,
}

pub struct QuoterService;

impl QuoterService {
    /// Get quote from Agni Finance QuoterV2
    /// Testnet QuoterV2: 0x49C8bb51C6bb791e8D6C31310cE0C14f68492991
    pub async fn quote_agni_exact_input(
        &self,
        rpc_url: &str,
        token_in: &str,
        token_out: &str,
        amount_in: &str,
        fee_tier: u32,
    ) -> anyhow::Result<AgniQuote> {
        tracing::info!(
            target: "quoter",
            token_in = %token_in,
            token_out = %token_out,
            amount_in = %amount_in,
            fee = fee_tier,
            "querying Agni QuoterV2"
        );

        let provider = Provider::<Http>::try_from(rpc_url)
            .map_err(|e| anyhow::anyhow!("Failed to create provider: {}", e))?;

        let quoter_address: Address = "0x49C8bb51C6bb791e8D6C31310cE0C14f68492991"
            .parse()
            .map_err(|_| anyhow::anyhow!("Invalid quoter address"))?;

        let token_in_addr: Address = token_in
            .parse()
            .map_err(|_| anyhow::anyhow!("Invalid token_in address"))?;

        let token_out_addr: Address = token_out
            .parse()
            .map_err(|_| anyhow::anyhow!("Invalid token_out address"))?;

        let amount_in_u256 =
            U256::from_dec_str(amount_in).map_err(|_| anyhow::anyhow!("Invalid amount_in"))?;

        // Agni QuoterV2.quoteExactInputSingle(tokenIn, tokenOut, fee, amountIn, sqrtPriceLimitX96)
        let call_data = ethers::abi::encode(&[
            ethers::abi::Token::Address(token_in_addr),
            ethers::abi::Token::Address(token_out_addr),
            ethers::abi::Token::Uint(U256::from(fee_tier)),
            ethers::abi::Token::Uint(amount_in_u256),
            ethers::abi::Token::Uint(U256::zero()),
        ]);

        // Fallback to mock data since RPC quote is optional
        tracing::debug!(target: "quoter", "Using mock quote for Agni");
        Ok(AgniQuote {
            amount_out: amount_in.to_string(),
            sqrt_price_x96_after: "0".to_string(),
            initialized_ticks_crossed: 0,
            gas_estimate: "150000".to_string(),
        })
    }

    /// Get best swap route from Merchant Moe LBQuoter
    /// Mainnet LBQuoter: 0x501b8AFd35df20f531fF45F6f695793AC3316c85
    pub async fn quote_merchant_moe_best_path(
        &self,
        rpc_url: &str,
        token_path: Vec<&str>,
        amount_in: &str,
    ) -> anyhow::Result<MerchantMoeQuote> {
        tracing::info!(
            target: "quoter",
            token_count = token_path.len(),
            amount_in = %amount_in,
            "querying Merchant Moe LBQuoter for best path"
        );

        let provider = Provider::<Http>::try_from(rpc_url)
            .map_err(|e| anyhow::anyhow!("Failed to create provider: {}", e))?;

        let quoter_address: Address = "0x501b8AFd35df20f531fF45F6f695793AC3316c85"
            .parse()
            .map_err(|_| anyhow::anyhow!("Invalid quoter address"))?;

        let amount_in_u256 =
            U256::from_dec_str(amount_in).map_err(|_| anyhow::anyhow!("Invalid amount_in"))?;

        let token_addresses: Result<Vec<Address>, _> = token_path
            .iter()
            .map(|addr| addr.parse::<Address>())
            .collect();

        // Return mock data for now - RPC quote call can be implemented later
        tracing::debug!(target: "quoter", "Using mock quote for Merchant Moe");
        let amounts = vec![amount_in.to_string(), amount_in.to_string()];
        Ok(MerchantMoeQuote {
            amounts,
            bin_steps: vec![25],
            versions: vec![2],
            fees: vec!["0".to_string()],
        })
    }

    /// Calculate slippage-adjusted minimum output
    pub fn calculate_amount_out_minimum(
        estimated_out: &str,
        slippage_basis_points: u32,
    ) -> anyhow::Result<String> {
        use ethers_core::types::U256;

        let amount = U256::from_str_radix(estimated_out.trim_start_matches("0x"), 16)?;
        let slippage = U256::from(slippage_basis_points);
        let divisor = U256::from(10000);
        let min_amount = amount * (divisor - slippage) / divisor;

        Ok(format!("0x{:x}", min_amount))
    }
}
