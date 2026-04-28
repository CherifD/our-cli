use super::*;
use rustyline::completion::Completer;
use rustyline::highlight::Highlighter;
use rustyline::history::DefaultHistory;

#[test]
fn editor_helper_defaults_to_plain_when_stdout_is_not_terminal() {
    let helper = MultilineHelper::new();

    assert!(helper.input_color.is_none());
    assert!(helper.colored_prompt.is_none());
}

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

#[test]
fn editor_highlighter_leaves_prompt_and_input_plain_without_color() {
    let helper = MultilineHelper {
        input_color: None,
        colored_prompt: None,
    };

    assert_eq!(helper.highlight_prompt(CHAT_EDITOR_PROMPT, true), "> ");
    assert_eq!(helper.highlight_prompt("custom", false), "custom");
    assert_eq!(helper.highlight("hello", 0), "hello");
    assert!(!helper.highlight_char("hello", 0, CmdKind::MoveCursor));
}

#[test]
fn editor_highlighter_requests_repaint_when_color_is_enabled() {
    let helper = MultilineHelper {
        input_color: Some("\x1b[38;2;1;2;3m".to_string()),
        colored_prompt: None,
    };

    assert!(helper.highlight_char("hello", 0, CmdKind::MoveCursor));
}

#[test]
fn editor_completer_returns_no_candidates() {
    let helper = MultilineHelper {
        input_color: None,
        colored_prompt: None,
    };
    let history = DefaultHistory::new();
    let ctx = rustyline::Context::new(&history);

    let (start, candidates) = helper.complete("hello", 2, &ctx).unwrap();
    assert_eq!(start, 0);
    assert!(candidates.is_empty());
}
