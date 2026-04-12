use crate::app::Basie64App;
use crate::core::convert::Format;
use base64::{engine::general_purpose, Engine as _};
use regex::Regex;
use std::sync::LazyLock;

// Detection priority: Percent → Hex → Base32 → Base58 → Base64 (existing)
//
// Ordering rationale:
// - Percent-encoded first: `%XX` sequences are unique and never appear in other formats.
// - Hex before Base32/Base58: short uppercase hex (e.g. "ABCDEF12") would otherwise match
//   Base32, so we check Hex first. Hex requires even-length all-[0-9a-fA-F] content.
// - Base32 before Base58: Base32 is a strict subset of the Base58 alphabet; the length
//   constraint (multiple of 8) and uppercase-only rule disambiguate them.
// - Base64 last: existing behaviour unchanged.

static HEX_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^[0-9a-f\s]+$").expect("static regex"));

static BASE32_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[A-Z2-7]+=*$").expect("static regex"));

static BASE58_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[1-9A-HJ-NP-Za-km-z]+$").expect("static regex"));

static PERCENT_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"%[0-9a-fA-F]{2}").expect("static regex"));

fn is_percent_encoded_input(input: &str) -> bool {
    if !PERCENT_RE.is_match(input) || input.chars().any(|c| c.is_whitespace()) {
        return false;
    }

    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            if i + 2 >= bytes.len()
                || !bytes[i + 1].is_ascii_hexdigit()
                || !bytes[i + 2].is_ascii_hexdigit()
            {
                return false;
            }
            i += 3;
        } else {
            i += 1;
        }
    }

    true
}

fn is_base58_candidate(input: &str) -> bool {
    if !BASE58_RE.is_match(input) || input.len() < 8 {
        return false;
    }

    let has_digit = input.chars().any(|c| c.is_ascii_digit());
    let has_upper = input.chars().any(|c| c.is_ascii_uppercase());
    let has_lower = input.chars().any(|c| c.is_ascii_lowercase());

    if !has_digit && !(has_upper && has_lower) {
        return false;
    }

    bs58::decode(input)
        .into_vec()
        .map(|decoded| !decoded.is_empty())
        .unwrap_or(false)
}

