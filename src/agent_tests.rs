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
