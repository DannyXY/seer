use std::collections::HashMap;
use std::sync::RwLock;

use chrono::{Duration, Utc};
use uuid::Uuid;

use crate::models::arena::{
    ArenaEntry, ArenaEntryRequest, ArenaEntryStatus, ArenaPosition, ArenaPrediction,
    ComparisonOperator, LeaderboardRow, PredictionStatus,
};

pub struct ArenaService {
    predictions: RwLock<HashMap<Uuid, ArenaPrediction>>,
    entries: RwLock<HashMap<Uuid, ArenaEntry>>,
    points: RwLock<HashMap<String, u32>>,
}

impl ArenaService {
    pub fn new() -> Self {
        let prediction = seed_prediction();
        Self {
            predictions: RwLock::new(HashMap::from([(prediction.id, prediction)])),
            entries: RwLock::new(HashMap::new()),
            points: RwLock::new(HashMap::new()),
        }
    }

    pub fn predictions(&self) -> Vec<ArenaPrediction> {
        self.predictions
            .read()
            .expect("arena prediction store poisoned")
            .values()
            .cloned()
            .collect()
    }

    pub fn get_prediction(&self, id: Uuid) -> Option<ArenaPrediction> {
        self.predictions
            .read()
            .expect("arena prediction store poisoned")
            .get(&id)
            .cloned()
    }

    pub fn validate_entry(&self, entry: &ArenaEntryRequest) -> anyhow::Result<()> {
        if entry.points_committed == 0 || entry.points_committed > 1_000 {
            anyhow::bail!("points_committed must be between 1 and 1000");
        }
        if entry.wallet_address.trim().is_empty() {
            anyhow::bail!("wallet_address is required");
        }
        Ok(())
    }

    pub fn user_points(&self, wallet_address: &str) -> u32 {
        let key = wallet_address.to_lowercase();
        *self
            .points
            .read()
            .expect("arena points store poisoned")
            .get(&key)
            .unwrap_or(&1_000)
    }

    pub fn prediction_pool(&self, prediction_id: Uuid) -> u32 {
        self.entries
            .read()
            .expect("arena entry store poisoned")
            .values()
            .filter(|entry| entry.prediction_id == prediction_id)
            .map(|entry| entry.points_committed)
            .sum()
    }

    pub fn enter_prediction(
        &self,
        prediction_id: Uuid,
        request: ArenaEntryRequest,
    ) -> anyhow::Result<ArenaEntry> {
        self.validate_entry(&request)?;
        let prediction = self
            .get_prediction(prediction_id)
            .ok_or_else(|| anyhow::anyhow!("prediction not found"))?;

        if !matches!(prediction.status, PredictionStatus::Open) {
            anyhow::bail!("prediction is not open");
        }
        if prediction.expiry_time <= Utc::now() {
            anyhow::bail!("prediction has expired");
        }

        let mut entries = self.entries.write().expect("arena entry store poisoned");
        let already_entered = entries.values().any(|entry| {
            entry.prediction_id == prediction_id
                && entry
                    .wallet_address
                    .eq_ignore_ascii_case(&request.wallet_address)
        });
        if already_entered {
            anyhow::bail!("wallet already entered this prediction");
        }

        let wallet_key = request.wallet_address.to_lowercase();
        let mut points = self.points.write().expect("arena points store poisoned");
        let balance = *points.get(&wallet_key).unwrap_or(&1_000);
        if request.points_committed > balance {
            anyhow::bail!("insufficient points balance");
        }
        points.insert(wallet_key, balance - request.points_committed);

        let entry = ArenaEntry {
            id: Uuid::new_v4(),
            prediction_id,
            wallet_address: request.wallet_address,
            user_position: request.user_position,
            points_committed: request.points_committed,
            status: ArenaEntryStatus::Active,
            points_delta: None,
            tx_hash: None,
            created_at: Utc::now(),
            resolved_at: None,
        };
        entries.insert(entry.id, entry.clone());
        Ok(entry)
    }

    pub fn entries_for_wallet(&self, wallet_address: &str) -> Vec<ArenaEntry> {
        self.entries
            .read()
            .expect("arena entry store poisoned")
            .values()
            .filter(|entry| entry.wallet_address.eq_ignore_ascii_case(wallet_address))
            .cloned()
            .collect()
    }

