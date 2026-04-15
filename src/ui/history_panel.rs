use crate::app::Basie64App;
use crate::theme::icons;
use crate::ui::widgets;
use eframe::egui::{self, CornerRadius, Frame, Margin, Sense, Stroke};

const PANEL_HEIGHT: f32 = 340.0;
const STATUS_FOOTER_HEIGHT: f32 = 32.0; // offset above the bottom status bar

pub fn show(app: &mut Basie64App, ctx: &egui::Context) {
    if !app.show_history_panel {
        return;
    }

    let screen_w = ctx.screen_rect().width();

    egui::Area::new("history_panel".into())
        .anchor(
            egui::Align2::CENTER_BOTTOM,
            egui::vec2(0.0, -STATUS_FOOTER_HEIGHT),
        )
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            widgets::glass_panel(ui, |ui| {
                ui.set_min_width(screen_w);
                ui.set_max_width(screen_w);

                ui.vertical(|ui| {
                    // Decorative drag handle
                    drag_handle(ui);
                    ui.add_space(4.0);

                    // Header row
                    ui.horizontal(|ui| {
                        let t = widgets::tokens(ui);
                        ui.label(
                            egui::RichText::new("History")
                                .font(egui::FontId::new(
                                    13.0,
                                    egui::FontFamily::Name("inter_semibold".into()),
                                ))
                                .color(t.text_primary),
                        );
                        ui.label(
                            egui::RichText::new(format!(
                                "· {} entries",
                                app.history_store.entries().len()
                            ))
                            .color(t.text_muted)
                            .small(),
                        );
                        ui.add_space(8.0);
                        if widgets::ghost_button(ui, "Clear All", None).clicked() {
                            app.history_store.clear();
                            app.ensure_selected_history_entry();
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if widgets::icon_button(ui, icons::X, "Close history (⌘H)", false)
                                .clicked()
                            {
                                app.show_history_panel = false;
                            }
                        });
                    });

                    ui.add_space(8.0);

                    // Search field
                    widgets::input_frame(ui, |ui| {
                        ui.horizontal(|ui| {
                            let t = widgets::tokens(ui);
                            ui.label(widgets::lucide_text(icons::SEARCH, 14.0, t.text_muted));
                            ui.add_space(4.0);
                            let response = ui.add(
                                egui::TextEdit::singleline(&mut app.history_query)
                                    .hint_text("Filter history…")
                                    .desired_width(f32::INFINITY),
                            );
                            if response.changed() {
                                app.ensure_selected_history_entry();
                            }
                        });
                    });

                    ui.add_space(8.0);

                    // Entry list
                    let visible_ids = app.visible_history_ids();
                    if visible_ids.is_empty() {
                        let t = widgets::tokens(ui);
                        ui.centered_and_justified(|ui| {
                            ui.label(
                                egui::RichText::new(if app.history_query.is_empty() {
                                    "No history entries yet."
                                } else {
                                    "No matching entries."
                                })
                                .color(t.text_muted),
                            );
                        });
                        return;
                    }

                    egui::ScrollArea::vertical()
                        .max_height(PANEL_HEIGHT - 140.0)
                        .show(ui, |ui| {
                            for entry_id in visible_ids {
                                let Some(entry) = app.history_store.get_by_id(&entry_id).cloned()
                                else {
                                    continue;
                                };
                                let selected = app.selected_history_entry.as_deref()
                                    == Some(entry.id.as_str());

                                history_row(ui, app, ctx, &entry, selected);
                                ui.add_space(4.0);
                            }
                        });
                });
            });
        });
}

fn drag_handle(ui: &mut egui::Ui) {
    let t = widgets::tokens(ui);
    // allocate a full-width row, then draw a centred pill inside it
    let full_width = ui.available_width();
    let (row_rect, _) = ui.allocate_exact_size(egui::Vec2::new(full_width, 12.0), Sense::hover());
    let pill = egui::Rect::from_center_size(row_rect.center(), egui::Vec2::new(40.0, 4.0));
    ui.painter()
        .rect_filled(pill, CornerRadius::same(2), t.border_default);
}

fn history_row(
    ui: &mut egui::Ui,
    app: &mut Basie64App,
    ctx: &egui::Context,
    entry: &crate::core::history::HistoryEntry,
    selected: bool,
) {
    let t = widgets::tokens(ui);
    let fill = if selected {
        t.accent_blue_dim
    } else {
        egui::Color32::TRANSPARENT
    };

    let mut frame = Frame::new()
        .fill(fill)
        .corner_radius(CornerRadius::same(6))
        .inner_margin(Margin::symmetric(10, 8));
    if selected {
        frame = frame.stroke(Stroke::new(1.0, t.accent_blue));
    }

    frame.show(ui, |ui| {
        ui.horizontal(|ui| {
            let row_text = format!(
                "{}  {}  {} → {}",
                entry.op.icon(),
                entry.relative_time(),
                truncate(&entry.input_preview, 28),
                truncate(&entry.output_preview, 28)
            );
            let response = ui.selectable_label(selected, row_text);
            if response.clicked() {
                app.selected_history_entry = Some(entry.id.clone());
            }
            if response.double_clicked() {
                app.selected_history_entry = Some(entry.id.clone());
                app.restore_selected_history_entry(ctx);
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if widgets::icon_button(ui, icons::X, "Remove entry", false).clicked() {
                    let was_selected = selected;
                    app.history_store.remove_by_id(&entry.id);
                    if was_selected {
                        app.ensure_selected_history_entry();
                    }
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
