use super::*;
use rustyline::highlight::Highlighter;

#[test]
fn editor_input_is_valid_on_return() {
    assert!(matches!(
        validate_chat_editor_input("first line"),
        ValidationResult::Valid(None)
    ));
}

#[test]
fn editor_input_is_valid_after_blank_submit() {
    assert!(matches!(
        validate_chat_editor_input("first line\n"),
        ValidationResult::Valid(None)
    ));
}

#[test]
fn editor_exit_command_is_valid_immediately() {
    assert!(matches!(
        validate_chat_editor_input("/exit"),
        ValidationResult::Valid(None)
    ));
}

#[test]
fn editor_empty_input_is_invalid() {
    assert!(matches!(
        validate_chat_editor_input(""),
        ValidationResult::Invalid(Some(_))
    ));
}

#[test]
fn editor_highlighter_colors_prompt_and_input() {
    let helper = MultilineHelper {
        input_color: Some("\x1b[38;2;1;2;3m".to_string()),
        colored_prompt: Some("\x1b[38;2;1;2;3m> \x1b[0m".to_string()),
    };

    assert_eq!(
        helper.highlight_prompt(CHAT_EDITOR_PROMPT, true),
        "\x1b[38;2;1;2;3m> \x1b[0m"
    );
    assert_eq!(helper.highlight("hello", 0), "\x1b[38;2;1;2;3mhello\x1b[0m");
}
