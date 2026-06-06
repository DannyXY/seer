use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserSettings {
    pub telegram_alerts: bool,
    pub risk_alert: u8,
    pub confidence_alert: u8,
    pub depeg_sensitivity: f32,
    pub spend_limit: u32,
    pub autonomous_execution: bool,
}

impl Default for UserSettings {
    fn default() -> Self {
        Self {
            telegram_alerts: true,
            risk_alert: 70,
            confidence_alert: 80,
            depeg_sensitivity: 2.0,
            spend_limit: 2_000,
            autonomous_execution: true,
        }
    }
}