pub fn run_detection(app: &mut Basie64App) {
    if app.input == app.last_input {
        return;
    }
    app.last_input = app.input.clone();
    app.show_banner = false;
    app.show_convert_banner = false;
    app.detected_format = None;
    app.mixed_matches.clear();

    let trimmed = app.input.trim();
    if trimmed.is_empty() {
        return;
    }

    let detected_non_b64 = if is_percent_encoded_input(trimmed) {
        Some(Format::PercentEncoded)
    } else {
        let hex_stripped: String = trimmed.chars().filter(|c| !c.is_whitespace()).collect();
        if hex_stripped.len() >= 8
            && hex_stripped.len().is_multiple_of(2)
            && HEX_RE.is_match(trimmed)
        {
            Some(Format::Hex)
        } else if BASE32_RE.is_match(trimmed)
            && trimmed.len() >= 8
            && trimmed.len().is_multiple_of(8)
        {
            Some(Format::Base32)
        } else if is_base58_candidate(trimmed) {
            Some(Format::Base58)
        } else {
            None
        }
    };

    if let Some(format) = detected_non_b64 {
        app.detected_format = Some(format);
        app.show_convert_banner = true;
    }

    // ── Base64 (existing behaviour) ─────────────────────────────────────────
    let is_plain_b64 = app.base64_regex.is_match(trimmed)
        && trimmed.len().is_multiple_of(4)
        && !trimmed.contains(' ');

    if detected_non_b64.is_none() && is_plain_b64 && general_purpose::STANDARD.decode(trimmed).is_ok()
    {
        app.show_banner = true;
        app.banner_message = "Looks like valid Base64!".to_string();
        app.banner_fade_start = Some(app.now);
        return;
    }

    for mat in app.base64_regex.find_iter(trimmed) {
        let matched_str = mat.as_str();
        if general_purpose::STANDARD.decode(matched_str).is_ok() {
            app.mixed_matches.push(matched_str.to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use regex::Regex;

    fn b64_regex() -> Regex {
        Regex::new(r"(?x) (?:[A-Za-z0-9+/]{4}){4,} (?:[A-Za-z0-9+/]{2}== | [A-Za-z0-9+/]{3}=)?")
            .unwrap()
    }

    #[test]
    fn regex_matches_valid() {
        let r = b64_regex();
        assert!(r.is_match("SGVsbG8sIHdvcmxkIQ=="));
    }

    #[test]
    fn regex_finds_mixed_content() {
        let r = b64_regex();
        let log =
            "Error at line 42: data=SGVsbG8sIHdvcmxkIQ== status=fail fallback=YW5vdGhlciBzdHJpbmc=";
        let matches: Vec<&str> = r.find_iter(log).map(|m| m.as_str()).collect();
        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0], "SGVsbG8sIHdvcmxkIQ==");
        assert_eq!(matches[1], "YW5vdGhlciBzdHJpbmc=");
    }

    #[test]
    fn detect_hex() {
        assert!(HEX_RE.is_match("48656c6c6f"));
        let stripped = "48656c6c6f";
        assert!(stripped.len() >= 8 && stripped.len().is_multiple_of(2));
    }

    #[test]
    fn detect_hex_with_spaces() {
        let input = "48 65 6c 6c 6f";
        let stripped: String = input.chars().filter(|c| !c.is_whitespace()).collect();
        assert!(HEX_RE.is_match(input));
        assert!(stripped.len() >= 8 && stripped.len().is_multiple_of(2));
    }

    #[test]
    fn detect_base32() {
        let input = "JBSWY3DP"; // Base32 of "Hello"
        assert!(BASE32_RE.is_match(input));
        assert!(input.len().is_multiple_of(8) && input.len() >= 8);
    }

    #[test]
    fn detect_base32_with_padding() {
        // 1 byte → "MY======" (2 content chars + 6 padding = 8 total)
        let input = "MY======";
        assert!(BASE32_RE.is_match(input));
        assert_eq!(input.len(), 8);
    }

    #[test]
    fn detect_base58() {
        // Base58 of "Hello" using bs58 alphabet
        let input = bs58::encode(b"Hello World!").into_string();
        assert!(BASE58_RE.is_match(&input));
        assert!(input.len() >= 8);
    }

    #[test]
    fn detect_percent() {
        assert!(is_percent_encoded_input("Hello%20World"));
        assert!(is_percent_encoded_input("%48%65%6c%6c%6f"));
        assert!(!is_percent_encoded_input("no percent here"));
        assert!(!is_percent_encoded_input("note=foo%20bar token=SGVsbG8="));
    }

    #[test]
    fn base64_not_confused_with_hex() {
        // Valid base64 that contains lowercase — should NOT trigger hex detection
        let b64 = "SGVsbG8=";
        // 'S', 'G', etc. include chars outside [0-9a-f], so HEX_RE should not match
        let stripped: String = b64.chars().filter(|c| !c.is_whitespace()).collect();
        // 'S' is not a hex char, so the regex should fail
        assert!(!HEX_RE.is_match(&stripped) || !stripped.len().is_multiple_of(2) || stripped.len() < 8);
    }

    #[test]
    fn base32_not_confused_with_base58() {
        // "JBSWY3DP" is valid Base32 but also matches Base58 alphabet;
        // Base32 check comes first so it wins
        let input = "JBSWY3DP";
        assert!(BASE32_RE.is_match(input) && input.len().is_multiple_of(8));
        // Base58 would also match but is checked later
        assert!(BASE58_RE.is_match(input));
    }

    #[test]
    fn base58_rejects_plain_lowercase_words() {
        assert!(!is_base58_candidate("password"));
        assert!(!is_base58_candidate("metadata"));
    }

    #[test]
    fn mixed_percent_and_base64_still_finds_embedded_base64() {
        let mut app = Basie64App::default();
        app.input = "note=foo%20bar token=SGVsbG8sIHdvcmxkIQ==".into();

        run_detection(&mut app);

        assert_eq!(app.detected_format, None);
        assert!(app.mixed_matches.iter().any(|m| m == "SGVsbG8sIHdvcmxkIQ=="));
    }
}
