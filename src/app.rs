use crate::agent::ask_agent;
use crate::chat::chat;
use crate::constants::APP_NAME;
use crate::memory::{build_transcript, read_memory, reset_memory, save_exchange};
use crate::output::print_response;
use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = APP_NAME)]
#[command(about = "A Rust CLI twin of asm-agent")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    prompt: Vec<String>,
}

#[derive(Subcommand)]
enum Command {
    Chat,
    History,
    Reset,
}

pub(crate) fn run() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Command::Chat) => chat(),
        Some(Command::History) => history(),
        Some(Command::Reset) => reset(),
        None => prompt(cli.prompt.join(" ").trim().to_string()),
    }
}

fn prompt(message: String) -> Result<()> {
    if message.is_empty() {
        return Err(anyhow!(
            "No prompt provided. Try: our-cli \"explain ownership simply\""
        ));
    }

    let transcript = build_transcript(&message)?;
    let response = ask_agent(&transcript)?;
    save_exchange(&message, &response.text)?;
    print_response(&response);
    Ok(())
}

fn history() -> Result<()> {
    let text = read_memory()?;
    if text.trim().is_empty() {
        println!("{APP_NAME} memory is empty.");
    } else {
        print!("{text}");
    }
    Ok(())
}

fn reset() -> Result<()> {
    reset_memory()?;
    println!("{APP_NAME} memory reset.");
    Ok(())
}
