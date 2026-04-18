use eframe::egui;
use egui::epaint::Shadow;
use image::GenericImageView;
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf, sync::Arc};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Theme {
    Light,
    #[default]
    Dark,
    System,
}

impl Theme {
    pub fn next(self) -> Self {
        match self {
            Theme::Light => Theme::Dark,
            Theme::Dark => Theme::System,
            Theme::System => Theme::Light,
        }
    }

    pub(crate) fn resolve(self) -> ResolvedTheme {
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
pub(crate) enum ResolvedTheme {
    Light,
    Dark,
}

/// Design tokens lifted verbatim from `basie_64.pen` (Pencil redesign). Every
/// value in the palette has a matching field here so tweaks to the design
/// system are a one-line edit in this file.
///
/// Dark mode mirrors the Pencil file exactly. Light mode is derived and marked
/// provisional until the Pencil source grows an explicit light palette.
// Most fields are read only after Phase 2+ migrations — keep all of them
// allocated now so the token layer is authoritative from day one.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct Tokens {
    pub bg_base: egui::Color32,
    pub bg_surface: egui::Color32,
    pub bg_elevated: egui::Color32,
    pub bg_card: egui::Color32,
    pub bg_input: egui::Color32,
    pub bg_hover: egui::Color32,

    pub border_subtle: egui::Color32,
    pub border_default: egui::Color32,
    pub border_focus: egui::Color32,

    pub text_primary: egui::Color32,
    pub text_secondary: egui::Color32,
    pub text_muted: egui::Color32,
    pub text_mono: egui::Color32,

    pub accent_blue: egui::Color32,
    pub accent_blue_dim: egui::Color32,
    pub accent_amber: egui::Color32,
    pub accent_amber_dim: egui::Color32,
    pub accent_green: egui::Color32,
    pub accent_green_dim: egui::Color32,
    pub accent_orange: egui::Color32,
    pub accent_orange_dim: egui::Color32,
    pub accent_purple: egui::Color32,
    pub accent_purple_dim: egui::Color32,
    pub accent_red: egui::Color32,
    pub accent_red_dim: egui::Color32,

    pub btn_primary_bg: egui::Color32,
    pub btn_primary_text: egui::Color32,
    pub btn_secondary_bg: egui::Color32,
    pub btn_secondary_text: egui::Color32,
    pub btn_ghost_text: egui::Color32,

    pub history_bg: egui::Color32,
    pub modal_backdrop: egui::Color32,
    pub modal_surface: egui::Color32,
    pub panel_glass: egui::Color32,
    pub overlay_surface: egui::Color32,

    pub shadow_sm: Shadow,
    pub shadow_lg: Shadow,
    pub shadow_up: Shadow,
}

impl Tokens {
    pub fn dark() -> Self {
        use egui::Color32 as C;
        Self {
            bg_base: C::from_rgb(0x0D, 0x0F, 0x12),
            bg_surface: C::from_rgb(0x14, 0x16, 0x1B),
            bg_elevated: C::from_rgb(0x1A, 0x1D, 0x24),
            bg_card: C::from_rgb(0x1E, 0x21, 0x28),
            bg_input: C::from_rgb(0x12, 0x14, 0x1A),
            bg_hover: C::from_rgb(0x25, 0x28, 0x30),

            border_subtle: C::from_rgb(0x2A, 0x2D, 0x36),
            border_default: C::from_rgb(0x36, 0x39, 0x44),
            border_focus: C::from_rgb(0x5B, 0x9B, 0xD5),

            text_primary: C::from_rgb(0xE8, 0xEA, 0xED),
            text_secondary: C::from_rgb(0x9B, 0xA1, 0xAD),
            text_muted: C::from_rgb(0x6B, 0x72, 0x80),
            text_mono: C::from_rgb(0xC4, 0xCA, 0xD4),

            accent_blue: C::from_rgb(0x5B, 0x9B, 0xD5),
            accent_blue_dim: C::from_rgb(0x2A, 0x3F, 0x55),
            accent_amber: C::from_rgb(0xD4, 0xB0, 0x6A),
            accent_amber_dim: C::from_rgb(0x3A, 0x32, 0x20),
            accent_green: C::from_rgb(0x7A, 0xBF, 0xA0),
            accent_green_dim: C::from_rgb(0x1E, 0x3A, 0x2F),
            accent_orange: C::from_rgb(0xD4, 0xA5, 0x74),
            accent_orange_dim: C::from_rgb(0x3A, 0x2E, 0x1E),
            accent_purple: C::from_rgb(0xA7, 0x8B, 0xDB),
            accent_purple_dim: C::from_rgb(0x2A, 0x1E, 0x3A),
            accent_red: C::from_rgb(0xD4, 0x8A, 0x8A),
            accent_red_dim: C::from_rgb(0x3A, 0x1E, 0x1E),

            btn_primary_bg: C::from_rgb(0x5B, 0x9B, 0xD5),
            btn_primary_text: C::from_rgb(0x0D, 0x0F, 0x12),
            btn_secondary_bg: C::from_rgb(0x1E, 0x21, 0x28),
            btn_secondary_text: C::from_rgb(0xC4, 0xCA, 0xD4),
            btn_ghost_text: C::from_rgb(0x9B, 0xA1, 0xAD),

            history_bg: C::from_rgba_unmultiplied(0x11, 0x13, 0x18, 0xCC),
            modal_backdrop: C::from_rgba_unmultiplied(0x0D, 0x0F, 0x12, 0x99),
            modal_surface: C::from_rgb(0x1A, 0x1D, 0x24),
            panel_glass: C::from_rgba_unmultiplied(0x14, 0x16, 0x1B, 0xEB),
            overlay_surface: C::from_rgba_unmultiplied(0x1A, 0x1D, 0x24, 0xF0),

            shadow_sm: Shadow {
                offset: [0, 1],
                blur: 3,
                spread: 0,
                color: C::from_black_alpha(64),
            },
            shadow_lg: Shadow {
                offset: [0, 4],
                blur: 16,
                spread: 0,
                color: C::from_black_alpha(89),
            },
            shadow_up: Shadow {
                offset: [0, -1],
                blur: 3,
                spread: 0,
                color: C::from_black_alpha(64),
            },
        }
    }

