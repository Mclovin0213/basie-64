use crate::app::Basie64App;
use eframe::egui;

const COPY_PULSE_DURATION: f64 = 0.35;

pub fn show(app: &mut Basie64App, ctx: &egui::Context, ui: &mut egui::Ui) {
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.heading("Output");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let copy_label = copy_button_label(app);
                    let copy_resp = ui
                        .button(copy_label)
                        .on_hover_text("Copy output to clipboard (⌘⇧C)");
                    if copy_resp.clicked() {
                        ctx.copy_text(app.output.clone());
                        app.mark_copy_pulse();
                    }
                    if let Some(data_uri) = &app.encoded_data_uri {
                        if ui
                            .button("🌐 Copy as Data URI")
                            .on_hover_text("Copy as data: URI for embedding in web content")
                            .clicked()
                        {
                            ctx.copy_text(data_uri.clone());
                            app.mark_copy_pulse();
                        }
                    }
                });
            });
            ui.add_space(4.0);
            egui::ScrollArea::vertical()
                .id_salt("output_scroll")
                .max_height(140.0)
                .show(ui, |ui| {
                    ui.add(
                        egui::TextEdit::multiline(&mut app.output)
                            .font(egui::TextStyle::Monospace)
                            .interactive(false)
                            .desired_width(f32::INFINITY)
                            .desired_rows(6),
                    );
                });

            if let Some(texture) = &app.image_preview {
                ui.add_space(8.0);
                ui.label("Image Preview:");
                ui.add(egui::Image::new(texture).max_width(ui.available_width()));
            }
        });
    });

    // Clear expired pulse
    if let Some(started) = app.copy_pulse_at {
        if app.now - started > COPY_PULSE_DURATION {
            app.copy_pulse_at = None;
        }
    }
}

fn copy_button_label(app: &Basie64App) -> &'static str {
    match app.copy_pulse_at {
        Some(started) if app.now - started < COPY_PULSE_DURATION => "✓ Copied!",
        _ => "📋 Copy",
    }
}
