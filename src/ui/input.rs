use crate::app::Basie64App;
use crate::samples::SAMPLES;
use eframe::egui;

pub fn show(app: &mut Basie64App, ui: &mut egui::Ui) {
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.heading("Input");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.menu_button("📦 Samples", |ui| {
                        for sample in SAMPLES {
                            if ui.button(sample.label).clicked() {
                                app.input = sample.payload.to_string();
                                ui.close_menu();
                            }
                        }
                    })
                    .response
                    .on_hover_text("Load a sample payload");
                });
            });
            ui.add_space(4.0);

            if app.input.is_empty() {
                ui.vertical_centered(|ui| {
                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new("Paste Base64, drop a file, or try a sample.")
                            .weak()
                            .italics(),
                    );
                    if !app.settings.shortcut_hint_dismissed {
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new("⌘↵ decode  ·  ⌘⇧C copy  ·  Esc clear")
                                    .small()
                                    .weak(),
                            );
                            if ui.small_button("Got it").clicked() {
                                app.settings.shortcut_hint_dismissed = true;
                                app.settings.save();
                            }
                        });
                    }
                    ui.add_space(8.0);
                });
            }

            egui::ScrollArea::vertical()
                .id_salt("input_scroll")
                .max_height(140.0)
                .show(ui, |ui| {
                    ui.add(
                        egui::TextEdit::multiline(&mut app.input)
                            .hint_text("Enter text, plain Base64, or mixed logs...")
                            .desired_width(f32::INFINITY)
                            .desired_rows(6),
                    );
                });
        });
    });
}
