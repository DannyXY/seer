use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserSettings {
    pub telegram_alerts: bool,
    /// Telegram @username the user wants alerts delivered to. Stored without the
    /// leading "@". Empty/None until the user sets it.
    #[serde(default)]
    pub telegram_username: Option<String>,
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
            telegram_username: None,
            risk_alert: 70,
            confidence_alert: 80,
            depeg_sensitivity: 2.0,
            spend_limit: 2_000,
            autonomous_execution: true,
        }
    }
}
