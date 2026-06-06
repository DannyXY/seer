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

        // TODO: Implement actual RPC call to QuoterV2
        // For now, this is a placeholder that returns a mock quote
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

        // TODO: Implement actual RPC call to LBQuoter.findBestPathFromAmountIn
        // For now, return a mock quote with single hop
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
        use std::str::FromStr;
        use ethers_core::types::U256;

        let amount = U256::from_str_radix(estimated_out.trim_start_matches("0x"), 16)?;
        let slippage = U256::from(slippage_basis_points);
        let divisor = U256::from(10000);
        let min_amount = amount * (divisor - slippage) / divisor;

        Ok(format!("0x{:x}", min_amount))
    }
}
