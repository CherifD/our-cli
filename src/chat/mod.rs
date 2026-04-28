mod editor;
mod input;
mod terminal;

use crate::agent::ask_agent;
use crate::memory::{build_transcript, save_exchange};
use crate::output::print_response;
use anyhow::{anyhow, Result};
use editor::{MultilineHelper, CHAT_EDITOR_PROMPT};
use input::{is_exit_command, normalize_chat_editor_input, read_chat_message, ChatInput};
use rustyline::error::ReadlineError;
use rustyline::history::DefaultHistory;
use rustyline::Editor;
use std::io::{self, IsTerminal};
use terminal::{discard_pending_input, PendingInputGuard};

pub(crate) fn chat() -> Result<()> {
    println!("our-cli chat. Press return to send a message. Type /exit to quit.");

    if io::stdin().is_terminal() {
        return chat_with_editor();
    }

    let stdin = io::stdin();
    let mut reader = stdin.lock();

    loop {
        let message = match read_chat_message(&mut reader)? {
            ChatInput::Message(message) => message,
            ChatInput::Exit | ChatInput::Eof => break,
        };

        let transcript = build_transcript(&message)?;
        let response = ask_agent(&transcript)?;
        save_exchange(&transcript, &response.text)?;
        print_response(&response);
    }

    Ok(())
}

fn chat_with_editor() -> Result<()> {
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
