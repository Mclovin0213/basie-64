use crate::app::Basie64App;
use base64::{engine::general_purpose, Engine as _};

pub fn run_detection(app: &mut Basie64App) {
    if app.input == app.last_input {
        return;
    }
    app.last_input = app.input.clone();
    app.show_banner = false;
    app.mixed_matches.clear();

    let trimmed = app.input.trim();
    if trimmed.is_empty() {
        return;
    }

    let is_plain_b64 = app.base64_regex.is_match(trimmed)
        && trimmed.len().is_multiple_of(4)
        && !trimmed.contains(' ');

    if is_plain_b64 && general_purpose::STANDARD.decode(trimmed).is_ok() {
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
    use regex::Regex;

    fn regex() -> Regex {
        Regex::new(r"(?x) (?:[A-Za-z0-9+/]{4}){4,} (?:[A-Za-z0-9+/]{2}== | [A-Za-z0-9+/]{3}=)?")
            .unwrap()
    }

    #[test]
    fn regex_matches_valid() {
        let r = regex();
        assert!(r.is_match("SGVsbG8sIHdvcmxkIQ=="));
    }

    #[test]
    fn regex_finds_mixed_content() {
        let r = regex();
        let log =
            "Error at line 42: data=SGVsbG8sIHdvcmxkIQ== status=fail fallback=YW5vdGhlciBzdHJpbmc=";
        let matches: Vec<&str> = r.find_iter(log).map(|m| m.as_str()).collect();
        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0], "SGVsbG8sIHdvcmxkIQ==");
        assert_eq!(matches[1], "YW5vdGhlciBzdHJpbmc=");
    }
}
