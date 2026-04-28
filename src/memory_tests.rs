use super::*;
use serde_json::Value;
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

    assert!(read_memory_file_at(&path).unwrap().exchanges.is_empty());
}

#[test]
fn saves_exchange_and_creates_parent_directory() {
    let path = temp_state_path("save").parent().unwrap().join(format!(
        "our-cli-memory-save-{}/state.txt",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(path.parent().unwrap());

    save_exchange_at(&path, "User: hello", "Assistant text").unwrap();

    let saved: Value = serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
    assert_eq!(
        saved,
        serde_json::json!({
            "version": 1,
            "exchanges": [
                {
                    "user": "User: hello",
                    "assistant": "Assistant text"
                }
            ]
        })
    );
}

#[test]
fn reset_memory_creates_empty_file() {
    let path = temp_state_path("reset");

    reset_memory_at(&path).unwrap();

    let saved: Value = serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
    assert_eq!(
        saved,
        serde_json::json!({
            "version": 1,
            "exchanges": []
        })
    );
}

#[test]
fn renders_transcript_from_exchanges() {
    let memory = MemoryFile {
        version: 1,
        exchanges: vec![
            Exchange {
                user: "old".to_string(),
                assistant: "reply".to_string(),
            },
            Exchange {
                user: "new".to_string(),
                assistant: "answer".to_string(),
            },
        ],
    };

    assert_eq!(
        render_transcript(&memory.exchanges),
        "User: old\n\nAssistant: reply\n\nUser: new\n\nAssistant: answer"
    );
}

#[test]
fn trims_history_to_requested_lines() {
    let mut memory = MemoryFile {
        version: 1,
        exchanges: vec![
            Exchange {
                user: "one".to_string(),
                assistant: "two".to_string(),
            },
            Exchange {
                user: "three".to_string(),
                assistant: "four".to_string(),
            },
        ],
    };

    trim_history_to(&mut memory, 3);

    assert_eq!(memory.exchanges.len(), 1);
    assert_eq!(memory.exchanges[0].user, "three");
}

#[test]
fn leaves_short_history_unchanged() {
    let mut memory = MemoryFile {
        version: 1,
        exchanges: vec![Exchange {
            user: "one".to_string(),
            assistant: "two".to_string(),
        }],
    };

    trim_history_to(&mut memory, 4);

    assert_eq!(memory.exchanges.len(), 1);
    assert_eq!(memory.exchanges[0].assistant, "two");
}
