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
#[path = "input_tests.rs"]
mod tests;
