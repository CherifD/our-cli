use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_our-cli"))
}

fn state_path(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("our-cli-{name}-{}.txt", std::process::id()));
    let _ = fs::remove_file(&path);
    path
}

fn assert_success(output: &std::process::Output) {
    assert!(
        output.status.success(),
        "expected success\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn reset_then_history_reports_empty_memory() {
    let state = state_path("empty-history");

    let reset = bin()
        .arg("reset")
        .env("OUR_CLI_STATE", &state)
        .output()
        .unwrap();
    assert_success(&reset);
    assert_eq!(
        String::from_utf8_lossy(&reset.stdout),
        "our-cli memory reset.\n"
    );

    let history = bin()
        .arg("history")
        .env("OUR_CLI_STATE", &state)
        .output()
        .unwrap();
    assert_success(&history);
    assert_eq!(
        String::from_utf8_lossy(&history.stdout),
        "our-cli memory is empty.\n"
    );
}

#[test]
fn prompt_uses_mock_response_and_saves_history() {
    let state = state_path("prompt-history");

    let prompt = bin()
        .arg("hello")
        .env("OUR_CLI_STATE", &state)
        .env("OUR_CLI_COLOR", "never")
        .env("OUR_CLI_MOCK_RESPONSE", "offline helper ok")
        .env("OUR_CLI_MOCK_TOTAL_TOKENS", "42")
        .output()
        .unwrap();
    assert_success(&prompt);

    let stdout = String::from_utf8_lossy(&prompt.stdout);
    assert!(stdout.contains("offline helper ok"));
    assert!(stdout.contains("[tokens: 42]"));

    let history = bin()
        .arg("history")
        .env("OUR_CLI_STATE", &state)
        .output()
        .unwrap();
    assert_success(&history);

    let stdout = String::from_utf8_lossy(&history.stdout);
    assert!(stdout.contains("User: hello"));
    assert!(stdout.contains("Assistant: offline helper ok"));
}

#[test]
fn missing_prompt_returns_error() {
    let output = bin().output().unwrap();

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr)
        .contains("No prompt provided. Try: our-cli \"explain ownership simply\""));
}

#[test]
fn chat_reads_piped_messages_until_exit() {
    let state = state_path("piped-chat");
    let mut child = bin()
        .arg("chat")
        .env("OUR_CLI_STATE", &state)
        .env("OUR_CLI_COLOR", "never")
        .env("OUR_CLI_MOCK_RESPONSE", "chat response")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(b"hello from chat\n\n/exit\n")
        .unwrap();

    let output = child.wait_with_output().unwrap();
    assert_success(&output);

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("our-cli chat. Press return to send a message."));
    assert!(stdout.contains("chat response"));

    let history = fs::read_to_string(&state).unwrap();
    assert!(history.contains("User: hello from chat"));
    assert!(history.contains("Assistant: chat response"));
}

#[test]
fn chat_exits_on_empty_stdin() {
    let state = state_path("empty-chat");
    let output = bin()
        .arg("chat")
        .env("OUR_CLI_STATE", &state)
        .stdin(Stdio::null())
        .output()
        .unwrap();

    assert_success(&output);
    assert!(String::from_utf8_lossy(&output.stdout)
        .contains("our-cli chat. Press return to send a message."));
    assert!(!state.exists());
}
