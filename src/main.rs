use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand};
use reqwest::blocking::Client;
use serde_json::{json, Value};
use std::env;
use std::fs;
use std::io::{self, IsTerminal, Write};
use std::path::PathBuf;

const DEFAULT_MODEL: &str = "gpt-5.4-mini";
const DEFAULT_INSTRUCTIONS: &str = "You are a concise terminal assistant. Answer directly, avoid markdown tables unless useful, and keep responses practical.";
const DEFAULT_PROMPT_COLOR: &str = "d8ae6dfc";
const DEFAULT_ASSISTANT_COLOR: &str = "00ffff";

#[derive(Parser)]
#[command(name = "our-cli")]
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

struct AgentResponse {
    text: String,
    total_tokens: Option<u64>,
}

fn main() -> Result<()> {
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
    save_exchange(&transcript, &response.text)?;
    print_response(&response);
    Ok(())
}

fn chat() -> Result<()> {
    println!("our-cli chat. Type /exit to quit.");

    loop {
        print_prompt()?;

        let mut message = String::new();
        if io::stdin().read_line(&mut message)? == 0 {
            reset_color();
            break;
        }
        reset_color();

        let message = message.trim();
        if message.is_empty() || message == "/exit" || message == "/quit" {
            break;
        }

        let transcript = build_transcript(message)?;
        let response = ask_agent(&transcript)?;
        save_exchange(&transcript, &response.text)?;
        print_response(&response);
    }

    Ok(())
}

fn history() -> Result<()> {
    let path = state_path()?;
    match fs::read_to_string(path) {
        Ok(text) if !text.trim().is_empty() => print!("{text}"),
        _ => println!("our-cli memory is empty."),
    }
    Ok(())
}

fn reset() -> Result<()> {
    let path = state_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, "")?;
    println!("our-cli memory reset.");
    Ok(())
}

fn build_transcript(message: &str) -> Result<String> {
    let existing = read_memory()?;
    if existing.trim().is_empty() {
        Ok(format!("User: {message}"))
    } else {
        Ok(format!("{}\n\nUser: {message}", existing.trim_end()))
    }
}

fn read_memory() -> Result<String> {
    match fs::read_to_string(state_path()?) {
        Ok(text) => Ok(text),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(String::new()),
        Err(error) => Err(error.into()),
    }
}

fn save_exchange(transcript: &str, answer: &str) -> Result<()> {
    let path = state_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let text = trim_history_lines(&format!("{transcript}\n\nAssistant: {answer}\n\n"));
    fs::write(path, text)?;
    Ok(())
}

fn trim_history_lines(text: &str) -> String {
    let max_lines = env::var("OUR_CLI_MAX_HISTORY_LINES")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(240);

    trim_history_lines_to(text, max_lines)
}

fn trim_history_lines_to(text: &str, max_lines: usize) -> String {
    let lines: Vec<&str> = text.lines().collect();
    if lines.len() <= max_lines {
        return text.to_string();
    }

    let mut trimmed = lines[lines.len() - max_lines..].join("\n");
    trimmed.push('\n');
    trimmed
}

fn ask_agent(input: &str) -> Result<AgentResponse> {
    if let Ok(mock) = env::var("OUR_CLI_MOCK_RESPONSE") {
        let total_tokens = env::var("OUR_CLI_MOCK_TOTAL_TOKENS")
            .ok()
            .and_then(|value| value.parse::<u64>().ok());
        return Ok(AgentResponse {
            text: mock,
            total_tokens,
        });
    }

    let api_key = env::var("OPENAI_API_KEY")
        .or_else(|_| env::var("AI_API_KEY"))
        .context("Missing OPENAI_API_KEY. Set it before calling the AI helper.")?;
    let model = env::var("OPENAI_MODEL").unwrap_or_else(|_| DEFAULT_MODEL.to_string());
    let base_url =
        env::var("OPENAI_BASE_URL").unwrap_or_else(|_| "https://api.openai.com/v1".to_string());
    let instructions = env::var("OUR_CLI_INSTRUCTIONS")
        .or_else(|_| env::var("ASM_AGENT_INSTRUCTIONS"))
        .unwrap_or_else(|_| DEFAULT_INSTRUCTIONS.to_string());
    let max_output_tokens = env::var("OPENAI_MAX_OUTPUT_TOKENS")
        .unwrap_or_else(|_| "800".to_string())
        .parse::<u64>()
        .context("OPENAI_MAX_OUTPUT_TOKENS must be a positive integer.")?;

    let client = Client::new();
    let response = client
        .post(format!("{}/responses", base_url.trim_end_matches('/')))
        .bearer_auth(api_key)
        .json(&json!({
            "model": model,
            "instructions": instructions,
            "input": input,
            "max_output_tokens": max_output_tokens
        }))
        .send()
        .context("OpenAI request failed before receiving a response.")?;

    let status = response.status();
    let body: Value = response
        .json()
        .context("OpenAI response was not valid JSON.")?;

    if !status.is_success() {
        let message = body
            .pointer("/error/message")
            .and_then(Value::as_str)
            .or_else(|| body.get("message").and_then(Value::as_str))
            .unwrap_or("unknown error");
        return Err(anyhow!("OpenAI request failed ({status}): {message}"));
    }

    let text = extract_response_text(&body)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| anyhow!("OpenAI response did not include text output."))?;
    let total_tokens = body.pointer("/usage/total_tokens").and_then(Value::as_u64);

    Ok(AgentResponse { text, total_tokens })
}

