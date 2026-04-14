//! Shared egui widget helpers that render against the `theme::Tokens` design
//! system. Every screen migration in Phases 2-7 pulls from this module so the
//! visual language is enforceable from one place.
//!
//! All helpers are pure — they read the current theme off `ui.visuals()` and
//! never touch `Basie64App` state. Callers are responsible for wiring click
//! responses back into the app.

// Helpers land in this module ahead of the per-screen migrations in later
// phases. Individual entries get consumed as the UI is rewritten — until then,
// the dead-code warnings are expected.
#![allow(dead_code)]

use crate::theme::{icons, Tokens};
use eframe::egui::{
    self, Align, Color32, CornerRadius, FontFamily, FontId, Frame, InnerResponse, Layout, Margin,
    Rect, Response, RichText, Sense, Stroke, Ui, Vec2,
};

/// Resolve the currently active design tokens from the UI's visuals. Widgets
/// never hardcode the palette — they always go through this helper so switching
/// theme at runtime reskins every call site for free.
pub fn tokens(ui: &Ui) -> Tokens {
    if ui.visuals().dark_mode {
        Tokens::dark()
    } else {
        Tokens::light()
    }
}

/// Semantic accent tone used by banners, chips, and warning strips.
#[allow(dead_code)] // some tones only land in later phases.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccentTone {
    Blue,
    Amber,
    Green,
    Orange,
    Purple,
    Red,
}

impl AccentTone {
    /// Returns `(strong, dim)` — strong is the foreground / border colour, dim
    /// is the soft background fill used behind the strong text.
    pub fn colors(self, t: &Tokens) -> (Color32, Color32) {
        match self {
            Self::Blue => (t.accent_blue, t.accent_blue_dim),
            Self::Amber => (t.accent_amber, t.accent_amber_dim),
            Self::Green => (t.accent_green, t.accent_green_dim),
            Self::Orange => (t.accent_orange, t.accent_orange_dim),
            Self::Purple => (t.accent_purple, t.accent_purple_dim),
            Self::Red => (t.accent_red, t.accent_red_dim),
        }
    }
}

// ---------- Typography helpers ------------------------------------------------

const INTER_SEMIBOLD: &str = "inter_semibold";
const LUCIDE: &str = "lucide";

fn inter_semibold(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(INTER_SEMIBOLD.into()))
}

fn lucide_font(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(LUCIDE.into()))
}

fn mono_font(size: f32) -> FontId {
    FontId::new(size, FontFamily::Monospace)
}

/// Build a `RichText` run set in the Lucide icon font at the given size.
pub fn lucide_text(glyph: char, size: f32, color: Color32) -> RichText {
    RichText::new(glyph.to_string())
        .font(lucide_font(size))
        .color(color)
}

// ---------- Buttons -----------------------------------------------------------

struct ButtonStyle {
    fill: Color32,
    text_color: Color32,
    border: Option<Color32>,
    corner: u8,
    padding: Vec2,
}

fn button_body(ui: &mut Ui, label: &str, icon: Option<char>, style: ButtonStyle) -> Response {
    let ButtonStyle {
        fill,
        text_color,
        border,
        corner,
        padding,
    } = style;
    let t = tokens(ui);
    let icon_width = if icon.is_some() { 14.0 + 6.0 } else { 0.0 };
    let font = FontId::new(13.0, FontFamily::Name(INTER_SEMIBOLD.into()));
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_owned(), font.clone(), text_color);
    let content_w = galley.size().x + icon_width;
    let desired = Vec2::new(content_w + padding.x * 2.0, padding.y * 2.0 + 18.0);
    let (rect, response) = ui.allocate_exact_size(desired, Sense::click());
    if !ui.is_rect_visible(rect) {
        return response;
    }

    let visuals_fill = if response.is_pointer_button_down_on() {
        t.bg_hover
    } else if response.hovered() {
        // Slight brightening of the base fill on hover.
        fill.gamma_multiply(1.08)
    } else {
        fill
    };

    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, CornerRadius::same(corner), visuals_fill);
    if let Some(stroke) = border {
        painter.rect_stroke(
            rect,
            CornerRadius::same(corner),
            Stroke::new(1.0, stroke),
            egui::StrokeKind::Inside,
        );
    }

    let mut cursor = rect.left_center() + Vec2::new(padding.x, 0.0);
    if let Some(glyph) = icon {
        let icon_galley = painter.layout_no_wrap(glyph.to_string(), lucide_font(14.0), text_color);
        let icon_pos = cursor - Vec2::new(0.0, icon_galley.size().y / 2.0);
        painter.galley(icon_pos, icon_galley, text_color);
        cursor.x += 14.0 + 6.0;
    }
    let label_pos = cursor - Vec2::new(0.0, galley.size().y / 2.0);
    painter.galley(label_pos, galley, text_color);

    response
}

