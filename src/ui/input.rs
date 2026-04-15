use crate::app::Basie64App;
use crate::samples::SAMPLES;
use crate::theme::icons;
use crate::ui::widgets;
use eframe::egui;

pub fn show(app: &mut Basie64App, ui: &mut egui::Ui) {
    widgets::input_frame(ui, |ui| {
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                widgets::section_header(ui, "Input");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let samples_btn = widgets::ghost_button(ui, "Samples", Some(icons::PACKAGE));
                    let popup_id = ui.make_persistent_id("samples_menu");
                    if samples_btn.clicked() {
                        ui.memory_mut(|m| m.toggle_popup(popup_id));
                    }
                    egui::popup::popup_below_widget(
                        ui,
                        popup_id,
                        &samples_btn,
                        egui::popup::PopupCloseBehavior::CloseOnClickOutside,
                        |ui| {
                            ui.set_min_width(200.0);
                            for sample in SAMPLES {
                                if ui.button(sample.label).clicked() {
                                    app.input = sample.payload.to_string();
                                    ui.memory_mut(|m| m.close_popup());
                                }
                            }
                        },
                    );
                });
            });
            ui.add_space(8.0);
            egui::ScrollArea::vertical()
                .id_salt("input_scroll")
                .max_height(140.0)
                .show(ui, |ui| {
                    let t = widgets::tokens(ui);
                    ui.add(
                        egui::TextEdit::multiline(&mut app.input)
                            .hint_text("Paste or type content here… (⌘+Enter to encode/decode)")
                            .desired_width(f32::INFINITY)
                            .desired_rows(6)
                            .text_color(t.text_primary)
                            .font(egui::TextStyle::Monospace),
                    );
                });
        });
    });
}
