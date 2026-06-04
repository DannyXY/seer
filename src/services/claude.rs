use std::time::Duration;

use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tracing::warn;

use crate::{
    config::Settings,
    models::{agent::ParsedIntent, signals::Signal},
};

const ANTHROPIC_API_URL: &str = "https://api.anthropic.com/v1/messages";
const ANTHROPIC_VERSION: &str = "2023-06-01";

#[derive(Clone)]
pub struct ClaudeService {
    client: reqwest::Client,
    api_key: Option<String>,
    model: String,
}

#[derive(Debug, Serialize)]
struct ClaudeMessageRequest {
    model: String,
    max_tokens: u32,
    system: String,
    messages: Vec<ClaudeInputMessage>,
}

#[derive(Debug, Serialize)]
struct ClaudeInputMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ClaudeMessageResponse {
    content: Vec<ClaudeContentBlock>,
}

#[derive(Debug, Deserialize)]
struct ClaudeContentBlock {
    #[serde(rename = "type")]
    content_type: String,
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SignalExplanationJson {
    headline: String,
    explanation: String,
    risk_note: String,
    confidence_reason: String,
}

#[derive(Debug, Deserialize)]
struct IntentExplanationJson {
    summary: String,
    trigger_summary: String,
    authorization_note: String,
}

impl ClaudeService {
    pub fn new(settings: Settings) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(18))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            client,
            api_key: settings.claude_api_key,
            model: settings.claude_model,
        }
    }

    pub async fn explain_signal(&self, signal: &Signal) -> anyhow::Result<String> {
        let fallback = self.fallback_signal_explanation(signal);
        let Some(text) = self
            .json_completion(
                "You explain on-chain intelligence for Seer. Use only the supplied JSON facts. Do not invent prices, yields, guarantees, or recommendations. Return JSON only.",
                &json!({
                    "task": "explain_signal",
                    "required_schema": {
                        "headline": "string",
                        "explanation": "string",
                        "risk_note": "string",
                        "confidence_reason": "string"
                    },
                    "facts": signal
                }),
                700,
            )
            .await?
        else {
            return Ok(fallback);
        };

        match parse_json_object::<SignalExplanationJson>(&text) {
            Ok(parsed)
                if valid_plain_copy(&parsed.headline) && valid_plain_copy(&parsed.explanation) =>
            {
                Ok(format!(
                    "{}. {} {} {}",
                    parsed.headline, parsed.explanation, parsed.risk_note, parsed.confidence_reason
                ))
            }
            Ok(_) => {
                warn!("Claude signal explanation failed validation; using fallback");
                Ok(fallback)
            }
            Err(err) => {
                warn!("Claude signal explanation was not valid JSON: {err}");
                Ok(fallback)
            }
        }
    }

    pub async fn explain_prediction(
        &self,
        metric: &str,
        target_value: f64,
    ) -> anyhow::Result<String> {
        let fallback = format!(
            "Seer is tracking {metric} against a target value of {target_value}. The reasoning is based on provider metrics, recent flows, and risk constraints."
        );
        let Some(text) = self
            .json_completion(
                "You explain points-based Arena predictions. Use only supplied facts. Do not use betting language. Return JSON only.",
                &json!({
                    "task": "explain_prediction",
                    "required_schema": {
                        "reasoning": "string",
                        "risk_note": "string"
                    },
                    "facts": {
                        "metric": metric,
                        "target_value": target_value,
                        "competition_type": "points-based prediction competition"
                    }
                }),
                500,
            )
            .await?
        else {
            return Ok(fallback);
        };

        let value: Value = match parse_json_object(&text) {
            Ok(value) => value,
            Err(err) => {
                warn!("Claude prediction explanation was not valid JSON: {err}");
                return Ok(fallback);
            }
        };
        Ok(value
            .get("reasoning")
            .and_then(Value::as_str)
            .filter(|reasoning| valid_plain_copy(reasoning))
            .map(str::to_string)
            .unwrap_or(fallback))
    }

    pub async fn parse_intent_explanation(&self, parsed: &ParsedIntent) -> anyhow::Result<String> {
        let fallback = format!(
            "Intent parsed as {:?} with {} condition(s). User authorization remains required unless a scoped execution policy is active.",
            parsed.trigger.mode,
            parsed.trigger.conditions.len()
        );
        let Some(text) = self
            .json_completion(
                "You explain parsed DeFi intents for Seer. Use only the parsed JSON. Do not claim execution has happened. Return JSON only.",
                &json!({
                    "task": "explain_parsed_intent",
                    "required_schema": {
                        "summary": "string",
                        "trigger_summary": "string",
                        "authorization_note": "string"
                    },
                    "parsed_intent": parsed
                }),
                600,
            )
            .await?
        else {
            return Ok(fallback);
        };

        match parse_json_object::<IntentExplanationJson>(&text) {
            Ok(parsed)
                if valid_plain_copy(&parsed.summary)
                    && valid_plain_copy(&parsed.trigger_summary)
                    && valid_plain_copy(&parsed.authorization_note) =>
            {
                Ok(format!(
                    "{} {} {}",
                    parsed.summary, parsed.trigger_summary, parsed.authorization_note
                ))
            }
            Ok(_) => {
                warn!("Claude intent explanation failed validation; using fallback");
                Ok(fallback)
            }
            Err(err) => {
                warn!("Claude intent explanation was not valid JSON: {err}");
                Ok(fallback)
            }
        }
    }

    async fn json_completion(
        &self,
        system: &str,
        input: &Value,
        max_tokens: u32,
    ) -> anyhow::Result<Option<String>> {
        let Some(api_key) = self.api_key.as_deref() else {
            return Ok(None);
        };

        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(
            "anthropic-version",
            HeaderValue::from_static(ANTHROPIC_VERSION),
        );
        headers.insert("x-api-key", HeaderValue::from_str(api_key)?);

        let request = ClaudeMessageRequest {
            model: self.model.clone(),
            max_tokens,
            system: system.to_string(),
            messages: vec![ClaudeInputMessage {
                role: "user".to_string(),
                content: input.to_string(),
            }],
        };

        let response = self
            .client
            .post(ANTHROPIC_API_URL)
            .headers(headers)
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            warn!("Claude request failed with status {status}: {body}");
            return Ok(None);
        }

        let response = response.json::<ClaudeMessageResponse>().await?;
        Ok(response
            .content
            .into_iter()
            .find(|block| block.content_type == "text")
            .and_then(|block| block.text))
    }

    fn fallback_signal_explanation(&self, signal: &Signal) -> String {
        format!(
            "{}. Seer detected this from structured provider facts and scored confidence at {}. This is an intelligence signal, not a guarantee of future results.",
            signal.headline, signal.confidence_score
        )
    }
}

