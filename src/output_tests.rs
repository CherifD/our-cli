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