    // TODO(design): provisional light mode — the Pencil file only ships a dark
    // palette today. Replace these values verbatim once the .pen source grows
    // an explicit light variant.
    pub fn light() -> Self {
        use egui::Color32 as C;
        Self {
            bg_base: C::from_rgb(0xF7, 0xF8, 0xFA),
            bg_surface: C::from_rgb(0xEF, 0xF1, 0xF4),
            bg_elevated: C::from_rgb(0xFF, 0xFF, 0xFF),
            bg_card: C::from_rgb(0xFF, 0xFF, 0xFF),
            bg_input: C::from_rgb(0xF4, 0xF5, 0xF8),
            bg_hover: C::from_rgb(0xE6, 0xE8, 0xED),

            border_subtle: C::from_rgb(0xE2, 0xE4, 0xEA),
            border_default: C::from_rgb(0xCB, 0xCF, 0xD7),
            border_focus: C::from_rgb(0x3B, 0x7B, 0xB8),

            text_primary: C::from_rgb(0x17, 0x19, 0x1D),
            text_secondary: C::from_rgb(0x5A, 0x61, 0x72),
            text_muted: C::from_rgb(0x8B, 0x91, 0x9F),
            text_mono: C::from_rgb(0x2A, 0x2D, 0x36),

            accent_blue: C::from_rgb(0x3B, 0x7B, 0xB8),
            accent_blue_dim: C::from_rgb(0xDD, 0xE8, 0xF6),
            accent_amber: C::from_rgb(0xB0, 0x86, 0x2C),
            accent_amber_dim: C::from_rgb(0xF6, 0xED, 0xD4),
            accent_green: C::from_rgb(0x3E, 0x8F, 0x6B),
            accent_green_dim: C::from_rgb(0xD9, 0xEE, 0xE3),
            accent_orange: C::from_rgb(0xB5, 0x7B, 0x34),
            accent_orange_dim: C::from_rgb(0xF6, 0xE7, 0xD3),
            accent_purple: C::from_rgb(0x79, 0x5A, 0xB8),
            accent_purple_dim: C::from_rgb(0xE6, 0xDD, 0xF4),
            accent_red: C::from_rgb(0xB8, 0x5A, 0x5A),
            accent_red_dim: C::from_rgb(0xF6, 0xDE, 0xDE),

            btn_primary_bg: C::from_rgb(0x3B, 0x7B, 0xB8),
            btn_primary_text: C::from_rgb(0xFF, 0xFF, 0xFF),
            btn_secondary_bg: C::from_rgb(0xFF, 0xFF, 0xFF),
            btn_secondary_text: C::from_rgb(0x17, 0x19, 0x1D),
            btn_ghost_text: C::from_rgb(0x5A, 0x61, 0x72),

            history_bg: C::from_rgba_unmultiplied(0xFF, 0xFF, 0xFF, 0xCC),
            modal_backdrop: C::from_rgba_unmultiplied(0x0D, 0x0F, 0x12, 0x66),
            modal_surface: C::from_rgb(0xFF, 0xFF, 0xFF),
            panel_glass: C::from_rgba_unmultiplied(0xEF, 0xF1, 0xF4, 0xEB),
            overlay_surface: C::from_rgba_unmultiplied(0xFF, 0xFF, 0xFF, 0xF0),

            shadow_sm: Shadow {
                offset: [0, 1],
                blur: 3,
                spread: 0,
                color: C::from_black_alpha(25),
            },
            shadow_lg: Shadow {
                offset: [0, 4],
                blur: 16,
                spread: 0,
                color: C::from_black_alpha(40),
            },
            shadow_up: Shadow {
                offset: [0, -1],
                blur: 3,
                spread: 0,
                color: C::from_black_alpha(25),
            },
        }
    }

