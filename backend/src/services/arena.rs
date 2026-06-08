use std::collections::HashMap;
use std::sync::RwLock;

use chrono::{Duration, Utc};
use uuid::Uuid;

use crate::models::arena::{
    ArenaEntry, ArenaEntryRequest, ArenaEntryStatus, ArenaPosition, ArenaPrediction,
    ComparisonOperator, LeaderboardRow, PredictionStatus,
};

#[derive(Clone)]
pub struct ArenaService {
    predictions: std::sync::Arc<RwLock<HashMap<Uuid, ArenaPrediction>>>,
    entries: std::sync::Arc<RwLock<HashMap<Uuid, ArenaEntry>>>,
    points: std::sync::Arc<RwLock<HashMap<String, u32>>>,
}

impl ArenaService {
    pub fn new() -> Self {
        let prediction = seed_prediction();
        Self {
            predictions: std::sync::Arc::new(RwLock::new(HashMap::from([(prediction.id, prediction)]))),
            entries: std::sync::Arc::new(RwLock::new(HashMap::new())),
            points: std::sync::Arc::new(RwLock::new(HashMap::new())),
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

    pub fn get_prediction_by_onchain_id(&self, onchain_id: u64) -> Option<ArenaPrediction> {
        self.predictions
            .read()
            .expect("arena prediction store poisoned")
            .values()
            .find(|p| p.onchain_prediction_id == Some(onchain_id))
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

    /// Sync the in-memory balance up to `onchain_balance` when it is higher.
    /// Called before bet placement so a wallet that claimed on-chain but
    /// whose balance isn't mirrored here yet isn't rejected.
    pub fn sync_points(&self, wallet_address: &str, onchain_balance: u64) {
        let key = wallet_address.to_lowercase();
        let mut points = self.points.write().expect("arena points store poisoned");
        let current = *points.get(&key).unwrap_or(&0);
        let onchain = onchain_balance.min(u32::MAX as u64) as u32;
        if onchain > current {
            points.insert(key, onchain);
        }
    }

    pub fn user_points(&self, wallet_address: &str) -> u32 {
        let key = wallet_address.to_lowercase();
        *self
            .points
            .read()
            .expect("arena points store poisoned")
            .get(&key)
            .unwrap_or(&0)
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

        let wallet_key = request.wallet_address.to_lowercase();

        // Acquire locks in a consistent order (points → entries) to prevent deadlock.
        // leaderboard() also reads points before entries, so this order is safe.
        let mut points = self.points.write().expect("arena points store poisoned");
        let balance = *points.get(&wallet_key).unwrap_or(&0);
        if request.points_committed > balance {
            anyhow::bail!("insufficient points balance");
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

        // Acquire in consistent order: points first, then entries.
        let points_snap = self
            .points
            .read()
            .expect("arena points store poisoned")
            .clone();
        let entries_snap = self
            .entries
            .read()
            .expect("arena entry store poisoned")
            .clone();

        for (wallet, balance) in &points_snap {
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

        for entry in entries_snap.values() {
            let wallet_key = entry.wallet_address.to_lowercase();
            let row = rows_by_wallet
                .entry(wallet_key.clone())
                .or_insert_with(|| LeaderboardRow {
                    rank: 0,
                    wallet_address: entry.wallet_address.clone(),
                    total_points: *points_snap.get(&wallet_key).unwrap_or(&0) as i32,
                    weekly_gain: 0,
                    accuracy_rate: None,
                    entries_count: 0,
                });
            row.entries_count += 1;
            if matches!(entry.status, ArenaEntryStatus::Active) {
                // Include locked (committed) points so the leaderboard reflects true total.
                row.total_points += entry.points_committed as i32;
            }
            if let Some(delta) = entry.points_delta {
                row.weekly_gain += delta;
            }
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

    /// Resolve all expired open predictions given a metric lookup function.
    /// Returns the IDs of predictions that were resolved.
    pub fn resolve_expired<F>(&self, fetch_metric: F) -> Vec<Uuid>
    where
        F: Fn(&str) -> Option<f64>,
    {
        let now = Utc::now();

        // Collect expired open predictions without holding the write lock.
        let expired: Vec<ArenaPrediction> = self
            .predictions
            .read()
            .expect("arena prediction store poisoned")
            .values()
            .filter(|p| matches!(p.status, PredictionStatus::Open) && p.expiry_time <= now)
            .cloned()
            .collect();

        let mut resolved_ids = Vec::new();

        for mut prediction in expired {
            let Some(final_value) = fetch_metric(&prediction.metric) else {
                continue;
            };

            let seer_correct = match prediction.comparison_operator {
                ComparisonOperator::GreaterThanOrEqual => final_value >= prediction.target_value,
                ComparisonOperator::LessThanOrEqual => final_value <= prediction.target_value,
            };

            let outcome_label = if seer_correct { "SeerCorrect" } else { "SeerIncorrect" };
            prediction.status = PredictionStatus::Resolved;
            prediction.final_value = Some(final_value);
            prediction.result = Some(outcome_label.to_string());

            self.predictions
                .write()
                .expect("arena prediction store poisoned")
                .insert(prediction.id, prediction.clone());

            // Acquire in consistent order: points first, entries second.
            let mut points = self.points.write().expect("arena points store poisoned");
            let mut entries = self.entries.write().expect("arena entry store poisoned");

            for entry in entries.values_mut().filter(|e| {
                e.prediction_id == prediction.id && matches!(e.status, ArenaEntryStatus::Active)
            }) {
                let entry_backed_seer = matches!(entry.user_position, ArenaPosition::BackSeer)
                    == matches!(prediction.seer_position, ArenaPosition::BackSeer);
                let entry_correct = if seer_correct {
                    entry_backed_seer
                } else {
                    !entry_backed_seer
                };

                let delta = if entry_correct {
                    entry.points_committed as i32
                } else {
                    -(entry.points_committed as i32)
                };

                entry.points_delta = Some(delta);
                entry.status = ArenaEntryStatus::Resolved;
                entry.resolved_at = Some(now);

                let wallet_key = entry.wallet_address.to_lowercase();
                let available = *points.get(&wallet_key).unwrap_or(&0);
                let new_balance = if delta >= 0 {
                    available + entry.points_committed + delta as u32
                } else {
                    available.saturating_sub((-delta) as u32 - entry.points_committed.min((-delta) as u32))
                        + entry.points_committed.saturating_sub((-delta) as u32)
                };
                points.insert(wallet_key, new_balance);
            }

            resolved_ids.push(prediction.id);
        }

        resolved_ids
    }

    /// Bulk-load predictions into in-memory store without overwriting existing entries.
    pub fn seed_predictions(&self, predictions: Vec<ArenaPrediction>) {
        let mut store = self.predictions.write().expect("arena prediction store poisoned");
        for pred in predictions {
            store.entry(pred.id).or_insert(pred);
        }
    }

    /// Bulk-load entries into in-memory store without overwriting existing entries.
    pub fn seed_entries(&self, entries: Vec<ArenaEntry>) {
        let mut store = self.entries.write().expect("arena entry store poisoned");
        for entry in entries {
            store.entry(entry.id).or_insert(entry);
        }
    }

    /// Set the on-chain prediction ID for the seed prediction (called after contract creation).
    pub fn register_onchain_prediction_id(&self, prediction_id: Uuid, onchain_id: u64) {
        let mut predictions = self.predictions.write().expect("arena prediction store poisoned");
        if let Some(pred) = predictions.get_mut(&prediction_id) {
            pred.onchain_prediction_id = Some(onchain_id);
        }
    }

    pub fn seer_record(&self) -> (u32, u32) {
        let entries_snap = self
            .entries
            .read()
            .expect("arena entry store poisoned")
            .clone();
        let predictions_snap = self
            .predictions
            .read()
            .expect("arena prediction store poisoned")
            .clone();

        let mut total = 0u32;
        let mut correct = 0u32;

        for entry in entries_snap.values() {
            if !matches!(entry.status, ArenaEntryStatus::Resolved) {
                continue;
            }
            let Some(prediction) = predictions_snap.get(&entry.prediction_id) else {
                continue;
            };
            let seer_correct = prediction
                .result
                .as_deref()
                .map(|r| r == "SeerCorrect")
                .unwrap_or(false);

            total += 1;
            if seer_correct {
                correct += 1;
            }
        }

        (total, correct)
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

    fn funded_request(wallet_address: &str) -> ArenaEntryRequest {
        ArenaEntryRequest {
            wallet_address: wallet_address.to_string(),
            user_position: ArenaPosition::BackSeer,
            points_committed: 100,
        }
    }

    fn fund(service: &ArenaService, wallet: &str, amount: u32) {
        service.points.write().expect("poisoned")
            .insert(wallet.to_lowercase(), amount);
    }

    #[test]
    fn enters_prediction_and_lists_wallet_entries() {
        let service = ArenaService::new();
        let prediction_id = service.predictions()[0].id;
        fund(&service, "0xabc", 500);

        let entry = service
            .enter_prediction(prediction_id, funded_request("0xabc"))
            .unwrap();

        assert_eq!(entry.prediction_id, prediction_id);
        assert_eq!(service.entries_for_wallet("0xABC").len(), 1);
        assert!(matches!(entry.status, ArenaEntryStatus::Active));
    }

    #[test]
    fn rejects_duplicate_prediction_entry_for_same_wallet() {
        let service = ArenaService::new();
        let prediction_id = service.predictions()[0].id;
        fund(&service, "0xabc", 500);

        service
            .enter_prediction(prediction_id, funded_request("0xabc"))
            .unwrap();
        let duplicate = service.enter_prediction(prediction_id, funded_request("0xABC"));

        assert!(duplicate.is_err());
    }

    #[test]
    fn leaderboard_reflects_entered_wallet() {
        let service = ArenaService::new();
        let prediction_id = service.predictions()[0].id;
        fund(&service, "0xleader", 500);

        service
            .enter_prediction(prediction_id, funded_request("0xleader"))
            .unwrap();
        let leaderboard = service.leaderboard();

        assert_eq!(leaderboard[0].wallet_address, "0xleader");
        assert_eq!(leaderboard[0].entries_count, 1);
        // 400 available + 100 locked = 500 total
        assert_eq!(leaderboard[0].total_points, 500);
    }
}
