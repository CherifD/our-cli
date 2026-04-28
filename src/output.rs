use crate::agent::AgentResponse;
use anyhow::Result;
use std::env;
use std::io::{self, IsTerminal, Write};

pub(crate) const DEFAULT_PROMPT_COLOR: &str = "d8ae6dfc";
const DEFAULT_ASSISTANT_COLOR: &str = "00ffff";

pub(crate) fn print_response(response: &AgentResponse) {
    print!("{}", format_response(response, use_color()));
}

pub(crate) fn print_prompt(first_line: bool) -> Result<()> {
    print!("{}", format_prompt(first_line, use_color()));
    io::stdout().flush()?;
    Ok(())
}

pub(crate) fn reset_color() {
    if use_color() {
        print!("\x1b[0m");
        let _ = io::stdout().flush();
    }
}

pub(crate) fn use_color() -> bool {
    io::stdout().is_terminal()
        && env::var_os("NO_COLOR").is_none()
        && env::var("OUR_CLI_COLOR")
            .map(|value| value != "never")
            .unwrap_or(true)
}

pub(crate) fn color_sequence(env_name: &str, fallback: &str) -> String {
    let value = env::var(env_name).unwrap_or_else(|_| fallback.to_string());
    let (red, green, blue) = resolve_hex_color(&value, fallback);
    format!("\x1b[38;2;{red};{green};{blue}m")
}

fn format_response(response: &AgentResponse, color: bool) -> String {
    let mut output = if color {
        format!(
            "{}{}\x1b[0m\n",
            color_sequence("OUR_CLI_ASSISTANT_COLOR", DEFAULT_ASSISTANT_COLOR),
            response.text
        )
    } else {
        format!("{}\n", response.text)
    };

    if let Some(total_tokens) = response.total_tokens {
        if color {
            output.push_str(&format!("\n\x1b[2m[tokens: {total_tokens}]\x1b[0m\n"));
        } else {
            output.push_str(&format!("\n[tokens: {total_tokens}]\n"));
        }
    }

    output
}

fn format_prompt(first_line: bool, color: bool) -> String {
    let marker = if first_line { "> " } else { "| " };
    if color {
        format!(
            "{}{}",
            color_sequence("OUR_CLI_PROMPT_COLOR", DEFAULT_PROMPT_COLOR),
            marker
        )
    } else {
        marker.to_string()
    }
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

#[cfg(test)]
#[path = "output_tests.rs"]
mod tests;