    pub fn for_theme(theme: Theme) -> Self {
        match theme.resolve() {
            ResolvedTheme::Dark => Self::dark(),
            ResolvedTheme::Light => Self::light(),
        }
    }

    pub fn with_private_tint(mut self) -> Self {
        use egui::Color32 as C;
        if self.bg_base == C::from_rgb(0x0D, 0x0F, 0x12) {
            // Dark mode tint
            self.bg_base = C::from_rgb(0x10, 0x0F, 0x16);
            self.bg_surface = C::from_rgb(0x17, 0x15, 0x1E);
            self.bg_elevated = C::from_rgb(0x1D, 0x1B, 0x27);
            self.bg_card = C::from_rgb(0x21, 0x1F, 0x2B);
            self.bg_input = C::from_rgb(0x14, 0x13, 0x1D);
            self.border_subtle = C::from_rgb(0x2E, 0x2B, 0x3A);
        } else {
            // Light mode tint
            self.bg_base = C::from_rgb(0xF5, 0xF4, 0xFA);
            self.bg_surface = C::from_rgb(0xED, 0xEB, 0xF4);
            self.bg_elevated = C::from_rgb(0xFC, 0xFB, 0xFF);
            self.bg_card = C::from_rgb(0xFC, 0xFB, 0xFF);
            self.bg_input = C::from_rgb(0xF2, 0xF0, 0xF8);
            self.border_subtle = C::from_rgb(0xDE, 0xDB, 0xEA);
        }
        self.border_focus = self.accent_purple;
        self
    }
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

pub fn apply(ctx: &egui::Context, theme: Theme, private_mode: bool) {
    let tokens = Tokens::for_theme(theme);
    let tokens = if private_mode {
        tokens.with_private_tint()
    } else {
        tokens
    };
    let resolved = theme.resolve();

    let mut visuals = match resolved {
        ResolvedTheme::Dark => egui::Visuals::dark(),
        ResolvedTheme::Light => egui::Visuals::light(),
    };

    visuals.panel_fill = tokens.bg_base;
    visuals.window_fill = tokens.modal_surface;
    visuals.window_stroke = egui::Stroke::NONE;
    visuals.window_corner_radius = egui::CornerRadius::same(12);
    visuals.menu_corner_radius = egui::CornerRadius::same(8);
    visuals.extreme_bg_color = tokens.bg_input;
    visuals.faint_bg_color = tokens.bg_elevated;
    visuals.code_bg_color = tokens.bg_input;
    visuals.override_text_color = Some(tokens.text_primary);
    visuals.window_shadow = tokens.shadow_sm;

    visuals.widgets.noninteractive.bg_fill = tokens.bg_card;
    visuals.widgets.noninteractive.weak_bg_fill = tokens.bg_card;
    visuals.widgets.noninteractive.bg_stroke = egui::Stroke::NONE;
    visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, tokens.text_secondary);
    visuals.widgets.noninteractive.corner_radius = egui::CornerRadius::same(8);

    visuals.widgets.inactive.bg_fill = tokens.bg_input;
    visuals.widgets.inactive.weak_bg_fill = tokens.bg_input;
    visuals.widgets.inactive.bg_stroke = egui::Stroke::NONE;
    visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, tokens.text_primary);
    visuals.widgets.inactive.corner_radius = egui::CornerRadius::same(6);

    visuals.widgets.hovered.bg_fill = tokens.bg_hover;
    visuals.widgets.hovered.weak_bg_fill = tokens.bg_hover;
    visuals.widgets.hovered.bg_stroke = egui::Stroke::NONE;
    visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, tokens.text_primary);
    visuals.widgets.hovered.corner_radius = egui::CornerRadius::same(6);

    visuals.widgets.active.bg_fill = tokens.accent_blue;
    visuals.widgets.active.weak_bg_fill = tokens.accent_blue_dim;
    visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0, tokens.border_focus);
    visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, tokens.btn_primary_text);
    visuals.widgets.active.corner_radius = egui::CornerRadius::same(6);

    visuals.widgets.open.bg_fill = tokens.bg_hover;
    visuals.widgets.open.weak_bg_fill = tokens.bg_hover;
    visuals.widgets.open.bg_stroke = egui::Stroke::NONE;
    visuals.widgets.open.fg_stroke = egui::Stroke::new(1.0, tokens.text_primary);
    visuals.widgets.open.corner_radius = egui::CornerRadius::same(6);

    visuals.selection.bg_fill = tokens.accent_blue_dim;
    visuals.selection.stroke = egui::Stroke::new(1.0, tokens.border_focus);

    visuals.hyperlink_color = tokens.accent_blue;
    visuals.error_fg_color = tokens.accent_red;
    visuals.warn_fg_color = tokens.accent_orange;

    let mut style = (*ctx.style()).clone();
    style.visuals = visuals;
    style.spacing.item_spacing = egui::vec2(8.0, 8.0);
    style.spacing.button_padding = egui::vec2(14.0, 8.0);
    style.spacing.window_margin = egui::Margin::same(16);
    style.spacing.menu_margin = egui::Margin::same(8);
    ctx.set_style(style);
}

