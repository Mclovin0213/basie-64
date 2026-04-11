use crate::decode::DecodeHint;
use crate::settings::Settings;
use crate::{detect, theme, ui};
use base64::{engine::general_purpose, Engine as _};
use eframe::egui;
use regex::Regex;
use std::fs;

pub const LARGE_PASTE_THRESHOLD: usize = 1_000_000;

pub struct Basie64App {
    pub(crate) input: String,
    pub(crate) last_input: String,
    pub(crate) output: String,
    pub(crate) error: Option<String>,
    pub(crate) error_hint: Option<DecodeHint>,
    pub(crate) show_banner: bool,
    pub(crate) banner_message: String,
    pub(crate) base64_regex: Regex,
    pub(crate) mixed_matches: Vec<String>,
    pub(crate) image_preview: Option<egui::TextureHandle>,
    pub(crate) encoded_data_uri: Option<String>,

    pub(crate) settings: Settings,
    pub(crate) applied_theme: Option<crate::theme::Theme>,

    pub(crate) now: f64,
    pub(crate) copy_pulse_at: Option<f64>,
    pub(crate) banner_fade_start: Option<f64>,
    pub(crate) large_paste_confirmed: bool,
}

impl Default for Basie64App {
    fn default() -> Self {
        Self {
            input: String::new(),
            last_input: String::new(),
            output: String::new(),
            error: None,
            error_hint: None,
            show_banner: false,
            banner_message: String::new(),
            base64_regex: Regex::new(
                r"(?x) (?:[A-Za-z0-9+/]{4}){4,} (?:[A-Za-z0-9+/]{2}== | [A-Za-z0-9+/]{3}=)?",
            )
            .expect("static regex must compile"),
            mixed_matches: Vec::new(),
            image_preview: None,
            encoded_data_uri: None,
            settings: Settings::load(),
            applied_theme: None,
            now: 0.0,
            copy_pulse_at: None,
            banner_fade_start: None,
            large_paste_confirmed: false,
        }
    }
}

impl Basie64App {
    pub fn clear(&mut self) {
        self.input.clear();
        self.output.clear();
        self.error = None;
        self.error_hint = None;
        self.show_banner = false;
        self.mixed_matches.clear();
        self.image_preview = None;
        self.encoded_data_uri = None;
        self.large_paste_confirmed = false;
    }

    pub fn mark_copy_pulse(&mut self) {
        self.copy_pulse_at = Some(self.now);
    }

    pub fn request_decode(&mut self, ctx: &egui::Context) {
        if self.input != self.last_input {
            self.large_paste_confirmed = false;
            self.last_input = self.input.clone();
        }

        if self.input.len() > LARGE_PASTE_THRESHOLD && !self.large_paste_confirmed {
            self.error = Some(format!(
                "Input is {:.1} MB — click Decode again to confirm.",
                self.input.len() as f64 / 1_000_000.0
            ));
            self.error_hint = None;
            self.large_paste_confirmed = true;
            return;
        }

        let b64 = self.input.clone();
        self.decode_input_str(ctx, &b64);
        self.large_paste_confirmed = false;
    }
}