    pub fn leaderboard(&self) -> Vec<LeaderboardRow> {
        let mut rows_by_wallet: HashMap<String, LeaderboardRow> = HashMap::new();

        for (wallet, balance) in self
            .points
            .read()
            .expect("arena points store poisoned")
            .iter()
        {
            rows_by_wallet.insert(
                wallet.clone(),
                LeaderboardRow {
                    rank: 0,
                    wallet_address: wallet.clone(),
                    total_points: *balance as i32,
                    weekly_gain: 0,
                    accuracy_rate: None,
                    entries_count: 0,
                },
            );
        }

        for entry in self
            .entries
            .read()
            .expect("arena entry store poisoned")
            .values()
        {
            let row = rows_by_wallet
                .entry(entry.wallet_address.clone())
                .or_insert_with(|| LeaderboardRow {
                    rank: 0,
                    wallet_address: entry.wallet_address.clone(),
                    total_points: 1_000,
                    weekly_gain: 0,
                    accuracy_rate: None,
                    entries_count: 0,
                });
            row.entries_count += 1;
            row.weekly_gain += entry.points_delta.unwrap_or(0);
            row.total_points += entry.points_delta.unwrap_or(0);
        }

        if rows_by_wallet.is_empty() {
            rows_by_wallet.insert(
                "0xsmart000000000000000000000000000000000001".to_string(),
                LeaderboardRow {
                    rank: 0,
                    wallet_address: "0xsmart000000000000000000000000000000000001".to_string(),
                    total_points: 1_240,
                    weekly_gain: 240,
                    accuracy_rate: Some(0.72),
                    entries_count: 6,
                },
            );
        }

        let mut rows: Vec<_> = rows_by_wallet.into_values().collect();
        rows.sort_by(|a, b| b.total_points.cmp(&a.total_points));
        for (index, row) in rows.iter_mut().enumerate() {
            row.rank = (index + 1) as u32;
        }
        rows
    }
}

fn seed_prediction() -> ArenaPrediction {
    ArenaPrediction {
        id: Uuid::from_u128(0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa),
        onchain_prediction_id: None,
        claim: "mETH Protocol TVL will stay above $40M for the next 24 hours".to_string(),
        metric: "protocol.tvl_usd:mETH Protocol".to_string(),
        target_value: 40_000_000.0,
        comparison_operator: ComparisonOperator::GreaterThanOrEqual,
        expiry_time: Utc::now() + Duration::hours(24),
        seer_position: ArenaPosition::BackSeer,
        seer_confidence: 76,
        reasoning: "Recent smart-money inflows and TVL momentum support this claim.".to_string(),
        status: PredictionStatus::Open,
        result: None,
        final_value: None,
        created_at: Utc::now(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(wallet_address: &str) -> ArenaEntryRequest {
        ArenaEntryRequest {
            wallet_address: wallet_address.to_string(),
            user_position: ArenaPosition::BackSeer,
            points_committed: 100,
        }
    }

    #[test]
    fn enters_prediction_and_lists_wallet_entries() {
        let service = ArenaService::new();
        let prediction_id = service.predictions()[0].id;

        let entry = service
            .enter_prediction(prediction_id, request("0xabc"))
            .unwrap();

        assert_eq!(entry.prediction_id, prediction_id);
        assert_eq!(service.entries_for_wallet("0xABC").len(), 1);
        assert!(matches!(entry.status, ArenaEntryStatus::Active));
    }

    #[test]
    fn rejects_duplicate_prediction_entry_for_same_wallet() {
        let service = ArenaService::new();
        let prediction_id = service.predictions()[0].id;

        service
            .enter_prediction(prediction_id, request("0xabc"))
            .unwrap();
        let duplicate = service.enter_prediction(prediction_id, request("0xABC"));

        assert!(duplicate.is_err());
    }

    #[test]
    fn leaderboard_reflects_entered_wallet() {
        let service = ArenaService::new();
        let prediction_id = service.predictions()[0].id;

        service
            .enter_prediction(prediction_id, request("0xleader"))
            .unwrap();
        let leaderboard = service.leaderboard();

        assert_eq!(leaderboard[0].wallet_address, "0xleader");
        assert_eq!(leaderboard[0].entries_count, 1);
        assert_eq!(leaderboard[0].total_points, 900);
    }
}
