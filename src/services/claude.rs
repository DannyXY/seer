use crate::{
    config::Settings,
    models::{agent::ParsedIntent, signals::Signal},
};

#[derive(Clone)]
pub struct ClaudeService {
    client: reqwest::Client,
    api_key: Option<String>,
    model: String,
}

impl ClaudeService {
    pub fn new(settings: Settings) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key: settings.claude_api_key,
            model: settings.claude_model,
        }
    }

    pub async fn explain_signal(&self, signal: &Signal) -> anyhow::Result<String> {
        let _ = (&self.client, &self.api_key, &self.model);
        Ok(format!(
            "{}. Seer detected this from structured provider facts and scored confidence at {}. This is an intelligence signal, not a guarantee of future results.",
            signal.headline, signal.confidence_score
        ))
    }

    pub async fn explain_prediction(
        &self,
        metric: &str,
        target_value: f64,
    ) -> anyhow::Result<String> {
        Ok(format!(
            "Seer is tracking {metric} against a target value of {target_value}. The reasoning is based on provider metrics, recent flows, and risk constraints."
        ))
    }

    pub async fn parse_intent_explanation(&self, parsed: &ParsedIntent) -> anyhow::Result<String> {
        Ok(format!(
            "Intent parsed as {:?} with {} condition(s). User authorization remains required unless a scoped execution policy is active.",
            parsed.trigger.mode,
            parsed.trigger.conditions.len()
        ))
    }
}