fn extract_response_text(body: &Value) -> Option<String> {
    if let Some(text) = body.get("output_text").and_then(Value::as_str) {
        return Some(text.to_string());
    }

    let mut chunks = Vec::new();
    let output = body.get("output")?.as_array()?;
    for item in output {
        if item.get("type").and_then(Value::as_str) != Some("message") {
            continue;
        }
        for content in item.get("content").and_then(Value::as_array)? {
            if content.get("type").and_then(Value::as_str) == Some("output_text") {
                if let Some(text) = content.get("text").and_then(Value::as_str) {
                    chunks.push(text);
                }
            }
        }
    }

    Some(chunks.join("\n"))
}

fn print_response(response: &AgentResponse) {
    if use_color() {
        println!(
            "{}{}\x1b[0m",
            color_sequence("OUR_CLI_ASSISTANT_COLOR", DEFAULT_ASSISTANT_COLOR),
            response.text
        );
    } else {
        println!("{}", response.text);
    }

    if let Some(total_tokens) = response.total_tokens {
        if use_color() {
            println!("\n\x1b[2m[tokens: {total_tokens}]\x1b[0m");
        } else {
            println!("\n[tokens: {total_tokens}]");
        }
    }
}

fn print_prompt() -> Result<()> {
    if use_color() {
        print!(
            "{}> ",
            color_sequence("OUR_CLI_PROMPT_COLOR", DEFAULT_PROMPT_COLOR)
        );
    } else {
        print!("> ");
    }
    io::stdout().flush()?;
    Ok(())
}

fn reset_color() {
    if use_color() {
        print!("\x1b[0m");
        let _ = io::stdout().flush();
    }
}

fn use_color() -> bool {
    io::stdout().is_terminal()
        && env::var_os("NO_COLOR").is_none()
        && env::var("OUR_CLI_COLOR")
            .map(|value| value != "never")
            .unwrap_or(true)
}

fn color_sequence(env_name: &str, fallback: &str) -> String {
    let value = env::var(env_name).unwrap_or_else(|_| fallback.to_string());
    let (red, green, blue) = resolve_hex_color(&value, fallback);
    format!("\x1b[38;2;{red};{green};{blue}m")
}

fn resolve_hex_color(value: &str, fallback: &str) -> (u8, u8, u8) {
    parse_hex_color(value)
        .or_else(|| parse_hex_color(fallback))
        .unwrap_or((255, 255, 255))
}

fn parse_hex_color(value: &str) -> Option<(u8, u8, u8)> {
    let trimmed = value.trim().trim_start_matches('#');
    if trimmed.len() != 6 && trimmed.len() != 8 {
        return None;
    }
    let red = u8::from_str_radix(&trimmed[0..2], 16).ok()?;
    let green = u8::from_str_radix(&trimmed[2..4], 16).ok()?;
    let blue = u8::from_str_radix(&trimmed[4..6], 16).ok()?;
    Some((red, green, blue))
}

fn state_path() -> Result<PathBuf> {
    if let Ok(path) = env::var("OUR_CLI_STATE") {
        return Ok(PathBuf::from(path));
    }

    let config_dir =
        dirs::config_dir().ok_or_else(|| anyhow!("Could not locate user config directory."))?;
    Ok(config_dir.join("our-cli").join("conversation.txt"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_hex_color_with_hash() {
        assert_eq!(parse_hex_color("#b84367"), Some((184, 67, 103)));
    }

    #[test]
    fn parses_hex_color_with_alpha_ignored() {
        assert_eq!(parse_hex_color("b84367fc"), Some((184, 67, 103)));
    }

    #[test]
    fn rejects_invalid_hex_color() {
        assert_eq!(parse_hex_color("nothex"), None);
        assert_eq!(parse_hex_color("12345"), None);
    }

    #[test]
    fn resolves_invalid_color_to_fallback() {
        assert_eq!(resolve_hex_color("nothex", "00ffff"), (0, 255, 255));
    }

    #[test]
    fn extracts_output_text_shortcut() {
        let body = json!({
            "output_text": "hello",
            "usage": { "total_tokens": 12 }
        });

        assert_eq!(extract_response_text(&body).as_deref(), Some("hello"));
    }

    #[test]
    fn extracts_response_text_from_output_messages() {
        let body = json!({
            "output": [
                {
                    "type": "message",
                    "content": [
                        { "type": "output_text", "text": "first" },
                        { "type": "output_text", "text": "second" }
                    ]
                }
            ]
        });

        assert_eq!(
            extract_response_text(&body).as_deref(),
            Some("first\nsecond")
        );
    }

    #[test]
    fn returns_none_when_response_has_no_output() {
        let body = json!({ "id": "response_without_text" });

        assert_eq!(extract_response_text(&body), None);
    }

    #[test]
    fn trims_history_to_requested_lines() {
        let text = "one\ntwo\nthree\nfour\n";

        assert_eq!(trim_history_lines_to(text, 2), "three\nfour\n");
    }

    #[test]
    fn leaves_short_history_unchanged() {
        let text = "one\ntwo\n";

        assert_eq!(trim_history_lines_to(text, 4), text);
    }
}