fn parse_json_object<T>(raw: &str) -> anyhow::Result<T>
where
    T: for<'de> Deserialize<'de>,
{
    let trimmed = raw.trim();
    if let Ok(parsed) = serde_json::from_str(trimmed) {
        return Ok(parsed);
    }
    let start = trimmed
        .find('{')
        .ok_or_else(|| anyhow::anyhow!("JSON object start not found"))?;
    let end = trimmed
        .rfind('}')
        .ok_or_else(|| anyhow::anyhow!("JSON object end not found"))?;
    Ok(serde_json::from_str(&trimmed[start..=end])?)
}

fn valid_plain_copy(value: &str) -> bool {
    let normalized = value.trim().to_lowercase();
    !normalized.is_empty()
        && !normalized.contains("guaranteed")
        && !normalized.contains("risk-free")
        && !normalized.contains("financial advice")
}

#[cfg(test)]
mod tests {
    use super::{parse_json_object, valid_plain_copy, SignalExplanationJson};

    #[test]
    fn parses_json_from_wrapped_claude_text() {
        let parsed = parse_json_object::<SignalExplanationJson>(
            "```json\n{\"headline\":\"Smart wallets moved\",\"explanation\":\"Provider facts show inflows.\",\"risk_note\":\"This can reverse.\",\"confidence_reason\":\"Multiple wallet clusters aligned.\"}\n```",
        )
        .unwrap();

        assert_eq!(parsed.headline, "Smart wallets moved");
    }

    #[test]
    fn rejects_guarantee_language() {
        assert!(!valid_plain_copy("This is guaranteed profit."));
        assert!(valid_plain_copy("Provider facts show increased activity."));
    }
}
