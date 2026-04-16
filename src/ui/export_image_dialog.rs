//! Export Image modal dialog — reskinned to Pencil Phase 6 design tokens.

use crate::app::Basie64App;
use crate::theme::{icons, Tokens};
use crate::ui::widgets;
use eframe::egui::{self, CornerRadius, Frame, Margin};

pub fn show(app: &mut Basie64App, ctx: &egui::Context) {
    let Some(dialog) = app.export_image_dialog.clone() else {
        return;
    };
    let Some(meta) = app.image_meta.clone() else {
        app.close_export_image_dialog();
        return;
    };

    let mut should_save = false;
    let mut should_cancel = false;
    let mut new_strip = dialog.strip_metadata;
    let mut new_expanded = dialog.exif_expanded;

    let screen_rect = ctx.screen_rect();
    let t = Tokens::for_theme(app.settings.theme);

    // Full-screen backdrop
    let backdrop_clicked = egui::Area::new("export_image_dialog_backdrop".into())
        .fixed_pos(screen_rect.min)
        .order(egui::Order::Foreground)
        .interactable(true)
        .show(ctx, |ui| {
            let response = ui.allocate_response(screen_rect.size(), egui::Sense::click_and_drag());
            ui.painter().rect_filled(screen_rect, 0.0, t.modal_backdrop);
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
            Frame::new()
                .fill(t.overlay_surface)
                .corner_radius(CornerRadius::same(12))
                .inner_margin(Margin::same(0))
                .shadow(t.shadow_lg)
                .show(ui, |ui| {
                    ui.set_min_width(480.0);
                    ui.set_max_width(600.0);
                    ui.vertical(|ui| {
                        // ── Header ────────────────────────────────────────
                        Frame::new()
                            .inner_margin(Margin::symmetric(20, 16))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label(
                                        egui::RichText::new("Export Image")
                                            .font(egui::FontId::new(
                                                15.0,
                                                egui::FontFamily::Name("inter_semibold".into()),
                                            ))
                                            .color(t.text_primary),
                                    );
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            if widgets::icon_button(
                                                ui,
                                                icons::X,
                                                "Close (Esc)",
                                                false,
                                            )
                                            .clicked()
                                            {
                                                should_cancel = true;
                                            }
                                        },
                                    );
                                });
                            });

                        widgets::divider(ui);

                        // ── Body ──────────────────────────────────────────
                        Frame::new()
                            .inner_margin(Margin::symmetric(20, 16))
                            .show(ui, |ui| {
                                ui.vertical(|ui| {
                                    ui.spacing_mut().item_spacing.y = 12.0;

                                    // Image readout
                                    widgets::input_frame(ui, |ui| {
                                        let info = format!(
                                            "{} · {}×{} · {}",
                                            meta.kind.label(),
                                            meta.width,
                                            meta.height,
                                            humanize_size(meta.size_bytes),
                                        );
                                        ui.label(
                                            egui::RichText::new(info)
                                                .font(egui::FontId::monospace(12.0))
                                                .color(t.text_primary),
                                        );
                                    });

                                    // Metadata section
                                    if !meta.exif.is_empty() {
                                        let expanded_label = if new_expanded {
                                            "Hide EXIF fields"
                                        } else {
                                            "Show EXIF fields"
                                        };
                                        let exif_banner_resp = widgets::accent_banner(
                                            ui,
                                            widgets::AccentTone::Amber,
                                            Some(icons::INFO),
                                            &format!(
                                                "{} EXIF {} detected",
                                                meta.exif.len(),
                                                if meta.exif.len() == 1 {
                                                    "field"
                                                } else {
                                                    "fields"
                                                }
                                            ),
                                            Some(expanded_label),
                                        );
                                        if exif_banner_resp.action_clicked {
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
                                                                        .font(
                                                                            egui::FontId::monospace(
                                                                                11.0,
                                                                            ),
                                                                        )
                                                                        .color(t.text_secondary),
                                                                );
                                                                ui.label(
                                                                    egui::RichText::new(
                                                                        &field.value,
                                                                    )
                                                                    .font(egui::FontId::monospace(
                                                                        11.0,
                                                                    ))
                                                                    .color(t.text_primary),
                                                                );
                                                                ui.end_row();
                                                            }
                                                        });
                                                });
                                        }
                                    } else if meta.has_strippable_metadata {
                                        widgets::accent_banner(
                                            ui,
                                            widgets::AccentTone::Amber,
                                            Some(icons::INFO),
                                            "Non-EXIF metadata present (text chunks / XMP / IPTC).",
                                            None,
                                        );
                                    } else {
                                        ui.label(
                                            egui::RichText::new("No metadata detected.")
                                                .small()
                                                .color(t.text_muted),
                                        );
                                    }

                                    // Strip metadata checkbox
                                    widgets::divider(ui);
                                    let strip_enabled =
                                        meta.strip_supported && meta.has_strippable_metadata;
                                    ui.add_enabled_ui(strip_enabled, |ui| {
                                        ui.checkbox(
                                            &mut new_strip,
                                            egui::RichText::new("Strip metadata before saving")
                                                .color(t.text_primary),
                                        );
                                    });
                                    ui.label(
                                        egui::RichText::new(strip_help_text(
                                            strip_enabled,
                                            meta.strip_supported,
                                            meta.kind.label(),
                                        ))
                                        .small()
                                        .color(t.text_muted),
                                    );
                                });
                            });

                        widgets::divider(ui);

                        // ── Footer ────────────────────────────────────────
                        Frame::new()
                            .inner_margin(Margin::symmetric(20, 12))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    widgets::key_chip(ui, "Esc", "close", false);
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            ui.spacing_mut().item_spacing.x = 8.0;
                                            if widgets::primary_button(
                                                ui,
                                                "Save…",
                                                Some(icons::DOWNLOAD),
                                            )
                                            .clicked()
                                            {
                                                should_save = true;
                                            }
                                            if widgets::secondary_button(ui, "Cancel", None)
                                                .clicked()
                                            {
                                                should_cancel = true;
                                            }
                                        },
                                    );
                                });
                            });
                    });
                });
        });

    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        should_cancel = true;
    }

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
