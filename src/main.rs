use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand};
use reqwest::blocking::Client;
use rustyline::completion::{Completer, Pair};
use rustyline::error::ReadlineError;
use rustyline::highlight::{CmdKind, Highlighter};
use rustyline::hint::Hinter;
use rustyline::history::DefaultHistory;
use rustyline::validate::{ValidationContext, ValidationResult, Validator};
use rustyline::{Context as RustylineContext, Editor, Helper};
use serde_json::{json, Value};
use std::borrow::Cow;
use std::env;
use std::fs;
use std::io::{self, IsTerminal, Write};
#[cfg(unix)]
use std::os::fd::AsRawFd;
use std::path::PathBuf;
#[cfg(unix)]
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
#[cfg(unix)]
use std::thread::{self, JoinHandle};
#[cfg(unix)]
use std::time::Duration;

const DEFAULT_MODEL: &str = "gpt-5.4-mini";
const DEFAULT_INSTRUCTIONS: &str = "You are a concise terminal assistant. Answer directly, avoid markdown tables unless useful, and keep responses practical.";
const DEFAULT_PROMPT_COLOR: &str = "d8ae6dfc";
const DEFAULT_ASSISTANT_COLOR: &str = "00ffff";
#[cfg(unix)]
const PENDING_INPUT_DRAIN_GRACE: Duration = Duration::from_millis(100);

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

#[derive(Debug, PartialEq, Eq)]
enum ChatInput {
    Message(String),
    Exit,
    Eof,
}

const CHAT_EDITOR_PROMPT: &str = "> ";

struct MultilineHelper {
    input_color: Option<String>,
    colored_prompt: Option<String>,
}

#[cfg(unix)]
struct PendingInputGuard {
    fd: libc::c_int,
    original: libc::termios,
    original_flags: libc::c_int,
    stop: Arc<AtomicBool>,
    drain_thread: Option<JoinHandle<()>>,
}

#[cfg(not(unix))]
struct PendingInputGuard;