/// Embed the design-system fonts (Inter, IBM Plex Mono, Lucide) into egui so
/// the app looks identical on every machine, offline or online.
///
/// Call this once at startup, after `apply()`.
pub fn install_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    fonts.font_data.insert(
        "inter".to_owned(),
        Arc::new(egui::FontData::from_static(include_bytes!(
            "../assets/fonts/Inter-Regular.ttf"
        ))),
    );
    fonts.font_data.insert(
        "inter_semibold".to_owned(),
        Arc::new(egui::FontData::from_static(include_bytes!(
            "../assets/fonts/Inter-SemiBold.ttf"
        ))),
    );
    fonts.font_data.insert(
        "plex_mono".to_owned(),
        Arc::new(egui::FontData::from_static(include_bytes!(
            "../assets/fonts/IBMPlexMono-Regular.ttf"
        ))),
    );
    fonts.font_data.insert(
        "lucide".to_owned(),
        Arc::new(egui::FontData::from_static(include_bytes!(
            "../assets/fonts/lucide.ttf"
        ))),
    );

    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .insert(0, "inter".to_owned());
    fonts
        .families
        .entry(egui::FontFamily::Monospace)
        .or_default()
        .insert(0, "plex_mono".to_owned());

    fonts.families.insert(
        egui::FontFamily::Name("inter_semibold".into()),
        vec!["inter_semibold".to_owned(), "inter".to_owned()],
    );
    fonts.families.insert(
        egui::FontFamily::Name("lucide".into()),
        vec!["lucide".to_owned()],
    );

    ctx.set_fonts(fonts);
}

/// Lucide icon glyphs (Private Use Area codepoints from `lucide-static 1.8.0`).
///
/// To add a glyph: open `assets/fonts/lucide-codepoints.json`, look up the
/// icon by its kebab-case name, and copy the `encodedCode` hex (`\eHHHH` —
/// the suffix is the codepoint). Then add a `pub const` here.
///
/// Codepoints in the Private Use Area are stable across Lucide releases, but
/// verify when bumping the font.
#[allow(dead_code)] // scaffolding for Phases 2-7; constants land as UI migrates.
pub mod icons {
    pub const BINARY: char = '\u{E1F2}';
    pub const SUN: char = '\u{E178}';
    pub const MOON: char = '\u{E11E}';
    pub const MONITOR: char = '\u{E11D}';
    pub const SETTINGS: char = '\u{E154}';
    pub const CLOCK_3: char = '\u{E250}';
    pub const HISTORY: char = '\u{E1F5}';
    pub const X: char = '\u{E1B2}';
    pub const SCAN_EYE: char = '\u{E536}';
    pub const LAYERS: char = '\u{E529}';
    pub const TRASH_2: char = '\u{E18E}';
    pub const DOWNLOAD: char = '\u{E0B2}';
    pub const TRIANGLE_ALERT: char = '\u{E193}';
    pub const CHEVRON_RIGHT: char = '\u{E06F}';
    pub const CHEVRON_DOWN: char = '\u{E06D}';
    pub const SEARCH: char = '\u{E151}';
    pub const PACKAGE: char = '\u{E129}';
    pub const SHIELD_CHECK: char = '\u{E1FF}';
    pub const KEY: char = '\u{E0FD}';
    pub const EYE: char = '\u{E0BA}';
    pub const COPY: char = '\u{E09E}';
    pub const FILE_DOWN: char = '\u{E318}';
    pub const ARROW_DOWN_UP: char = '\u{E046}';
    pub const CIRCLE_CHECK: char = '\u{E226}';
    pub const CIRCLE_X: char = '\u{E084}';
    pub const CIRCLE_ALERT: char = '\u{E077}';
    pub const INFO: char = '\u{E0F9}';
    pub const COLUMNS_2: char = '\u{E098}';
    pub const EYE_OFF: char = '\u{E0BB}';
}
