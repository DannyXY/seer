use serde_json::json;

/// Sends alert messages through the Telegram Bot API. When TELEGRAM_BOT_TOKEN
/// or TELEGRAM_CHAT_ID is unset the service is a silent no-op.
pub struct NotifierService {
    client: reqwest::Client,
    telegram: Option<TelegramConfig>,
}

struct TelegramConfig {
    bot_token: String,
    chat_id: String,
}

impl NotifierService {
    pub fn new(bot_token: Option<String>, chat_id: Option<String>) -> Self {
        let telegram = match (bot_token, chat_id) {
            (Some(bot_token), Some(chat_id)) => Some(TelegramConfig { bot_token, chat_id }),
            _ => None,
        };
        Self {
            client: reqwest::Client::new(),
            telegram,
        }
    }

    pub fn is_configured(&self) -> bool {
        self.telegram.is_some()
    }

    pub async fn send_message(&self, text: &str) -> anyhow::Result<()> {
        let Some(telegram) = &self.telegram else {
            return Ok(());
        };
        let url = format!(
            "https://api.telegram.org/bot{}/sendMessage",
            telegram.bot_token
        );
        let response = self
            .client
            .post(&url)
            .json(&json!({ "chat_id": telegram.chat_id, "text": text }))
            .send()
            .await?;
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("telegram sendMessage failed with status {status}: {body}");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn unconfigured_notifier_is_silent_noop() {
        let notifier = NotifierService::new(None, None);
        assert!(!notifier.is_configured());
        assert!(notifier.send_message("hello").await.is_ok());
    }

    #[test]
    fn configured_only_when_both_values_present() {
        assert!(!NotifierService::new(Some("token".into()), None).is_configured());
        assert!(!NotifierService::new(None, Some("chat".into())).is_configured());
        assert!(NotifierService::new(Some("token".into()), Some("chat".into())).is_configured());
    }
}
