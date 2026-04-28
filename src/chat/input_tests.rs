use super::*;

#[test]
fn reads_multiline_chat_message_until_blank_line() {
    let input = b"first line\nsecond line\n\n";
    let mut reader = &input[..];

    assert_eq!(
        read_chat_message_inner(&mut reader, false).unwrap(),
        ChatInput::Message("first line\nsecond line".to_string())
    );
}

#[test]
fn reads_single_line_chat_message_after_blank_submit() {
    let input = b"hello\n\n";
    let mut reader = &input[..];

    assert_eq!(
        read_chat_message_inner(&mut reader, false).unwrap(),
        ChatInput::Message("hello".to_string())
    );
}

#[test]
fn exits_chat_when_exit_is_first_line() {
    let input = b"/exit\n";
    let mut reader = &input[..];

    assert_eq!(
        read_chat_message_inner(&mut reader, false).unwrap(),
        ChatInput::Exit
    );
}

#[test]
fn exits_chat_when_quit_is_first_line_with_whitespace() {
    let input = b"  /quit  \n";
    let mut reader = &input[..];

    assert_eq!(
        read_chat_message_inner(&mut reader, false).unwrap(),
        ChatInput::Exit
    );
}

#[test]
fn ignores_blank_lines_before_first_message() {
    let input = b"\n\nhello\n\n";
    let mut reader = &input[..];

    assert_eq!(
        read_chat_message_inner(&mut reader, false).unwrap(),
        ChatInput::Message("hello".to_string())
    );
}

#[test]
fn returns_eof_when_no_message_is_buffered() {
    let input = b"";
    let mut reader = &input[..];

    assert_eq!(
        read_chat_message_inner(&mut reader, false).unwrap(),
        ChatInput::Eof
    );
}

#[test]
fn sends_buffered_message_on_eof() {
    let input = b"line without blank submit";
    let mut reader = &input[..];

    assert_eq!(
        read_chat_message_inner(&mut reader, false).unwrap(),
        ChatInput::Message("line without blank submit".to_string())
    );
}

#[test]
fn normalizes_windows_newlines() {
    assert_eq!(
        normalize_chat_editor_input("first\r\nsecond\r\n"),
        "first\nsecond"
    );
}

#[test]
fn editor_input_normalizes_trailing_submit_newline() {
    assert_eq!(
        normalize_chat_editor_input("first line\nsecond line\n"),
        "first line\nsecond line"
    );
}

#[test]
fn recognizes_exit_commands_after_trimming() {
    assert!(is_exit_command(" /exit "));
    assert!(is_exit_command("\t/quit\n"));
    assert!(!is_exit_command("/exit now"));
}
