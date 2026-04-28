use crate::constants::{APP_NAME, ENV_OUR_CLI_MAX_HISTORY_LINES, ENV_OUR_CLI_STATE};
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

const USER_PREFIX: &str = "User:";
const ASSISTANT_PREFIX: &str = "Assistant:";
const STATE_FILE_NAME: &str = "conversation.json";
const MEMORY_VERSION: u8 = 1;

#[derive(Debug, Default, Deserialize, Serialize)]
struct MemoryFile {
    version: u8,
    exchanges: Vec<Exchange>,
}

#[derive(Debug, Deserialize, Serialize)]
struct Exchange {
    user: String,
    assistant: String,
}

pub(crate) fn build_transcript(message: &str) -> Result<String> {
    let existing = read_memory()?;
    Ok(build_transcript_from_memory(&existing, message))
}

fn build_transcript_from_memory(existing: &str, message: &str) -> String {
    if existing.trim().is_empty() {
        format!("{USER_PREFIX} {message}")
    } else {
        format!("{}\n\n{USER_PREFIX} {message}", existing.trim_end())
    }
}

pub(crate) fn read_memory() -> Result<String> {
    let memory = read_memory_file_at(&state_path()?)?;
    Ok(render_transcript(&memory.exchanges))
}

fn read_memory_file_at(path: &Path) -> Result<MemoryFile> {
    match fs::read_to_string(path) {
        Ok(text) if text.trim().is_empty() => Ok(empty_memory()),
        Ok(text) => Ok(serde_json::from_str(&text)?),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(empty_memory()),
        Err(error) => Err(error.into()),
    }
}

pub(crate) fn save_exchange(message: &str, answer: &str) -> Result<()> {
    let path = state_path()?;
    save_exchange_at(&path, message, answer)
}

fn save_exchange_at(path: &Path, message: &str, answer: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut memory = read_memory_file_at(path)?;
    memory.exchanges.push(Exchange {
        user: message.to_string(),
        assistant: answer.to_string(),
    });
    trim_history(&mut memory);
    write_memory_file_at(path, &memory)?;
    Ok(())
}

pub(crate) fn reset_memory() -> Result<()> {
    let path = state_path()?;
    reset_memory_at(&path)
}

fn reset_memory_at(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    write_memory_file_at(path, &empty_memory())?;
    Ok(())
}

fn trim_history(memory: &mut MemoryFile) {
    let max_lines = env::var(ENV_OUR_CLI_MAX_HISTORY_LINES)
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(240);

    trim_history_to(memory, max_lines);
}

fn trim_history_to(memory: &mut MemoryFile, max_lines: usize) {
    while !memory.exchanges.is_empty() && transcript_line_count(&memory.exchanges) > max_lines {
        memory.exchanges.remove(0);
    }
}

fn transcript_line_count(exchanges: &[Exchange]) -> usize {
    render_transcript(exchanges).lines().count()
}

fn empty_memory() -> MemoryFile {
    MemoryFile {
        version: MEMORY_VERSION,
        exchanges: Vec::new(),
    }
}

fn write_memory_file_at(path: &Path, memory: &MemoryFile) -> Result<()> {
    let text = serde_json::to_string_pretty(memory)?;
    fs::write(path, format!("{text}\n"))?;
    Ok(())
}

fn render_transcript(exchanges: &[Exchange]) -> String {
    let mut parts = Vec::new();
    for exchange in exchanges {
        parts.push(format!("{USER_PREFIX} {}", exchange.user));
        parts.push(format!("{ASSISTANT_PREFIX} {}", exchange.assistant));
    }
    parts.join("\n\n")
}

fn state_path() -> Result<PathBuf> {
    if let Ok(path) = env::var(ENV_OUR_CLI_STATE) {
        return Ok(PathBuf::from(path));
    }

    let config_dir =
        dirs::config_dir().ok_or_else(|| anyhow!("Could not locate user config directory."))?;
    Ok(config_dir.join(APP_NAME).join(STATE_FILE_NAME))
}

#[cfg(test)]
#[path = "memory_tests.rs"]
mod tests;
