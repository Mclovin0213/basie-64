use crate::app::Basie64App;
use eframe::egui;

pub fn show(app: &mut Basie64App, ctx: &egui::Context) {
    egui::TopBottomPanel::bottom("history_panel")
        .resizable(true)
        .default_height(220.0)
        .min_height(120.0)
        .max_height(400.0)
        .show(ctx, |ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.strong("History");
                    ui.label(format!("({} entries)", app.history_store.entries().len()));
                    ui.add_space(8.0);
                    if ui.small_button("Clear All").clicked() {
                        app.history_store.clear();
                        app.ensure_selected_history_entry();
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button("✕").clicked() {
                            app.show_history_panel = false;
                        }
                    });
                });

                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    ui.label("🔍");
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut app.history_query)
                            .hint_text("Filter history"),
                    );
                    if response.changed() {
                        app.ensure_selected_history_entry();
                    }
                });

                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new("Enter reloads selected entry. Delete removes it.")
                        .small()
                        .weak(),
                );
                ui.add_space(4.0);

                let visible_ids = app.visible_history_ids();
                if visible_ids.is_empty() {
                    ui.centered_and_justified(|ui| {
                        ui.label(if app.history_query.is_empty() {
                            "No history entries yet."
                        } else {
                            "No matching entries."
                        });
                    });
                    return;
                }

                egui::ScrollArea::vertical().show(ui, |ui| {
                    for entry_id in visible_ids {
                        let Some(entry) = app.history_store.get_by_id(&entry_id).cloned() else {
                            continue;
                        };

                        let selected =
                            app.selected_history_entry.as_deref() == Some(entry.id.as_str());
                        let row_text = format!(
                            "{}  {}  {} → {}",
                            entry.op.icon(),
                            entry.relative_time(),
                            truncate(&entry.input_preview, 30),
                            truncate(&entry.output_preview, 30)
                        );

                        ui.horizontal(|ui| {
                            let response = ui.selectable_label(selected, row_text.clone());
                            if response.clicked() {
                                app.selected_history_entry = Some(entry.id.clone());
                            }
                            if response.double_clicked() {
                                app.selected_history_entry = Some(entry.id.clone());
                                app.restore_selected_history_entry(ctx);
                            }

                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if ui.small_button("✕").clicked() {
                                        let was_selected = selected;
                                        app.history_store.remove_by_id(&entry.id);
                                        if was_selected {
                                            app.ensure_selected_history_entry();
                                        }
                                    }
                                },
                            );
                        });
                        ui.separator();
                    }
                });
            });
        });
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_len).collect();
        format!("{truncated}…")
    }
}
