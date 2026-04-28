use super::*;
use reqwest::StatusCode;
use serde_json::json;
use std::collections::HashMap;

fn test_config() -> AgentConfig {
    AgentConfig {
        api_key: "key".to_string(),
        model: "model".to_string(),
        base_url: "https://example.test/v1/".to_string(),
        instructions: "instructions".to_string(),
        max_output_tokens: 123,
    }
}

fn config_from(values: &[(&str, &str)]) -> Result<AgentConfig> {
    let values: HashMap<&str, &str> = values.iter().copied().collect();
    AgentConfig::from_env_vars(|name| values.get(name).map(|value| (*value).to_string()))
}

#[test]
fn builds_config_with_defaults_from_primary_api_key() {
    let config = config_from(&[("OPENAI_API_KEY", "key")]).unwrap();

    assert_eq!(config.api_key, "key");
    assert_eq!(config.model, DEFAULT_MODEL);
    assert_eq!(config.base_url, "https://api.openai.com/v1");
    assert_eq!(config.instructions, DEFAULT_INSTRUCTIONS);
    assert_eq!(config.max_output_tokens, 800);
}

#[test]
fn builds_config_from_overrides_and_alternate_api_key() {
    let config = config_from(&[
        ("AI_API_KEY", "fallback-key"),
        ("OPENAI_MODEL", "custom-model"),
        ("OPENAI_BASE_URL", "https://example.test"),
        ("ASM_AGENT_INSTRUCTIONS", "legacy instructions"),
        ("OPENAI_MAX_OUTPUT_TOKENS", "99"),
    ])
    .unwrap();

    assert_eq!(config.api_key, "fallback-key");
    assert_eq!(config.model, "custom-model");
    assert_eq!(config.base_url, "https://example.test");
    assert_eq!(config.instructions, "legacy instructions");
    assert_eq!(config.max_output_tokens, 99);
}

#[test]
fn prefers_our_cli_instructions_over_legacy_instructions() {
    let config = config_from(&[
        ("OPENAI_API_KEY", "key"),
        ("OUR_CLI_INSTRUCTIONS", "new instructions"),
        ("ASM_AGENT_INSTRUCTIONS", "old instructions"),
    ])
    .unwrap();

    assert_eq!(config.instructions, "new instructions");
}

#[test]
fn rejects_config_without_api_key() {
    let error = config_from(&[]).unwrap_err();

    assert!(error
        .to_string()
        .contains("Missing OPENAI_API_KEY. Set it before calling the AI helper."));
}

#[test]
fn rejects_invalid_max_output_tokens() {
    let error = config_from(&[
        ("OPENAI_API_KEY", "key"),
        ("OPENAI_MAX_OUTPUT_TOKENS", "not-a-number"),
    ])
    .unwrap_err();

    assert!(error
        .to_string()
        .contains("OPENAI_MAX_OUTPUT_TOKENS must be a positive integer."));
}

#[test]
fn trims_base_url_when_building_endpoint() {
    assert_eq!(
        test_config().endpoint_url(),
        "https://example.test/v1/responses"
    );
}

#[test]
fn builds_request_body_from_config() {
    assert_eq!(
        test_config().request_body("hello"),
        json!({
            "model": "model",
            "instructions": "instructions",
            "input": "hello",
            "max_output_tokens": 123
        })
    );
}

#[test]
fn parses_successful_agent_response() {
    let response = parse_agent_response(
        StatusCode::OK,
        json!({
            "output_text": "hello",
            "usage": { "total_tokens": 10 }
        }),
    )
    .unwrap();

    assert_eq!(response.text, "hello");
    assert_eq!(response.total_tokens, Some(10));
}

#[test]
fn rejects_successful_response_without_text() {
    let error = parse_agent_response(StatusCode::OK, json!({ "id": "missing_text" })).unwrap_err();

    assert!(error
        .to_string()
        .contains("OpenAI response did not include text output."));
}

#[test]
fn rejects_successful_response_with_blank_text() {
    let error = parse_agent_response(StatusCode::OK, json!({ "output_text": "   " })).unwrap_err();

    assert!(error
        .to_string()
        .contains("OpenAI response did not include text output."));
}

#[test]
fn reports_api_error_message_from_error_pointer() {
    let error = parse_agent_response(
        StatusCode::BAD_REQUEST,
        json!({ "error": { "message": "bad request" } }),
    )
    .unwrap_err();

    assert!(error.to_string().contains("bad request"));
}

#[test]
fn reports_api_error_message_from_top_level_message() {
    let error = parse_agent_response(StatusCode::BAD_REQUEST, json!({ "message": "top level" }))
        .unwrap_err();

    assert!(error.to_string().contains("top level"));
}

#[test]
fn reports_unknown_api_error_when_message_is_absent() {
    let error = parse_agent_response(StatusCode::BAD_REQUEST, json!({})).unwrap_err();

    assert!(error.to_string().contains("unknown error"));
}

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
fn skips_non_message_outputs_and_non_text_content() {
    let body = json!({
        "output": [
            {
                "type": "tool_call",
                "content": [
                    { "type": "output_text", "text": "ignored" }
                ]
            },
            {
                "type": "message",
                "content": [
                    { "type": "summary_text", "text": "ignored" },
                    { "type": "output_text", "text": "kept" }
                ]
            }
        ]
    });

    assert_eq!(extract_response_text(&body).as_deref(), Some("kept"));
}

#[test]
fn joins_empty_output_chunks_when_message_has_no_text() {
    let body = json!({
        "output": [
            {
                "type": "message",
                "content": [
                    { "type": "summary_text", "text": "ignored" }
                ]
            }
        ]
    });

    assert_eq!(extract_response_text(&body).as_deref(), Some(""));
}

#[test]
fn returns_none_when_response_has_no_output() {
    let body = json!({ "id": "response_without_text" });

    assert_eq!(extract_response_text(&body), None);
}

#[test]
fn returns_none_when_output_is_not_an_array() {
    let body = json!({ "output": "not an array" });

    assert_eq!(extract_response_text(&body), None);
}
