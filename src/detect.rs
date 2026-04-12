use crate::app::Basie64App;
use crate::core::detect::detect;

pub fn run_detection(app: &mut Basie64App) {
    if app.input == app.last_input {
        return;
    }
    app.last_input = app.input.clone();
    app.show_banner = false;
    app.show_convert_banner = false;
    app.detected_format = None;
    app.mixed_matches.clear();

    let result = detect(&app.input, &app.base64_regex);

    app.detected_format = result.detected_format;
    app.show_convert_banner = result.detected_format.is_some();
    app.mixed_matches = result.mixed_matches;

    if result.is_base64 {
        app.show_banner = true;
        if let Some(msg) = result.banner_message {
            app.banner_message = msg;
        }
        app.banner_fade_start = Some(app.now);
    }

    if let Some((a, b)) = result.diff_split {
        app.diff_input_a = a;
        app.diff_input_b = b;
        app.show_diff_view = true;
        app.run_diff();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mixed_percent_and_base64_still_finds_embedded_base64() {
        let mut app = Basie64App {
            input: "note=foo%20bar token=SGVsbG8sIHdvcmxkIQ==".into(),
            ..Default::default()
        };

        run_detection(&mut app);

        assert_eq!(app.detected_format, None);
        assert!(app
            .mixed_matches
            .iter()
            .any(|m| m == "SGVsbG8sIHdvcmxkIQ=="));
    }

    #[test]
    fn valid_diff_delimiter_opens_diff_view() {
        let mut app = Basie64App {
            input: "U0dWc2JHOD0=\n---\nV29ybGQ=".into(),
            ..Default::default()
        };

        run_detection(&mut app);

        assert!(app.show_diff_view);
        assert_eq!(app.diff_input_a, "U0dWc2JHOD0=");
        assert_eq!(app.diff_input_b, "V29ybGQ=");
        assert!(app.diff_error.is_none());
    }

    #[test]
    fn invalid_diff_delimiter_does_not_hijack_main_ui() {
        let mut app = Basie64App {
            input: "title\n---\nbody".into(),
            ..Default::default()
        };

        run_detection(&mut app);

        assert!(!app.show_diff_view);
        assert!(app.diff_input_a.is_empty());
        assert!(app.diff_input_b.is_empty());
    }
}
