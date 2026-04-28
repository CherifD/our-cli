use crate::chat::input::normalize_chat_editor_input;
use crate::output::{color_sequence, use_color, DEFAULT_PROMPT_COLOR};
use rustyline::completion::{Completer, Pair};
use rustyline::highlight::{CmdKind, Highlighter};
use rustyline::hint::Hinter;
use rustyline::validate::{ValidationContext, ValidationResult, Validator};
use rustyline::{Context as RustylineContext, Helper};
use std::borrow::Cow;

pub(super) const CHAT_EDITOR_PROMPT: &str = "> ";

pub(super) struct MultilineHelper {
    input_color: Option<String>,
    colored_prompt: Option<String>,
}

impl MultilineHelper {
    pub(super) fn new() -> Self {
        if use_color() {
            let input_color = color_sequence("OUR_CLI_PROMPT_COLOR", DEFAULT_PROMPT_COLOR);
            let colored_prompt = format!("{input_color}{CHAT_EDITOR_PROMPT}\x1b[0m");
            Self {
                input_color: Some(input_color),
                colored_prompt: Some(colored_prompt),
            }
        } else {
            Self {
                input_color: None,
                colored_prompt: None,
            }
        }
    }
}

impl Helper for MultilineHelper {}

impl Completer for MultilineHelper {
    type Candidate = Pair;

    fn complete(
        &self,
        _line: &str,
        _pos: usize,
        _ctx: &RustylineContext<'_>,
    ) -> rustyline::Result<(usize, Vec<Self::Candidate>)> {
        Ok((0, Vec::new()))
    }
}

impl Hinter for MultilineHelper {
    type Hint = String;
}

impl Highlighter for MultilineHelper {
    fn highlight_prompt<'b, 's: 'b, 'p: 'b>(
        &'s self,
        prompt: &'p str,
        default: bool,
    ) -> Cow<'b, str> {
        if default {
            if let Some(colored_prompt) = &self.colored_prompt {
                return Cow::Borrowed(colored_prompt.as_str());
            }
        }

        Cow::Borrowed(prompt)
    }

    fn highlight<'l>(&self, line: &'l str, _pos: usize) -> Cow<'l, str> {
        if let Some(input_color) = &self.input_color {
            Cow::Owned(format!("{input_color}{line}\x1b[0m"))
        } else {
            Cow::Borrowed(line)
        }
    }

    fn highlight_char(&self, _line: &str, _pos: usize, _kind: CmdKind) -> bool {
        self.input_color.is_some()
    }
}

impl Validator for MultilineHelper {
    fn validate(&self, ctx: &mut ValidationContext<'_>) -> rustyline::Result<ValidationResult> {
        Ok(validate_chat_editor_input(ctx.input()))
    }
}

fn validate_chat_editor_input(input: &str) -> ValidationResult {
    let normalized = normalize_chat_editor_input(input);

    if normalized.trim().is_empty() {
        ValidationResult::Invalid(Some(
            "Enter a message, or type /exit to quit.\n".to_string(),
        ))
    } else {
        ValidationResult::Valid(None)
    }
}

#[cfg(test)]
#[path = "editor_tests.rs"]
mod tests;
