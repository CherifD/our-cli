use super::*;

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
