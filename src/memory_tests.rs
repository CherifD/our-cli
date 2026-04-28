use super::*;
use std::fs;
use std::path::PathBuf;

fn temp_state_path(name: &str) -> PathBuf {
    let path =
        std::env::temp_dir().join(format!("our-cli-memory-{name}-{}.txt", std::process::id()));
    let _ = fs::remove_file(&path);
    path
}

#[test]
fn builds_transcript_without_existing_memory() {
    assert_eq!(build_transcript_from_memory("", "hello"), "User: hello");
}

#[test]
fn builds_transcript_after_existing_memory() {
    assert_eq!(
        build_transcript_from_memory("User: old\n\nAssistant: reply\n\n", "new"),
        "User: old\n\nAssistant: reply\n\nUser: new"
    );
}

#[test]
fn reads_missing_memory_as_empty() {
    let path = temp_state_path("missing");

    assert_eq!(read_memory_at(&path).unwrap(), "");
}

#[test]
fn saves_exchange_and_creates_parent_directory() {
    let path = temp_state_path("save").parent().unwrap().join(format!(
        "our-cli-memory-save-{}/state.txt",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(path.parent().unwrap());

    save_exchange_at(&path, "User: hello", "Assistant text").unwrap();

    assert_eq!(
        fs::read_to_string(path).unwrap(),
        "User: hello\n\nAssistant: Assistant text\n\n"
    );
}

#[test]
fn reset_memory_creates_empty_file() {
    let path = temp_state_path("reset");

    reset_memory_at(&path).unwrap();

    assert_eq!(fs::read_to_string(path).unwrap(), "");
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
