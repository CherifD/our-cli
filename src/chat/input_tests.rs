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
fn sends_buffered_message_on_eof() {
    let input = b"line without blank submit";
    let mut reader = &input[..];

    assert_eq!(
        read_chat_message_inner(&mut reader, false).unwrap(),
        ChatInput::Message("line without blank submit".to_string())
    );
}

#[test]
fn editor_input_normalizes_trailing_submit_newline() {
    assert_eq!(
        normalize_chat_editor_input("first line\nsecond line\n"),
        "first line\nsecond line"
    );
}
