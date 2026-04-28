use anyhow::{anyhow, Context, Result};
use reqwest::blocking::Client;
use reqwest::StatusCode;
use serde_json::{json, Value};
use std::env;

const DEFAULT_MODEL: &str = "gpt-5.4-mini";
const DEFAULT_INSTRUCTIONS: &str = "You are a concise terminal assistant. Answer directly, avoid markdown tables unless useful, and keep responses practical.";

#[derive(Debug)]
pub(crate) struct AgentResponse {
    pub(crate) text: String,
    pub(crate) total_tokens: Option<u64>,
}

#[derive(Debug)]
struct AgentConfig {
    api_key: String,
    model: String,
    base_url: String,
    instructions: String,
    max_output_tokens: u64,
}

impl AgentConfig {
    fn from_env() -> Result<Self> {
        Self::from_env_vars(|name| env::var(name).ok())
    }

    fn from_env_vars(get: impl Fn(&str) -> Option<String>) -> Result<Self> {
        let api_key = get("OPENAI_API_KEY")
            .or_else(|| get("AI_API_KEY"))
            .context("Missing OPENAI_API_KEY. Set it before calling the AI helper.")?;
        let model = get("OPENAI_MODEL").unwrap_or_else(|| DEFAULT_MODEL.to_string());
        let base_url =
            get("OPENAI_BASE_URL").unwrap_or_else(|| "https://api.openai.com/v1".to_string());
        let instructions = get("OUR_CLI_INSTRUCTIONS")
            .or_else(|| get("ASM_AGENT_INSTRUCTIONS"))
            .unwrap_or_else(|| DEFAULT_INSTRUCTIONS.to_string());
        let max_output_tokens = get("OPENAI_MAX_OUTPUT_TOKENS")
            .unwrap_or_else(|| "800".to_string())
            .parse::<u64>()
            .context("OPENAI_MAX_OUTPUT_TOKENS must be a positive integer.")?;

        Ok(Self {
            api_key,
            model,
            base_url,
            instructions,
            max_output_tokens,
        })
    }

    fn endpoint_url(&self) -> String {
        format!("{}/responses", self.base_url.trim_end_matches('/'))
    }

    fn request_body(&self, input: &str) -> Value {
        json!({
            "model": self.model,
            "instructions": self.instructions,
            "input": input,
            "max_output_tokens": self.max_output_tokens
        })
    }
}

pub(crate) fn ask_agent(input: &str) -> Result<AgentResponse> {
    if let Ok(mock) = env::var("OUR_CLI_MOCK_RESPONSE") {
        let total_tokens = env::var("OUR_CLI_MOCK_TOTAL_TOKENS")
            .ok()
            .and_then(|value| value.parse::<u64>().ok());
        return Ok(AgentResponse {
            text: mock,
            total_tokens,
        });
    }

    let config = AgentConfig::from_env()?;
    let client = Client::new();
    let response = client
        .post(config.endpoint_url())
        .bearer_auth(&config.api_key)
        .json(&config.request_body(input))
        .send()
        .context("OpenAI request failed before receiving a response.")?;

    let status = response.status();
    let body: Value = response
        .json()
        .context("OpenAI response was not valid JSON.")?;

    parse_agent_response(status, body)
}

fn parse_agent_response(status: StatusCode, body: Value) -> Result<AgentResponse> {
    if !status.is_success() {
        let message = body
            .pointer("/error/message")
            .and_then(Value::as_str)
            .or_else(|| body.get("message").and_then(Value::as_str))
            .unwrap_or("unknown error");
        return Err(anyhow!("OpenAI request failed ({status}): {message}"));
    }

    let text = extract_response_text(&body)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| anyhow!("OpenAI response did not include text output."))?;
    let total_tokens = body.pointer("/usage/total_tokens").and_then(Value::as_u64);

    Ok(AgentResponse { text, total_tokens })
}

fn extract_response_text(body: &Value) -> Option<String> {
    if let Some(text) = body.get("output_text").and_then(Value::as_str) {
        return Some(text.to_string());
    }

    let mut chunks = Vec::new();
    let output = body.get("output")?.as_array()?;
    for item in output {
        if item.get("type").and_then(Value::as_str) != Some("message") {
            continue;
        }
        for content in item.get("content").and_then(Value::as_array)? {
            if content.get("type").and_then(Value::as_str) == Some("output_text") {
                if let Some(text) = content.get("text").and_then(Value::as_str) {
                    chunks.push(text);
                }
            }
        }
    }

    Some(chunks.join("\n"))
}

#[cfg(test)]
#[path = "agent_tests.rs"]
mod tests;
