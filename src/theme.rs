use eframe::egui;
use image::GenericImageView;
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Theme {
    Light,
    #[default]
    Dark,
    System,
}

impl Theme {
    pub fn label(self) -> &'static str {
        match self {
            Theme::Light => "☀ Light",
            Theme::Dark => "🌙 Dark",
            Theme::System => "🖥 System",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Theme::Light => Theme::Dark,
            Theme::Dark => Theme::System,
            Theme::System => Theme::Light,
        }
    }

    fn resolve(self) -> ResolvedTheme {
        match self {
            Theme::Light => ResolvedTheme::Light,
            Theme::Dark => ResolvedTheme::Dark,
            Theme::System => match dark_light::detect() {
                dark_light::Mode::Light => ResolvedTheme::Light,
                dark_light::Mode::Dark | dark_light::Mode::Default => ResolvedTheme::Dark,
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ResolvedTheme {
    Light,
    Dark,
}

pub fn load_icon() -> Option<egui::IconData> {
    let icon_bytes = load_icon_bytes()?;
    let image = image::load_from_memory(&icon_bytes).ok()?;
    let (width, height) = image.dimensions();
    let rgba = image.into_rgba8().into_raw();
    Some(egui::IconData {
        rgba,
        width,
        height,
    })
}

fn load_icon_bytes() -> Option<Vec<u8>> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let candidates = [
        manifest_dir.join("icon.png"),
        manifest_dir.join("icons.png"),
    ];

    for path in candidates {
        if let Ok(bytes) = fs::read(path) {
            return Some(bytes);
        }
    }

    None
}

pub fn apply(ctx: &egui::Context, theme: Theme) {
    let mut visuals = match theme.resolve() {
        ResolvedTheme::Dark => {
            let mut v = egui::Visuals::dark();
            v.selection.bg_fill = egui::Color32::from_rgb(108, 113, 196);
            v.panel_fill = egui::Color32::from_rgba_unmultiplied(20, 20, 25, 230);
            v.override_text_color = Some(egui::Color32::from_rgb(230, 230, 235));
            v
        }
        ResolvedTheme::Light => {
            let mut v = egui::Visuals::light();
            v.selection.bg_fill = egui::Color32::from_rgb(86, 91, 165);
            v.panel_fill = egui::Color32::from_rgba_unmultiplied(248, 248, 252, 240);
            v.override_text_color = Some(egui::Color32::from_rgb(28, 28, 34));
            v
        }
    };
    visuals.window_shadow = egui::epaint::Shadow::NONE;

    let mut style = (*ctx.style()).clone();
    style.visuals = visuals;
    style.spacing.item_spacing = egui::vec2(8.0, 8.0);
    style.spacing.button_padding = egui::vec2(10.0, 6.0);
    ctx.set_style(style);
}

pub fn top_bar_fill(theme: Theme) -> egui::Color32 {
    match theme.resolve() {
        ResolvedTheme::Dark => egui::Color32::from_rgba_unmultiplied(35, 35, 40, 240),
        ResolvedTheme::Light => egui::Color32::from_rgba_unmultiplied(235, 235, 242, 245),
    }
}
