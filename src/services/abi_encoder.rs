use ethers_core::{
    abi::{encode, Token},
    types::Address,
    utils::id,
};

/// Encodes protocol-specific function calls into calldata
pub struct AbiEncoder;

impl AbiEncoder {
    /// Encode Agni exactInputSingle call
    /// SwapRouter.exactInputSingle(params)
    pub fn encode_agni_exact_input_single(
        token_in: &str,
        token_out: &str,
        fee: u32,
        recipient: &str,
        deadline: u64,
        amount_in: &str,
        amount_out_minimum: &str,
    ) -> anyhow::Result<String> {
        let token_in_addr: Address = token_in.parse()?;
        let token_out_addr: Address = token_out.parse()?;
        let recipient_addr: Address = recipient.parse()?;

        let amount_in_u256 = parse_u256(amount_in)?;
        let amount_out_min_u256 = parse_u256(amount_out_minimum)?;

        // Function signature: exactInputSingle((address,address,uint24,address,uint256,uint256,uint256,uint160))
        let selector = &id("exactInputSingle((address,address,uint24,address,uint256,uint256,uint256,uint160))")[..4];

        // Encode the tuple parameters
        let params = vec![
            Token::Address(token_in_addr),
            Token::Address(token_out_addr),
            Token::Uint(ethers_core::types::U256::from(fee)),
            Token::Address(recipient_addr),
            Token::Uint(ethers_core::types::U256::from(deadline)),
            Token::Uint(amount_in_u256),
            Token::Uint(amount_out_min_u256),
            Token::Uint(ethers_core::types::U256::zero()), // sqrtPriceLimitX96 = 0
        ];

        let encoded = encode(&params);
        let mut calldata = selector.to_vec();
        calldata.extend(encoded);

        Ok(format!("0x{}", encode_hex(&calldata)))
    }

    /// Encode Agni mint (addLiquidity) call
    pub fn encode_agni_mint(
        token0: &str,
        token1: &str,
        fee: u32,
        tick_lower: i32,
        tick_upper: i32,
        amount0_desired: &str,
        amount1_desired: &str,
        amount0_min: &str,
        amount1_min: &str,
        recipient: &str,
        deadline: u64,
    ) -> anyhow::Result<String> {
        let token0_addr: Address = token0.parse()?;
        let token1_addr: Address = token1.parse()?;
        let recipient_addr: Address = recipient.parse()?;

        let selector = &id("mint((address,address,uint24,int24,int24,uint256,uint256,uint256,uint256,address,uint256))")[..4];

        let params = vec![
            Token::Address(token0_addr),
            Token::Address(token1_addr),
            Token::Uint(ethers_core::types::U256::from(fee)),
            Token::Uint(ethers_core::types::U256::from(tick_lower as u32)),
            Token::Uint(ethers_core::types::U256::from(tick_upper as u32)),
            Token::Uint(parse_u256(amount0_desired)?),
            Token::Uint(parse_u256(amount1_desired)?),
            Token::Uint(parse_u256(amount0_min)?),
            Token::Uint(parse_u256(amount1_min)?),
            Token::Address(recipient_addr),
            Token::Uint(ethers_core::types::U256::from(deadline)),
        ];

        let encoded = encode(&params);
        let mut calldata = selector.to_vec();
        calldata.extend(encoded);

        Ok(format!("0x{}", encode_hex(&calldata)))
    }

    /// Encode Merchant Moe swapExactTokensForTokens call
    pub fn encode_moe_swap_exact_tokens_for_tokens(
        amount_in: &str,
        amount_out_min: &str,
        bin_steps: &[u32],
        versions: &[u8],
        token_path: &[&str],
        to: &str,
        deadline: u64,
    ) -> anyhow::Result<String> {
        let to_addr: Address = to.parse()?;

        // Convert token path strings to Address tokens
        let mut path_addrs = Vec::new();
        for token in token_path {
            let addr: Address = token.parse()?;
            path_addrs.push(Token::Address(addr));
        }

        // Encode bin steps as uint256[]
        let bin_steps_encoded: Vec<Token> = bin_steps
            .iter()
            .map(|step| Token::Uint(ethers_core::types::U256::from(*step)))
            .collect();

        // Encode versions as uint8[]
        let versions_encoded: Vec<Token> = versions
            .iter()
            .map(|v| Token::Uint(ethers_core::types::U256::from(*v)))
            .collect();

        let selector = &id("swapExactTokensForTokens(uint256,uint256,uint256[],uint8[],address[],address,uint256)")[..4];

        let params = vec![
            Token::Uint(parse_u256(amount_in)?),
            Token::Uint(parse_u256(amount_out_min)?),
            Token::Array(bin_steps_encoded),
            Token::Array(versions_encoded),
            Token::Array(path_addrs),
            Token::Address(to_addr),
            Token::Uint(ethers_core::types::U256::from(deadline)),
        ];

        let encoded = encode(&params);
        let mut calldata = selector.to_vec();
        calldata.extend(encoded);

        Ok(format!("0x{}", encode_hex(&calldata)))
    }

    /// Encode mETH stake call
    pub fn encode_meth_stake(amount_eth: &str) -> anyhow::Result<String> {
        let selector = &id("stake(uint256)")[..4];
        let amount = parse_u256(amount_eth)?;
        let encoded = encode(&[Token::Uint(amount)]);

        let mut calldata = selector.to_vec();
        calldata.extend(encoded);

        Ok(format!("0x{}", encode_hex(&calldata)))
    }

    /// Encode ERC20 approve call (for all protocols)
    pub fn encode_erc20_approve(spender: &str, amount: &str) -> anyhow::Result<String> {
        let spender_addr: Address = spender.parse()?;
        let selector = &id("approve(address,uint256)")[..4];
        let amount_u256 = parse_u256(amount)?;

        let encoded = encode(&[Token::Address(spender_addr), Token::Uint(amount_u256)]);
        let mut calldata = selector.to_vec();
        calldata.extend(encoded);

        Ok(format!("0x{}", encode_hex(&calldata)))
    }
}

fn parse_u256(value: &str) -> anyhow::Result<ethers_core::types::U256> {
    use std::str::FromStr;
    if value.starts_with("0x") {
        Ok(ethers_core::types::U256::from_str_radix(&value[2..], 16)?)
    } else {
        Ok(ethers_core::types::U256::from_str(value)?)
    }
}

fn encode_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}