impl eframe::App for Basie64App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.now = ctx.input(|i| i.time);

        // Apply theme if it changed (or first frame).
        if self.applied_theme != Some(self.settings.theme) {
            theme::apply(ctx, self.settings.theme);
            self.applied_theme = Some(self.settings.theme);
        }

        // Keyboard shortcuts
        ctx.input(|i| {
            if i.modifiers.command && i.key_pressed(egui::Key::Enter) {
                self.request_decode(ctx);
            }
            if i.modifiers.command
                && i.modifiers.shift
                && i.key_pressed(egui::Key::C)
                && !self.output.is_empty()
            {
                ctx.copy_text(self.output.clone());
                self.copy_pulse_at = Some(self.now);
            }
            if i.key_pressed(egui::Key::Escape) {
                self.clear();
            }
        });

        // Drag-drop files
        let dropped = ctx.input(|i| i.raw.dropped_files.first().cloned());
        if let Some(file) = dropped {
            if let Some(path) = &file.path {
                if let Ok(bytes) = fs::read(path) {
                    self.input = format!(
                        "[File: {}]\n(Size: {} bytes)\n",
                        path.display(),
                        bytes.len()
                    );
                    self.output = general_purpose::STANDARD.encode(&bytes);
                    let mime_type = infer::get(&bytes)
                        .map(|t| t.mime_type())
                        .unwrap_or("application/octet-stream");
                    self.encoded_data_uri =
                        Some(format!("data:{};base64,{}", mime_type, self.output));
                    self.error = None;
                    self.error_hint = None;
                    self.show_banner = false;
                    self.mixed_matches.clear();
                    self.image_preview = None;
                    self.settings.push_recent_file(path.clone());
                    self.settings.save();
                }
            }
        }

        detect::run_detection(self);

        ui::top_bar::show(self, ctx);
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.add_space(10.0);
                ui::banner::show(self, ctx, ui);
                ui::banner::show_mixed_matches(self, ctx, ui);
                ui::input::show(self, ui);
                ui.add_space(12.0);
                ui::buttons::show(self, ctx, ui);
                ui.add_space(12.0);
                ui::output::show(self, ctx, ui);
                ui::banner::show_error(self, ctx, ui);
            });
        });

        // Keep animations ticking
        if self.copy_pulse_at.is_some()
            || ui::banner::is_fade_active(self.banner_fade_start, self.now)
        {
            ctx.request_repaint();
        }

        if !ui::banner::is_fade_active(self.banner_fade_start, self.now) {
            self.banner_fade_start = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regex_compiles_via_default() {
        let app = Basie64App::default();
        assert!(app.base64_regex.is_match("SGVsbG8sIHdvcmxkIQ=="));
    }

    #[test]
    fn encode_text_roundtrip() {
        let input = "Hello, world!";
        let expected = "SGVsbG8sIHdvcmxkIQ==";
        assert_eq!(general_purpose::STANDARD.encode(input), expected);
    }

    #[test]
    fn decode_valid_text() {
        let mut app = Basie64App::default();
        let ctx = egui::Context::default();
        app.decode_input_str(&ctx, "SGVsbG8sIHdvcmxkIQ==");
        assert_eq!(app.output, "Hello, world!");
        assert!(app.error.is_none());
        assert!(app.image_preview.is_none());
    }

    #[test]
    fn decode_invalid() {
        let mut app = Basie64App::default();
        let ctx = egui::Context::default();
        app.decode_input_str(&ctx, "not_valid_b64!!_");
        assert!(app.error.is_some());
        assert!(app.error.as_ref().unwrap().contains("Invalid Base64"));
        assert!(app.image_preview.is_none());
    }

    #[test]
    fn decode_binary_not_utf8() {
        let mut app = Basie64App::default();
        let ctx = egui::Context::default();
        app.decode_input_str(&ctx, "////");
        assert!(app.output.contains("Decoded 3 binary bytes"));
        assert!(app.error.is_none());
    }

    #[test]
    fn decode_jwt() {
        let mut app = Basie64App::default();
        let ctx = egui::Context::default();
        let header =
            general_purpose::URL_SAFE_NO_PAD.encode(b"{\"alg\":\"HS256\",\"typ\":\"JWT\"}");
        let payload = general_purpose::URL_SAFE_NO_PAD
            .encode(b"{\"sub\":\"1234567890\",\"name\":\"John Doe\",\"iat\":1516239022}");
        let signature = "SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";
        let jwt = format!("{}.{}.{}", header, payload, signature);
        app.decode_input_str(&ctx, &jwt);
        assert!(app.output.contains("JWT Detected"));
        assert!(app.output.contains("John Doe"));
        assert!(app.error.is_none());
    }

    #[test]
    fn decode_data_uri() {
        let mut app = Basie64App::default();
        let ctx = egui::Context::default();
        let valid_b64 = "SGVsbG8sIHdvcmxkIQ==";
        let data_uri = format!("data:text/plain;base64,{}", valid_b64);
        app.decode_input_str(&ctx, &data_uri);
        assert_eq!(app.output, "Hello, world!");
        assert!(app.error.is_none());
    }

    #[test]
    fn decode_url_safe() {
        let mut app = Basie64App::default();
        let ctx = egui::Context::default();
        let url_safe = general_purpose::URL_SAFE.encode(b"hello world!?");
        app.decode_input_str(&ctx, &url_safe);
        assert_eq!(app.output, "hello world!?");
        assert!(app.error.is_none());
    }

    #[test]
    fn large_input_requires_confirmation() {
        let mut app = Basie64App::default();
        let ctx = egui::Context::default();

        app.input = "A".repeat(LARGE_PASTE_THRESHOLD + 4);

        app.request_decode(&ctx);
        assert!(app
            .error
            .as_ref()
            .is_some_and(|err| err.contains("click Decode again")));
        assert!(app.output.is_empty());
        assert!(app.large_paste_confirmed);

        app.request_decode(&ctx);
        assert!(!app.output.is_empty());
        assert!(app.error.is_none());
        assert!(!app.large_paste_confirmed);
    }

    #[test]
    fn large_input_confirmation_resets_after_input_change() {
        let mut app = Basie64App::default();
        let ctx = egui::Context::default();

        app.input = "A".repeat(LARGE_PASTE_THRESHOLD + 4);
        app.request_decode(&ctx);
        assert!(app.large_paste_confirmed);

        app.input = "B".repeat(LARGE_PASTE_THRESHOLD + 4);
        app.request_decode(&ctx);

        assert!(app
            .error
            .as_ref()
            .is_some_and(|err| err.contains("click Decode again")));
        assert!(app.output.is_empty());
        assert!(app.large_paste_confirmed);
    }
}
