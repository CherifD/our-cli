use anyhow::{anyhow, Context, Result};
use reqwest::blocking::Client;
use serde_json::{json, Value};
use std::env;

const DEFAULT_MODEL: &str = "gpt-5.4-mini";
const DEFAULT_INSTRUCTIONS: &str = "You are a concise terminal assistant. Answer directly, avoid markdown tables unless useful, and keep responses practical.";

pub(crate) struct AgentResponse {
    pub(crate) text: String,
    pub(crate) total_tokens: Option<u64>,
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

    let api_key = env::var("OPENAI_API_KEY")
        .or_else(|_| env::var("AI_API_KEY"))
        .context("Missing OPENAI_API_KEY. Set it before calling the AI helper.")?;
    let model = env::var("OPENAI_MODEL").unwrap_or_else(|_| DEFAULT_MODEL.to_string());
    let base_url =
        env::var("OPENAI_BASE_URL").unwrap_or_else(|_| "https://api.openai.com/v1".to_string());
    let instructions = env::var("OUR_CLI_INSTRUCTIONS")
        .or_else(|_| env::var("ASM_AGENT_INSTRUCTIONS"))
        .unwrap_or_else(|_| DEFAULT_INSTRUCTIONS.to_string());
    let max_output_tokens = env::var("OPENAI_MAX_OUTPUT_TOKENS")
        .unwrap_or_else(|_| "800".to_string())
        .parse::<u64>()
        .context("OPENAI_MAX_OUTPUT_TOKENS must be a positive integer.")?;

    let client = Client::new();
    let response = client
        .post(format!("{}/responses", base_url.trim_end_matches('/')))
        .bearer_auth(api_key)
        .json(&json!({
            "model": model,
            "instructions": instructions,
            "input": input,
            "max_output_tokens": max_output_tokens
        }))
        .send()
        .context("OpenAI request failed before receiving a response.")?;

    let status = response.status();
    let body: Value = response
        .json()
        .context("OpenAI response was not valid JSON.")?;

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
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn extracts_output_text_shortcut() {
        let body = json!({
            "output_text": "hello",
            "usage": { "total_tokens": 12 }
        });

        assert_eq!(extract_response_text(&body).as_deref(), Some("hello"));
    }

    #[test]
    fn extracts_response_text_from_output_messages() {
        let body = json!({
            "output": [
                {
                    "type": "message",
                    "content": [
                        { "type": "output_text", "text": "first" },
                        { "type": "output_text", "text": "second" }
                    ]
                }
            ]
        });

        assert_eq!(
            extract_response_text(&body).as_deref(),
            Some("first\nsecond")
        );
    }

    #[test]
    fn returns_none_when_response_has_no_output() {
        let body = json!({ "id": "response_without_text" });

        assert_eq!(extract_response_text(&body), None);
    }
}
