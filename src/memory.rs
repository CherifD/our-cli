use anyhow::{anyhow, Result};
use std::env;
use std::fs;
use std::io;
use std::path::PathBuf;

pub(crate) fn build_transcript(message: &str) -> Result<String> {
    let existing = read_memory()?;
    if existing.trim().is_empty() {
        Ok(format!("User: {message}"))
    } else {
        Ok(format!("{}\n\nUser: {message}", existing.trim_end()))
    }
}

pub(crate) fn read_memory() -> Result<String> {
    match fs::read_to_string(state_path()?) {
        Ok(text) => Ok(text),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(String::new()),
        Err(error) => Err(error.into()),
    }
}

pub(crate) fn save_exchange(transcript: &str, answer: &str) -> Result<()> {
    let path = state_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let text = trim_history_lines(&format!("{transcript}\n\nAssistant: {answer}\n\n"));
    fs::write(path, text)?;
    Ok(())
}

pub(crate) fn reset_memory() -> Result<()> {
    let path = state_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, "")?;
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

fn state_path() -> Result<PathBuf> {
    if let Ok(path) = env::var("OUR_CLI_STATE") {
        return Ok(PathBuf::from(path));
    }

    let config_dir =
        dirs::config_dir().ok_or_else(|| anyhow!("Could not locate user config directory."))?;
    Ok(config_dir.join("our-cli").join("conversation.txt"))
}

#[cfg(test)]
#[path = "memory_tests.rs"]
mod tests;
