use super::*;
use crate::agent::AgentResponse;

#[test]
fn parses_hex_color_with_hash() {
    assert_eq!(parse_hex_color("#b84367"), Some((184, 67, 103)));
}

#[test]
fn parses_hex_color_with_alpha_ignored() {
    assert_eq!(parse_hex_color("b84367fc"), Some((184, 67, 103)));
}

#[test]
fn rejects_invalid_hex_color() {
    assert_eq!(parse_hex_color("nothex"), None);
    assert_eq!(parse_hex_color("12345"), None);
}

#[test]
fn resolves_invalid_color_to_fallback() {
    assert_eq!(resolve_hex_color("nothex", "00ffff"), (0, 255, 255));
}

#[test]
fn defaults_invalid_color_and_fallback_to_white() {
    assert_eq!(resolve_hex_color("nothex", "also-bad"), (255, 255, 255));
}

#[test]
fn color_sequence_uses_fallback_for_unset_env() {
    assert_eq!(
        color_sequence("OUR_CLI_TEST_COLOR_UNSET", "010203"),
        "\x1b[38;2;1;2;3m"
    );
}

#[test]
fn stdout_is_not_colored_in_tests() {
    assert!(!use_color());
}

#[test]
fn print_response_handles_plain_output_without_tokens() {
    print_response(&AgentResponse {
        text: "hello".to_string(),
        total_tokens: None,
    });
}

#[test]
fn formats_plain_response_without_tokens() {
    assert_eq!(
        format_response(
            &AgentResponse {
                text: "hello".to_string(),
                total_tokens: None,
            },
            false
        ),
        "hello\n"
    );
}

#[test]
fn print_response_handles_plain_output_with_tokens() {
    print_response(&AgentResponse {
        text: "hello".to_string(),
        total_tokens: Some(12),
    });
}

#[test]
fn formats_plain_response_with_tokens() {
    assert_eq!(
        format_response(
            &AgentResponse {
                text: "hello".to_string(),
                total_tokens: Some(12),
            },
            false
        ),
        "hello\n\n[tokens: 12]\n"
    );
}

#[test]
fn formats_colored_response_with_tokens() {
    assert_eq!(
        format_response(
            &AgentResponse {
                text: "hello".to_string(),
                total_tokens: Some(12),
            },
            true
        ),
        "\x1b[38;2;0;255;255mhello\x1b[0m\n\n\x1b[2m[tokens: 12]\x1b[0m\n"
    );
}

#[test]
fn print_prompt_handles_first_and_continuation_lines() {
    print_prompt(true).unwrap();
    print_prompt(false).unwrap();
}

#[test]
fn formats_plain_prompts() {
    assert_eq!(format_prompt(true, false), "> ");
    assert_eq!(format_prompt(false, false), "| ");
}

#[test]
fn formats_colored_prompt() {
    assert_eq!(format_prompt(true, true), "\x1b[38;2;216;174;109m> ");
}

#[test]
fn reset_color_noops_when_color_is_disabled() {
    reset_color();
}