impl MultilineHelper {
    fn new() -> Self {
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

#[cfg(unix)]
impl PendingInputGuard {
    fn new() -> Result<Self> {
        let stdin = io::stdin();
        let fd = stdin.as_raw_fd();
        let original_flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
        if original_flags < 0 {
            return Err(io::Error::last_os_error())
                .context("Could not inspect terminal input flags");
        }

        let mut termios = unsafe {
            let mut termios = std::mem::zeroed();
            if libc::tcgetattr(fd, &mut termios) != 0 {
                return Err(io::Error::last_os_error()).context("Could not read terminal settings");
            }
            termios
        };
        let original = termios;

        termios.c_lflag &= !(libc::ECHO | libc::ICANON | libc::IEXTEN | libc::ISIG);
        termios.c_cc[libc::VMIN] = 1;
        termios.c_cc[libc::VTIME] = 0;

        if unsafe { libc::tcsetattr(fd, libc::TCSAFLUSH, &termios) } != 0 {
            return Err(io::Error::last_os_error()).context("Could not block pending input");
        }

        if unsafe { libc::fcntl(fd, libc::F_SETFL, original_flags | libc::O_NONBLOCK) } < 0 {
            let _ = unsafe { libc::tcsetattr(fd, libc::TCSAFLUSH, &original) };
            return Err(io::Error::last_os_error()).context("Could not drain terminal input");
        }

        drain_nonblocking_input(fd)?;

        let stop = Arc::new(AtomicBool::new(false));
        let drain_stop = Arc::clone(&stop);
        let drain_thread = thread::spawn(move || {
            while !drain_stop.load(Ordering::Relaxed) {
                let _ = drain_nonblocking_input(fd);
                thread::sleep(Duration::from_millis(10));
            }
            let _ = drain_nonblocking_input(fd);
        });

        Ok(Self {
            fd,
            original,
            original_flags,
            stop,
            drain_thread: Some(drain_thread),
        })
    }
}

#[cfg(unix)]
impl Drop for PendingInputGuard {
    fn drop(&mut self) {
        thread::sleep(PENDING_INPUT_DRAIN_GRACE);
        self.stop.store(true, Ordering::Relaxed);
        if let Some(drain_thread) = self.drain_thread.take() {
            let _ = drain_thread.join();
        }
        let _ = drain_nonblocking_input(self.fd);
        unsafe {
            libc::tcflush(self.fd, libc::TCIFLUSH);
            libc::fcntl(self.fd, libc::F_SETFL, self.original_flags);
            libc::tcsetattr(self.fd, libc::TCSAFLUSH, &self.original);
        }
    }
}

#[cfg(not(unix))]
impl PendingInputGuard {
    fn new() -> Result<Self> {
        Ok(Self)
    }
}

#[cfg(unix)]
fn drain_pending_input(fd: libc::c_int) -> Result<()> {
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    if flags < 0 {
        return Err(io::Error::last_os_error()).context("Could not inspect terminal input flags");
    }

    if unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0 {
        return Err(io::Error::last_os_error()).context("Could not drain terminal input");
    }

    let drain_result = drain_nonblocking_input(fd);

    if unsafe { libc::fcntl(fd, libc::F_SETFL, flags) } < 0 {
        return Err(io::Error::last_os_error()).context("Could not restore terminal input flags");
    }

    drain_result?;

    unsafe {
        libc::tcflush(fd, libc::TCIFLUSH);
    }
    Ok(())
}

#[cfg(unix)]
fn drain_nonblocking_input(fd: libc::c_int) -> Result<()> {
    let mut buf = [0_u8; 1024];
    loop {
        let read = unsafe { libc::read(fd, buf.as_mut_ptr().cast(), buf.len()) };
        if read > 0 {
            continue;
        }

        if read == 0 {
            return Ok(());
        }

        let error = io::Error::last_os_error();
        match error.kind() {
            io::ErrorKind::WouldBlock => return Ok(()),
            io::ErrorKind::Interrupted => continue,
            _ => return Err(error).context("Could not drain terminal input"),
        }
    }
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

#[cfg(unix)]
fn discard_pending_input() -> Result<()> {
    let stdin = io::stdin();
    let fd = stdin.as_raw_fd();
    drain_pending_input(fd)?;
    unsafe {
        libc::tcflush(fd, libc::TCIFLUSH);
    }
    Ok(())
}

#[cfg(not(unix))]
fn discard_pending_input() -> Result<()> {
    Ok(())
}

fn read_chat_message<R: io::BufRead>(reader: &mut R) -> Result<ChatInput> {
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

fn normalize_chat_editor_input(input: &str) -> String {
    input
        .replace("\r\n", "\n")
        .trim_end_matches(['\r', '\n'])
        .to_string()
}

fn is_exit_command(input: &str) -> bool {
    matches!(input.trim(), "/exit" | "/quit")
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

fn print_prompt(first_line: bool) -> Result<()> {
    let marker = if first_line { "> " } else { "| " };
    if use_color() {
        print!(
            "{}{}",
            color_sequence("OUR_CLI_PROMPT_COLOR", DEFAULT_PROMPT_COLOR),
            marker
        );
    } else {
        print!("{marker}");
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
    fn editor_input_is_valid_on_return() {
        assert!(matches!(
            validate_chat_editor_input("first line"),
            ValidationResult::Valid(None)
        ));
    }

    #[test]
    fn editor_input_is_valid_after_blank_submit() {
        assert!(matches!(
            validate_chat_editor_input("first line\n"),
            ValidationResult::Valid(None)
        ));
    }

    #[test]
    fn editor_input_normalizes_trailing_submit_newline() {
        assert_eq!(
            normalize_chat_editor_input("first line\nsecond line\n"),
            "first line\nsecond line"
        );
    }

    #[test]
    fn editor_exit_command_is_valid_immediately() {
        assert!(matches!(
            validate_chat_editor_input("/exit"),
            ValidationResult::Valid(None)
        ));
    }

    #[test]
    fn editor_empty_input_is_invalid() {
        assert!(matches!(
            validate_chat_editor_input(""),
            ValidationResult::Invalid(Some(_))
        ));
    }

    #[test]
    fn editor_highlighter_colors_prompt_and_input() {
        let helper = MultilineHelper {
            input_color: Some("\x1b[38;2;1;2;3m".to_string()),
            colored_prompt: Some("\x1b[38;2;1;2;3m> \x1b[0m".to_string()),
        };

        assert_eq!(
            helper.highlight_prompt(CHAT_EDITOR_PROMPT, true),
            "\x1b[38;2;1;2;3m> \x1b[0m"
        );
        assert_eq!(helper.highlight("hello", 0), "\x1b[38;2;1;2;3mhello\x1b[0m");
    }
}
