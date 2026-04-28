mod editor;
mod input;
mod interactive;
mod terminal;

use crate::agent::ask_agent;
use crate::constants::APP_NAME;
use crate::memory::{build_transcript, save_exchange};
use crate::output::print_response;
use anyhow::Result;
use input::{read_chat_message, ChatInput};
use interactive::chat_with_editor;
use std::io::{self, IsTerminal};

pub(crate) fn chat() -> Result<()> {
    println!("{APP_NAME} chat. Press return to send a message. Type /exit to quit.");

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
        save_exchange(&message, &response.text)?;
        print_response(&response);
    }

    Ok(())
}
