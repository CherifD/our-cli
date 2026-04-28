use crate::agent::ask_agent;
use crate::chat::editor::{MultilineHelper, CHAT_EDITOR_PROMPT};
use crate::chat::input::{is_exit_command, normalize_chat_editor_input};
use crate::chat::terminal::{discard_pending_input, PendingInputGuard};
use crate::memory::{build_transcript, save_exchange};
use crate::output::print_response;
use anyhow::{anyhow, Result};
use rustyline::error::ReadlineError;
use rustyline::history::DefaultHistory;
use rustyline::Editor;

pub(super) fn chat_with_editor() -> Result<()> {
    let mut editor: Editor<MultilineHelper, DefaultHistory> = Editor::new()?;
    editor.set_helper(Some(MultilineHelper::new()));

    loop {
        discard_pending_input()?;
        match editor.readline(CHAT_EDITOR_PROMPT) {
            Ok(input) => {
                let message = normalize_chat_editor_input(&input);
                if is_exit_command(&message) {
                    break;
                }

                let _ = editor.add_history_entry(message.as_str());
                let transcript = build_transcript(&message)?;
                let _pending_input = PendingInputGuard::new()?;
                let response = ask_agent(&transcript)?;
                save_exchange(&transcript, &response.text)?;
                print_response(&response);
            }
            Err(ReadlineError::Interrupted | ReadlineError::Eof) => break,
            Err(error) => return Err(anyhow!("Could not read chat input: {error}")),
        }
    }

    Ok(())
}
