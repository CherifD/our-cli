use crate::output::{print_prompt, reset_color};
use anyhow::Result;
use std::io;

#[derive(Debug, PartialEq, Eq)]
pub(super) enum ChatInput {
    Message(String),
    Exit,
    Eof,
}

pub(super) fn read_chat_message<R: io::BufRead>(reader: &mut R) -> Result<ChatInput> {
    read_chat_message_inner(reader, true)
}

fn read_chat_message_inner<R: io::BufRead>(reader: &mut R, show_prompt: bool) -> Result<ChatInput> {
    let mut lines = Vec::new();

    loop {
        if show_prompt {
            print_prompt(lines.is_empty())?;
        }

        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 {
            if show_prompt {
                reset_color();
            }
            return if lines.is_empty() {
                Ok(ChatInput::Eof)
            } else {
                Ok(ChatInput::Message(lines.join("\n")))
            };
        }

        if show_prompt {
            reset_color();
        }

        let line = line.trim_end_matches(['\r', '\n']);
        if lines.is_empty() && is_exit_command(line) {
            return Ok(ChatInput::Exit);
        }

        if line.is_empty() {
            if lines.is_empty() {
                continue;
            }
            return Ok(ChatInput::Message(lines.join("\n")));
        }

        lines.push(line.to_string());
    }
}

pub(super) fn normalize_chat_editor_input(input: &str) -> String {
    input
        .replace("\r\n", "\n")
        .trim_end_matches(['\r', '\n'])
        .to_string()
}

pub(super) fn is_exit_command(input: &str) -> bool {
    matches!(input.trim(), "/exit" | "/quit")
}

#[cfg(test)]
mod tests {
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
}