/// Primary CTA — accent-blue fill, dark ink, bold label.
pub fn primary_button(ui: &mut Ui, label: &str, icon: Option<char>) -> Response {
    let t = tokens(ui);
    button_body(
        ui,
        label,
        icon,
        ButtonStyle {
            fill: t.btn_primary_bg,
            text_color: t.btn_primary_text,
            border: None,
            corner: 6,
            padding: Vec2::new(14.0, 8.0),
        },
    )
}

/// Secondary action — card fill with a subtle border, primary-text label.
pub fn secondary_button(ui: &mut Ui, label: &str, icon: Option<char>) -> Response {
    let t = tokens(ui);
    button_body(
        ui,
        label,
        icon,
        ButtonStyle {
            fill: t.btn_secondary_bg,
            text_color: t.btn_secondary_text,
            border: Some(t.border_subtle),
            corner: 6,
            padding: Vec2::new(14.0, 8.0),
        },
    )
}

/// Ghost action — transparent fill, muted label, subtle hover.
pub fn ghost_button(ui: &mut Ui, label: &str, icon: Option<char>) -> Response {
    let t = tokens(ui);
    button_body(
        ui,
        label,
        icon,
        ButtonStyle {
            fill: Color32::TRANSPARENT,
            text_color: t.btn_ghost_text,
            border: None,
            corner: 6,
            padding: Vec2::new(10.0, 8.0),
        },
    )
}

/// Square icon button (32×32 by default) for top-bar / toolbar controls.
///
/// `active` flips the button into accent-blue-dim pressed state — use for
/// toggles whose "on" state should read as selected.
pub fn icon_button(ui: &mut Ui, glyph: char, tooltip: &str, active: bool) -> Response {
    let t = tokens(ui);
    let size = Vec2::splat(32.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    if !ui.is_rect_visible(rect) {
        return response.on_hover_text(tooltip);
    }
    let painter = ui.painter_at(rect);
    let fill = if active {
        t.accent_blue_dim
    } else if response.hovered() || response.is_pointer_button_down_on() {
        t.bg_hover
    } else {
        Color32::TRANSPARENT
    };
    let border = if active { Some(t.accent_blue) } else { None };
    painter.rect_filled(rect, CornerRadius::same(6), fill);
    if let Some(stroke) = border {
        painter.rect_stroke(
            rect,
            CornerRadius::same(6),
            Stroke::new(1.0, stroke),
            egui::StrokeKind::Inside,
        );
    }
    let glyph_color = if active {
        t.accent_blue
    } else {
        t.text_secondary
    };
    let galley = painter.layout_no_wrap(glyph.to_string(), lucide_font(16.0), glyph_color);
    let pos = rect.center() - galley.size() * 0.5;
    painter.galley(pos, galley, glyph_color);

    response.on_hover_text(tooltip)
}

// ---------- Surfaces: cards, inputs, glass panels ----------------------------

/// A "card" surface: `bg_card` fill, `border_subtle` outline, radius 8, 16px
/// inner padding.
pub fn card_frame<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
    let t = tokens(ui);
    Frame::new()
        .fill(t.bg_card)
        .stroke(Stroke::new(1.0, t.border_subtle))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::same(16))
        .show(ui, add)
}

/// An "input" surface: `bg_input` fill, `border_subtle` outline, radius 6, 16px
/// inner padding.
pub fn input_frame<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
    let t = tokens(ui);
    Frame::new()
        .fill(t.bg_input)
        .stroke(Stroke::new(1.0, t.border_subtle))
        .corner_radius(CornerRadius::same(6))
        .inner_margin(Margin::same(16))
        .show(ui, add)
}

/// Glass overlay panel — translucent history/settings surface. Top corners
/// rounded, bottom flat, `panel_glass` fill.
pub fn glass_panel<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
    let t = tokens(ui);
    Frame::new()
        .fill(t.panel_glass)
        .stroke(Stroke::new(1.0, t.border_subtle))
        .corner_radius(CornerRadius {
            nw: 12,
            ne: 12,
            sw: 0,
            se: 0,
        })
        .inner_margin(Margin::same(16))
        .show(ui, add)
}

