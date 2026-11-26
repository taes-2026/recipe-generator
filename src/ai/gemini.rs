use std::time::Instant;

use reqwest::StatusCode;
use serde_json::{json, Value};

use crate::{
    config::{Config, ConfigError},
    domain::{allowed_units, GeneratedRecipe},
};

use super::{prompt::SYSTEM_PROMPT, provider::ProviderError};

pub struct GeminiProvider {
    client: reqwest::Client,
    api_key: String,
    model: String,
    base_url: String,
    thinking_budget: Option<u32>,
}

impl GeminiProvider {
    pub fn new(config: &Config, api_key: String) -> Result<Self, ConfigError> {
        let client = reqwest::Client::builder()
            .timeout(config.upstream_timeout)
            .build()
            .map_err(|error| ConfigError::Invalid {
                var: "GEMINI_*",
                message: format!("could not build the HTTP client: {error}"),
            })?;

        Ok(Self {
            client,
            api_key,
            model: config.gemini_model.clone(),
            base_url: config.gemini_base_url.clone(),
            thinking_budget: config.gemini_thinking_budget,
        })
    }

    pub fn model(&self) -> &str {
        &self.model
    }

    pub async fn generate(&self, prompt: &str) -> Result<GeneratedRecipe, ProviderError> {
        let url = format!(
            "{}/v1beta/models/{}:generateContent",
            self.base_url, self.model
        );
        let started = Instant::now();

        tracing::debug!(model = %self.model, "calling gemini");

        let response = self
            .client
            .post(&url)
            .header("x-goog-api-key", &self.api_key)
            .json(&self.request_body(prompt))
            .send()
            .await
            .map_err(|error| {
                let mapped = map_transport_error(error);
                tracing::warn!(
                    model = %self.model,
                    elapsed_ms = started.elapsed().as_millis() as u64,
                    error = %mapped,
                    "gemini transport failed"
                );
                mapped
            })?;

        let status = response.status();
        let body = response.text().await.map_err(map_transport_error)?;
        let elapsed_ms = started.elapsed().as_millis() as u64;

        tracing::info!(model = %self.model, status = %status, elapsed_ms, "gemini responded");

        if !status.is_success() {
            let error = map_status(status, &body);
            tracing::warn!(
                model = %self.model,
                status = %status,
                elapsed_ms,
                error = %error,
                "gemini request failed"
            );
            return Err(error);
        }

        let envelope: Value = serde_json::from_str(&body)
            .map_err(|error| ProviderError::InvalidOutput(format!("provider envelope: {error}")))?;

        tracing::debug!(
            model = %self.model,
            prompt_tokens = envelope.pointer("/usageMetadata/promptTokenCount").and_then(|v| v.as_u64()),
            output_tokens = envelope.pointer("/usageMetadata/candidatesTokenCount").and_then(|v| v.as_u64()),
            "gemini usage"
        );

        parse_candidate(&envelope)
    }

    fn request_body(&self, prompt: &str) -> Value {
        let mut generation_config = json!({
            "temperature": 0.7,
            "maxOutputTokens": 2048,
            "responseMimeType": "application/json",
            "responseSchema": response_schema(),
        });

        if let Some(budget) = self.thinking_budget {
            generation_config["thinkingConfig"] = json!({ "thinkingBudget": budget });
        }

        json!({
            "systemInstruction": { "parts": [{ "text": SYSTEM_PROMPT }] },
            "contents": [{ "role": "user", "parts": [{ "text": prompt }] }],
            "generationConfig": generation_config,
        })
    }
}

fn map_transport_error(error: reqwest::Error) -> ProviderError {
    if error.is_timeout() {
        ProviderError::Timeout
    } else {
        ProviderError::Upstream(format!("transport: {error}"))
    }
}

fn map_status(status: StatusCode, body: &str) -> ProviderError {
    let message = serde_json::from_str::<Value>(body)
        .ok()
        .and_then(|value| {
            value
                .pointer("/error/message")
                .and_then(Value::as_str)
                .map(str::to_owned)
        })
        .unwrap_or_else(|| truncate(body, 300));

    if status == StatusCode::TOO_MANY_REQUESTS {
        return ProviderError::RateLimited;
    }

    ProviderError::Upstream(format!("upstream {status}: {message}"))
}

fn truncate(input: &str, max_chars: usize) -> String {
    let trimmed: String = input.chars().take(max_chars).collect();
    if trimmed.len() < input.len() {
        format!("{trimmed}...")
    } else {
        trimmed
    }
}

