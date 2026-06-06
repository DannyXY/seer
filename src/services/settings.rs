use std::collections::HashMap;
use std::sync::RwLock;

use crate::models::settings::UserSettings;

pub struct SettingsService {
    settings: RwLock<HashMap<String, UserSettings>>,
}

impl SettingsService {
    pub fn new() -> Self {
        Self {
            settings: RwLock::new(HashMap::new()),
        }
    }

    pub fn get(&self, wallet_address: &str) -> UserSettings {
        self.settings
            .read()
            .expect("settings store poisoned")
            .get(&wallet_address.to_lowercase())
            .cloned()
            .unwrap_or_default()
    }

    pub fn save(&self, wallet_address: &str, settings: UserSettings) -> UserSettings {
        self.settings
            .write()
            .expect("settings store poisoned")
            .insert(wallet_address.to_lowercase(), settings.clone());
        settings
    }
}