/// 1px horizontal divider painted in `border_subtle`.
pub fn divider(ui: &mut Ui) {
    let t = tokens(ui);
    let height = 1.0;
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());
    ui.painter().hline(
        rect.x_range(),
        rect.center().y,
        Stroke::new(1.0, t.border_subtle),
    );
}

// ---------- Banners, chips, headers ------------------------------------------

/// Response payload for [`accent_banner`] — exposes both the outer click
/// response (for "dismiss by clicking anywhere") and the inline action button
/// click state.
#[derive(Debug)]
pub struct BannerResponse {
    pub outer: Response,
    pub action_clicked: bool,
}

/// A tinted banner row: soft background, strong-coloured text, optional leading
/// icon, optional trailing ghost-style action button.
pub fn accent_banner(
    ui: &mut Ui,
    tone: AccentTone,
    icon: Option<char>,
    text: &str,
    action_label: Option<&str>,
) -> BannerResponse {
    let t = tokens(ui);
    let (strong, dim) = tone.colors(&t);
    let mut action_clicked = false;

    let inner = Frame::new()
        .fill(dim)
        .stroke(Stroke::new(1.0, strong))
        .corner_radius(CornerRadius::same(6))
        .inner_margin(Margin::symmetric(12, 10))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                if let Some(glyph) = icon {
                    ui.label(lucide_text(glyph, 14.0, strong));
                    ui.add_space(8.0);
                }
                ui.label(RichText::new(text).font(inter_semibold(13.0)).color(strong));
                if let Some(label) = action_label {
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let r = ui.add(
                            egui::Button::new(
                                RichText::new(label)
                                    .font(inter_semibold(12.0))
                                    .color(strong),
                            )
                            .fill(Color32::TRANSPARENT)
                            .stroke(Stroke::new(1.0, strong))
                            .corner_radius(CornerRadius::same(4)),
                        );
                        if r.clicked() {
                            action_clicked = true;
                        }
                    });
                }
            });
        });

    BannerResponse {
        outer: inner.response,
        action_clicked,
    }
}

/// Keyboard shortcut pill used in the bottom hint row.
///
/// Rendered as `[key]  label` where the key chip has a soft background and the
/// label sits in muted text.
pub fn key_chip(ui: &mut Ui, key: &str, label: &str) {
    let t = tokens(ui);
    ui.horizontal(|ui| {
        Frame::new()
            .fill(t.bg_elevated)
            .stroke(Stroke::new(1.0, t.border_subtle))
            .corner_radius(CornerRadius::same(4))
            .inner_margin(Margin::symmetric(6, 2))
            .show(ui, |ui| {
                ui.label(RichText::new(key).font(mono_font(11.0)).color(t.text_mono));
            });
        ui.add_space(4.0);
        ui.label(
            RichText::new(label)
                .font(FontId::new(11.0, FontFamily::Proportional))
                .color(t.text_muted),
        );
    });
}

/// Tiny filled kind chip — e.g. "PNG", "JPEG", "GIF" tags above the image
/// metadata bar.
pub fn meta_chip(ui: &mut Ui, kind: &str) {
    let t = tokens(ui);
    Frame::new()
        .fill(t.bg_elevated)
        .stroke(Stroke::new(1.0, t.border_subtle))
        .corner_radius(CornerRadius::same(4))
        .inner_margin(Margin::symmetric(6, 2))
        .show(ui, |ui| {
            ui.label(
                RichText::new(kind)
                    .font(inter_semibold(11.0))
                    .color(t.text_primary),
            );
        });
}

/// Section heading — 13px Inter SemiBold in `text_primary`.
pub fn section_header(ui: &mut Ui, label: &str) {
    let t = tokens(ui);
    ui.label(
        RichText::new(label)
            .font(inter_semibold(13.0))
            .color(t.text_primary),
    );
}

// ---------- Rebind helpers (unused in Phase 1, here for later phases) --------

#[allow(dead_code)]
pub fn accent_chevron(ui: &mut Ui, expanded: bool) -> Response {
    let t = tokens(ui);
    let glyph = if expanded {
        icons::CHEVRON_DOWN
    } else {
        icons::CHEVRON_RIGHT
    };
    let size = Vec2::splat(16.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    let painter = ui.painter_at(rect);
    let galley = painter.layout_no_wrap(glyph.to_string(), lucide_font(14.0), t.text_secondary);
    let pos = rect.center() - galley.size() * 0.5;
    painter.galley(pos, galley, t.text_secondary);
    response
}

#[allow(dead_code)]
pub fn inset_rect(rect: Rect, by: f32) -> Rect {
    rect.shrink(by)
}
