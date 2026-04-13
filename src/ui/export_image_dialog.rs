//! Export Image modal dialog. Opened by the metadata bar's Export button or
//! the Cmd+K command palette. Shows a short readout of the decoded image
//! and an optional checkbox to strip metadata losslessly before saving.

use crate::app::Basie64App;
use eframe::egui;

pub fn show(app: &mut Basie64App, ctx: &egui::Context) {
    let Some(dialog) = app.export_image_dialog.clone() else {
        return;
    };
    let Some(meta) = app.image_meta.clone() else {
        // Defensive: the open_export_image_dialog guard should prevent this,
        // but if state somehow drifts, close rather than panic.
        app.close_export_image_dialog();
        return;
    };

    let mut should_save = false;
    let mut should_cancel = false;
    let mut new_strip = dialog.strip_metadata;
    let mut new_expanded = dialog.exif_expanded;

    let screen_rect = ctx.screen_rect();

    // Full-screen backdrop that *eats* pointer events so clicks can't fall
    // through to the main UI while the dialog is open. `Order::Foreground`
    // puts it above the main panels; the dialog itself lives one layer
    // higher at `Order::Tooltip` so its buttons still receive input.
    let backdrop_clicked = egui::Area::new("export_image_dialog_backdrop".into())
        .fixed_pos(screen_rect.min)
        .order(egui::Order::Foreground)
        .interactable(true)
        .show(ctx, |ui| {
            let response = ui.allocate_response(screen_rect.size(), egui::Sense::click_and_drag());
            ui.painter()
                .rect_filled(screen_rect, 0.0, egui::Color32::from_black_alpha(160));
            response.clicked()
        })
        .inner;

    if backdrop_clicked {
        should_cancel = true;
    }

    egui::Area::new("export_image_dialog".into())
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .order(egui::Order::Tooltip)
        .show(ctx, |ui| {
            egui::Frame::window(ui.style()).show(ui, |ui| {
                ui.set_min_width(420.0);
                ui.set_max_width(520.0);
                ui.vertical(|ui| {
                    ui.heading("Export Image");
                    ui.add_space(8.0);

                    ui.label(
                        egui::RichText::new(format!(
                            "{} · {}×{} · {}",
                            meta.kind.label(),
                            meta.width,
                            meta.height,
                            humanize_size(meta.size_bytes),
                        ))
                        .monospace(),
                    );

                    ui.add_space(6.0);

                    let warn_color = egui::Color32::from_rgb(0xE0, 0x9F, 0x3E);
                    if !meta.exif.is_empty() {
                        ui.label(
                            egui::RichText::new(format!(
                                "⚠ {} EXIF {} detected",
                                meta.exif.len(),
                                if meta.exif.len() == 1 {
                                    "field"
                                } else {
                                    "fields"
                                },
                            ))
                            .color(warn_color),
                        );

                        let expanded_label = if new_expanded {
                            "▾ hide fields"
                        } else {
                            "▸ show fields"
                        };
                        if ui.button(expanded_label).clicked() {
                            new_expanded = !new_expanded;
                        }

                        if new_expanded {
                            egui::ScrollArea::vertical()
                                .id_salt("export_image_exif_list")
                                .max_height(180.0)
                                .show(ui, |ui| {
                                    egui::Grid::new("export_image_exif_grid")
                                        .num_columns(2)
                                        .spacing([12.0, 2.0])
                                        .striped(true)
                                        .show(ui, |ui| {
                                            for field in &meta.exif {
                                                ui.label(
                                                    egui::RichText::new(&field.tag)
                                                        .monospace()
                                                        .small(),
                                                );
                                                ui.label(
                                                    egui::RichText::new(&field.value)
                                                        .monospace()
                                                        .small(),
                                                );
                                                ui.end_row();
                                            }
                                        });
                                });
                        }
                    } else if meta.has_strippable_metadata {
                        // Non-EXIF metadata present (e.g. PNG tEXt / JPEG XMP).
                        // We don't have a field list to show, but the strip
                        // pass will still scrub it.
                        ui.label(
                            egui::RichText::new(
                                "⚠ Non-EXIF metadata present (text chunks / XMP / IPTC).",
                            )
                            .color(warn_color),
                        );
                    } else {
                        ui.label(egui::RichText::new("No metadata detected.").small().weak());
                    }

                    ui.add_space(10.0);
                    ui.separator();
                    ui.add_space(8.0);

                    let strip_enabled = meta.strip_supported && meta.has_strippable_metadata;
                    ui.add_enabled_ui(strip_enabled, |ui| {
                        ui.checkbox(&mut new_strip, "Strip metadata before saving");
                    });
                    ui.label(
                        egui::RichText::new(strip_help_text(
                            strip_enabled,
                            meta.strip_supported,
                            meta.kind.label(),
                        ))
                        .small()
                        .weak(),
                    );

                    ui.add_space(12.0);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Save…").clicked() {
                            should_save = true;
                        }
                        if ui.button("Cancel").clicked() {
                            should_cancel = true;
                        }
                    });
                });
            });
        });

    // Escape closes the dialog.
    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        should_cancel = true;
    }

    // Write back any local changes before acting.
    if let Some(existing) = app.export_image_dialog.as_mut() {
        existing.strip_metadata = new_strip && meta.strip_supported && meta.has_strippable_metadata;
        existing.exif_expanded = new_expanded;
    }

    if should_cancel {
        app.close_export_image_dialog();
    } else if should_save {
        app.execute_export_image_save();
    }
}

fn strip_help_text(enabled: bool, kind_supported: bool, kind_label: &str) -> String {
    if enabled {
        "Lossless — EXIF, text chunks, and IPTC blocks are removed. Pixel data is unchanged."
            .to_string()
    } else if !kind_supported {
        format!("Lossless strip is not supported for {}.", kind_label)
    } else {
        "No metadata to strip.".to_string()
    }
}

fn humanize_size(bytes: usize) -> String {
    const KB: usize = 1024;
    const MB: usize = KB * 1024;
    const GB: usize = MB * 1024;
    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}
