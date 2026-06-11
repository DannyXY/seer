use axum::{
    extract::{Path, State},
    Json,
};
use ethers_core::types::U256;
use serde_json::{json, Value};

use crate::{
    errors::ApiError,
    models::{
        execution::Erc20AllowanceRequest,
        provider::PortfolioPosition,
        wallet::{TransactionCalldata, WalletApproval},
    },
    services::contracts::PortfolioHolding,
    AppState,
};

pub async fn summary(
    State(state): State<AppState>,
    Path(address): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let provider = state.services.provider.provider().await;
    let mut summary = state
        .services
        .wallet
        .summary(provider, &address)
        .await
        .map_err(|err| ApiError::Service(err.to_string()))?;

    // The RPC nonce is ground truth for outgoing transactions on Mantle;
    // provider profiles either hardcode 0 (Nansen) or a canned value (mock).
    if let Ok(tx_count) = state
        .services
        .contracts
        .get_transaction_count(&address)
        .await
    {
        summary.transaction_count = tx_count;
    }

    // Real on-chain portfolio (native MNT + priced ERC-20s). Merge any token
    // the provider did not already report; native MNT leads the list.
    let portfolio = state.services.contracts.get_portfolio(&address).await;
    for (index, holding) in portfolio.positions.into_iter().enumerate() {
        let already_listed = summary
            .balances
            .iter()
            .any(|position| position.symbol.eq_ignore_ascii_case(&holding.symbol));
        if already_listed {
            continue;
        }
        let position = portfolio_position_from_holding(&holding);
        if holding.symbol == "MNT" && index == 0 {
            summary.balances.insert(0, position);
        } else {
            summary.balances.push(position);
        }
    }

    summary.mainnet_balances = summary.balances.clone();
    let testnet_portfolio = state
        .services
        .contracts
        .get_execution_portfolio(&address)
        .await;
    summary.testnet_balances = testnet_portfolio
        .positions
        .iter()
        .map(portfolio_position_from_holding)
        .collect();
    summary.seer_token_faucet_calldata = state
        .services
        .contracts
        .seer_token_faucet_calldata(&address);

    Ok(Json(json!(summary)))
}

fn portfolio_position_from_holding(holding: &PortfolioHolding) -> PortfolioPosition {
    PortfolioPosition {
        symbol: holding.symbol.clone(),
        amount: format!("{:.6}", holding.amount),
        usd_value: holding.usd_value,
        protocol: None,
    }
}

pub async fn activity(
    State(state): State<AppState>,
    Path(address): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let provider = state.services.provider.provider().await;
    let activity = provider
        .get_wallet_transactions(&address)
        .await
        .map_err(|err| ApiError::Service(err.to_string()))?;
    Ok(Json(json!({ "address": address, "activity": activity })))
}

pub async fn risk(
    State(state): State<AppState>,
    Path(address): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let provider = state.services.provider.provider().await;
    let profile = provider
        .get_wallet_profile(&address)
        .await
        .map_err(|err| ApiError::Service(err.to_string()))?;
    Ok(Json(json!({
        "address": address,
        "risk_score": profile.risk_score,
        "risk_band": if profile.risk_score >= 75 { "high" } else if profile.risk_score >= 45 { "medium" } else { "low" }
    })))
}

pub async fn approvals(
    State(state): State<AppState>,
    Path(address): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let mut approvals = Vec::new();
    for target in state.services.execution.approval_targets() {
        let allowance_result = state
            .services
            .contracts
            .erc20_allowance(Erc20AllowanceRequest {
                token_address: target.token_address.clone(),
                owner_address: address.clone(),
                spender_address: target.spender_address.clone(),
            })
            .await;

        let (allowance_hex, allowance_value, read_error) = match allowance_result {
            Ok(response) => {
                let value = parse_rpc_u256(&response.allowance).unwrap_or_else(|_| U256::zero());
                (response.allowance, value, None)
            }
            Err(err) => ("0x0".to_string(), U256::zero(), Some(err.to_string())),
        };
        let active = !allowance_value.is_zero();
        let revoke_calldata = active
            .then(|| {
                state
                    .services
                    .contracts
                    .erc20_approve_calldata(&target.spender_address, U256::zero())
            })
            .flatten()
            .map(|data| TransactionCalldata {
                label: format!(
                    "Revoke {} approval for {}",
                    target.token_symbol, target.spender_label
                ),
                to: target.token_address.clone(),
                data,
                chain_id: state.services.contracts.chain_id,
            });

        approvals.push(WalletApproval {
            id: format!(
                "{}:{}",
                target.token_address.to_lowercase(),
                target.spender_address.to_lowercase()
            ),
            token_symbol: target.token_symbol.clone(),
            token_address: target.token_address,
            spender_label: target.spender_label,
            spender_address: target.spender_address,
            allowance: allowance_hex,
            allowance_display: format_token_units(
                allowance_value,
                token_decimals(&target.token_symbol),
            ),
            active,
            revoke_calldata,
            read_error,
        });
    }
    approvals.sort_by(|left, right| {
        right
            .active
            .cmp(&left.active)
            .then(left.token_symbol.cmp(&right.token_symbol))
            .then(left.spender_label.cmp(&right.spender_label))
    });

    Ok(Json(json!({
        "address": address,
        "chain_id": state.services.contracts.chain_id,
        "approvals": approvals,
    })))
}

fn parse_rpc_u256(value: &str) -> anyhow::Result<U256> {
    let trimmed = value.trim();
    let digits = trimmed.strip_prefix("0x").unwrap_or(trimmed);
    if digits.is_empty() {
        anyhow::bail!("empty value")
    }
    Ok(U256::from_str_radix(digits, 16)?)
}

fn token_decimals(symbol: &str) -> u32 {
    match symbol.to_uppercase().as_str() {
        "USDC" | "USDT" => 6,
        _ => 18,
    }
}

fn format_token_units(value: U256, decimals: u32) -> String {
    if value.is_zero() {
        return "0".to_string();
    }
    let digits = value.to_string();
    if decimals == 0 {
        return digits;
    }

    let decimals = decimals as usize;
    let padded = if digits.len() <= decimals {
        format!("{}{}", "0".repeat(decimals + 1 - digits.len()), digits)
    } else {
        digits
    };
    let split = padded.len() - decimals;
    let whole = &padded[..split];
    let fraction = padded[split..].trim_end_matches('0');
    if fraction.is_empty() {
        return whole.to_string();
    }
    let shown_fraction = &fraction[..fraction.len().min(6)];
    format!("{whole}.{shown_fraction}")
}