pub fn parse_candidate(envelope: &Value) -> Result<GeneratedRecipe, ProviderError> {
    let candidates = envelope.get("candidates").and_then(Value::as_array);

    let Some(candidates) = candidates.filter(|candidates| !candidates.is_empty()) else {
        if let Some(reason) = envelope
            .pointer("/promptFeedback/blockReason")
            .and_then(Value::as_str)
        {
            return Err(ProviderError::Refused(format!("blocked: {reason}")));
        }
        return Err(ProviderError::InvalidOutput(
            "no candidates returned".to_owned(),
        ));
    };

    let candidate = &candidates[0];

    if let Some(reason) = candidate.get("finishReason").and_then(Value::as_str) {
        if reason != "STOP" && reason != "MAX_TOKENS" {
            return Err(ProviderError::Refused(format!("finishReason={reason}")));
        }
    }

    let text = candidate
        .pointer("/content/parts/0/text")
        .and_then(Value::as_str)
        .ok_or_else(|| ProviderError::InvalidOutput("candidate had no text part".to_owned()))?;

    serde_json::from_str(text).map_err(|error| {
        ProviderError::InvalidSchema(format!("recipe JSON did not match the schema: {error}"))
    })
}

fn response_schema() -> Value {
    json!({
        "type": "OBJECT",
        "properties": {
            "title": { "type": "STRING", "description": "Dish name, pt-PT, max 90 chars" },
            "summary": { "type": "STRING", "description": "One or two sentences, pt-PT" },
            "servings": { "type": "INTEGER", "description": "Must equal the requested servings" },
            "prepMinutes": { "type": "INTEGER", "minimum": 0, "maximum": 240 },
            "cookMinutes": { "type": "INTEGER", "minimum": 0, "maximum": 240 },
            "ingredients": {
                "type": "ARRAY",
                "items": {
                    "type": "OBJECT",
                    "properties": {
                        "name": { "type": "STRING" },
                        "quantity": { "type": "NUMBER" },
                        "unit": { "type": "STRING", "enum": allowed_units() },
                        "optional": { "type": "BOOLEAN" }
                    },
                    "required": ["name", "quantity", "unit"],
                    "propertyOrdering": ["name", "quantity", "unit", "optional"]
                }
            },
            "steps": {
                "type": "ARRAY",
                "items": {
                    "type": "OBJECT",
                    "properties": {
                        "n": { "type": "INTEGER", "minimum": 1 },
                        "text": { "type": "STRING" },
                        "uses": {
                            "type": "ARRAY",
                            "items": { "type": "STRING" },
                            "description": "Exact ingredient names needed for this step"
                        },
                        "timerSeconds": { "type": "INTEGER", "minimum": 0, "maximum": 7200 }
                    },
                    "required": ["n", "text", "uses"],
                    "propertyOrdering": ["n", "text", "uses", "timerSeconds"]
                }
            },
            "tips": {
                "type": "ARRAY",
                "items": { "type": "STRING" }
            }
        },
        "required": ["title", "servings", "prepMinutes", "cookMinutes", "ingredients", "steps"],
        "propertyOrdering": ["title", "summary", "servings", "prepMinutes", "cookMinutes", "ingredients", "steps", "tips"]
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn envelope(text: &str) -> Value {
        json!({
            "candidates": [{
                "content": { "parts": [{ "text": text }] },
                "finishReason": "STOP"
            }]
        })
    }

    const RECIPE: &str = r#"{
        "title": "Salteado de tomate",
        "servings": 2,
        "prepMinutes": 5,
        "cookMinutes": 10,
        "ingredients": [{"name": "Tomate", "quantity": 2, "unit": "unit"}],
        "steps": [{"n": 1, "text": "Corte o tomate.", "uses": ["Tomate"]}]
    }"#;

    #[test]
    fn parses_a_valid_candidate() {
        let recipe = parse_candidate(&envelope(RECIPE)).expect("valid recipe");
        assert_eq!(recipe.title, "Salteado de tomate");
    }

    #[test]
    fn missing_or_blocked_candidates_are_reported() {
        assert!(matches!(
            parse_candidate(&json!({ "candidates": [] })),
            Err(ProviderError::InvalidOutput(_))
        ));

        let blocked = json!({ "candidates": [], "promptFeedback": { "blockReason": "SAFETY" } });
        assert!(matches!(
            parse_candidate(&blocked),
            Err(ProviderError::Refused(_))
        ));
    }

    #[test]
    fn malformed_json_is_a_schema_error() {
        assert!(matches!(
            parse_candidate(&envelope("not json")),
            Err(ProviderError::InvalidSchema(_))
        ));
    }

    #[test]
    fn unknown_units_are_a_schema_error() {
        let recipe = RECIPE.replace("\"unit\": \"unit\"", "\"unit\": \"colher\"");

        assert!(matches!(
            parse_candidate(&envelope(&recipe)),
            Err(ProviderError::InvalidSchema(_))
        ));
    }
}
