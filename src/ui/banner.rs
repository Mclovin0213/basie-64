use crate::app::Basie64App;
use eframe::egui;

const FADE_DURATION: f64 = 0.25;

pub fn show(app: &mut Basie64App, ctx: &egui::Context, ui: &mut egui::Ui) {
    if !app.show_banner {
        return;
    }
    let alpha = fade_alpha(app.banner_fade_start, app.now);
    let accent = egui::Color32::from_rgba_unmultiplied(255, 204, 0, alpha);
    let text_color = ui
        .visuals()
        .override_text_color
        .unwrap_or(ui.visuals().text_color());
    let text_color = egui::Color32::from_rgba_unmultiplied(
        text_color.r(),
        text_color.g(),
        text_color.b(),
        alpha,
    );

    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("✨ ").color(accent));
        ui.label(
            egui::RichText::new(&app.banner_message)
                .color(text_color)
                .strong(),
        );
        if ui
            .button("Yes, decode it!")
            .on_hover_text("Decode this detected Base64")
            .clicked()
        {
            let b64 = app.input.clone();
            app.decode_input_str(ctx, &b64);
        }
    });
    ui.add_space(10.0);
}

pub fn show_mixed_matches(app: &mut Basie64App, ctx: &egui::Context, ui: &mut egui::Ui) {
    if app.mixed_matches.is_empty() {
        return;
    }
    ui.group(|ui| {
        ui.label(
            egui::RichText::new(format!(
                "🔍 Found {} potential Base64 strings in text:",
                app.mixed_matches.len()
            ))
            .strong(),
        );
        let mut match_to_decode = None;
        for (i, m) in app.mixed_matches.iter().enumerate().take(5) {
            ui.horizontal(|ui| {
                ui.label(format!(
                    "{}: {}...",
                    i + 1,
                    &m.chars().take(20).collect::<String>()
                ));
                if ui.button("Decode").clicked() {
                    match_to_decode = Some(m.clone());
                }
            });
        }
        if let Some(m) = match_to_decode {
            app.input = m.clone();
            app.decode_input_str(ctx, &m);
        }
        if app.mixed_matches.len() > 5 {
            ui.label(format!("...and {} more", app.mixed_matches.len() - 5));
        }
    });
    ui.add_space(10.0);
}

pub fn show_error(app: &mut Basie64App, ctx: &egui::Context, ui: &mut egui::Ui) {
    let Some(err) = app.error.clone() else { return };
    ui.add_space(8.0);
    ui.colored_label(egui::Color32::LIGHT_RED, format!("⚠️ Error: {}", err));

    if let Some(hint) = app.error_hint.clone() {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(hint.message()).italics().weak());
            if let Some(label) = hint.action_label() {
                if ui.button(label).clicked() {
                    let stripped: String =
                        app.input.chars().filter(|c| !c.is_whitespace()).collect();
                    app.input = stripped.clone();
                    app.decode_input_str(ctx, &stripped);
                }
            }
        });
    }
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
