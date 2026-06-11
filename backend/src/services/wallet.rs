use crate::{models::wallet::WalletSummary, services::data_provider::OnchainDataProvider};

pub struct WalletService;

impl WalletService {
    pub fn new() -> Self {
        Self
    }

    pub async fn summary(
        &self,
        provider: &dyn OnchainDataProvider,
        address: &str,
    ) -> anyhow::Result<WalletSummary> {
        let profile = match provider.get_wallet_profile(address).await {
            Ok(profile) => profile,
            Err(_) => {
                crate::services::data_provider::MockProvider
                    .get_wallet_profile(address)
                    .await?
            }
        };
        let balances = provider
            .get_wallet_positions(address)
            .await
            .unwrap_or_else(|_| Vec::new());

        let mainnet_balances = balances.clone();

        Ok(WalletSummary {
            address: profile.address,
            network: profile.network,
            balances,
            mainnet_balances,
            testnet_balances: Vec::new(),
            seer_token_faucet_calldata: None,
            risk_score: profile.risk_score,
            wallet_age_days: profile.wallet_age_days,
            protocols_used: profile.protocols_used.len(),
            transaction_count: profile.transaction_count,
        })
    }
}
