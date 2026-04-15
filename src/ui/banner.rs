use crate::app::Basie64App;
use crate::core::convert::Format;
use crate::core::history::{HistoryEntry, HistoryOp};
use crate::theme::icons;
use crate::ui::widgets::{self, AccentTone};
use eframe::egui::{self, CornerRadius, Frame, Margin, Stroke};

const FADE_DURATION: f64 = 0.25;

/// Base64 detection banner — blue accent, scan-eye icon, action button.
pub fn show(app: &mut Basie64App, ctx: &egui::Context, ui: &mut egui::Ui) {
    if !app.show_banner {
        return;
    }
    let alpha = fade_alpha(app.banner_fade_start, app.now);
    if alpha == 255 {
        let resp = widgets::accent_banner(
            ui,
            AccentTone::Blue,
            Some(icons::SCAN_EYE),
            &app.banner_message,
            Some("Yes, decode it!"),
        );
        if resp.action_clicked {
            let b64 = app.input.clone();
            app.decode_input_str(ctx, &b64);
        }
    } else {
        let t = widgets::tokens(ui);
        let strong = egui::Color32::from_rgba_unmultiplied(
            t.accent_blue.r(),
            t.accent_blue.g(),
            t.accent_blue.b(),
            alpha,
        );
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(&app.banner_message)
                    .color(strong)
                    .strong(),
            );
            if ui.button("Yes, decode it!").clicked() {
                let b64 = app.input.clone();
                app.decode_input_str(ctx, &b64);
            }
        });
    }
    ui.add_space(6.0);
}

/// Format-conversion hint banner — green accent with ComboBox.
pub fn show_convert_hint(app: &mut Basie64App, _ctx: &egui::Context, ui: &mut egui::Ui) {
    if !app.show_convert_banner {
        return;
    }
    let Some(detected) = app.detected_format else {
        return;
    };
    let t = widgets::tokens(ui);
    let (strong, dim) = AccentTone::Green.colors(&t);

    Frame::new()
        .fill(dim)
        .stroke(Stroke::new(1.0, strong))
        .corner_radius(CornerRadius::same(6))
        .inner_margin(Margin::symmetric(12, 10))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(format!("Detected {} — Convert to:", detected))
                        .color(strong)
                        .strong(),
                );
                egui::ComboBox::from_id_salt("convert_target")
                    .selected_text(format!("{}", app.convert_target))
                    .show_ui(ui, |ui| {
                        for &fmt in Format::all() {
                            if fmt == detected {
                                continue;
                            }
                            ui.selectable_value(
                                &mut app.convert_target,
                                fmt,
                                format!("{}", fmt),
                            );
                        }
                    });
                if ui
                    .button(egui::RichText::new("Convert").color(strong))
                    .clicked()
                {
                    let input_snapshot = app.input.trim().to_string();
                    let variant_label =
                        format!("{} → {}", detected, app.convert_target);
                    app.run_convert();
                    if app.error.is_none() {
                        app.history_store.append(HistoryEntry::new(
                            HistoryOp::Convert,
                            &input_snapshot,
                            &app.output,
                            &variant_label,
                        ));
                    }
                }
            });
        });
    ui.add_space(6.0);
}

/// Mixed-match list banner — card frame with individual Decode buttons.
pub fn show_mixed_matches(app: &mut Basie64App, ctx: &egui::Context, ui: &mut egui::Ui) {
    if app.mixed_matches.is_empty() {
        return;
    }
    widgets::card_frame(ui, |ui| {
        let t = widgets::tokens(ui);
        ui.label(
            egui::RichText::new(format!(
                "Found {} potential Base64 strings in text:",
                app.mixed_matches.len()
            ))
            .color(t.text_primary)
            .strong(),
        );
        ui.add_space(4.0);
        let mut match_to_decode = None;
        for (i, m) in app.mixed_matches.iter().enumerate().take(5) {
            ui.horizontal(|ui| {
                let t2 = widgets::tokens(ui);
                ui.label(
                    egui::RichText::new(format!(
                        "{}: {}…",
                        i + 1,
                        &m.chars().take(24).collect::<String>()
                    ))
                    .font(egui::FontId::monospace(12.0))
                    .color(t2.text_secondary),
                );
                if widgets::ghost_button(ui, "Decode", None).clicked() {
                    match_to_decode = Some(m.clone());
                }
            });
        }
        if let Some(m) = match_to_decode {
            app.input = m.clone();
            app.decode_input_str(ctx, &m);
        }
        if app.mixed_matches.len() > 5 {
            let t2 = widgets::tokens(ui);
            ui.label(
                egui::RichText::new(format!("…and {} more", app.mixed_matches.len() - 5))
                    .color(t2.text_muted)
                    .small(),
            );
        }
    });
    ui.add_space(6.0);
}

/// Error + optional hint banner — red accent.
pub fn show_error(app: &mut Basie64App, ctx: &egui::Context, ui: &mut egui::Ui) {
    let Some(err) = app.error.clone() else {
        return;
    };
    let action_label = app.error_hint.as_ref().and_then(|h| h.action_label());
    let resp = widgets::accent_banner(
        ui,
        AccentTone::Red,
        Some(icons::TRIANGLE_ALERT),
        &err,
        action_label,
    );
    if resp.action_clicked {
        let stripped: String = app.input.chars().filter(|c| !c.is_whitespace()).collect();
        app.input = stripped.clone();
        app.decode_input_str(ctx, &stripped);
    }

    if let Some(hint) = &app.error_hint {
        if !hint.message().is_empty() {
            let t = widgets::tokens(ui);
            ui.label(
                egui::RichText::new(hint.message())
                    .italics()
                    .color(t.text_muted)
                    .small(),
            );
        }
    }
    ui.add_space(6.0);
}

fn fade_alpha(start: Option<f64>, now: f64) -> u8 {
    match start {
        None => 255,
        Some(t) => {
            let elapsed = (now - t).max(0.0);
            if elapsed >= FADE_DURATION {
                255
            } else {
                ((elapsed / FADE_DURATION) * 255.0) as u8
            }
        }
    }
}

pub fn is_fade_active(start: Option<f64>, now: f64) -> bool {
    start.is_some_and(|t| (now - t).max(0.0) < FADE_DURATION)
}

#[cfg(test)]
mod tests {
    use super::is_fade_active;

    #[test]
    fn fade_expires_after_duration() {
        assert!(is_fade_active(Some(1.0), 1.1));
        assert!(!is_fade_active(Some(1.0), 1.25));
        assert!(!is_fade_active(Some(1.0), 1.4));
        assert!(!is_fade_active(None, 1.1));
    }
}
