use crate::app::Basie64App;
use crate::core::batch::{BatchOp, BatchSourceKind, BatchStatus};
use crate::theme::Tokens;
use eframe::egui;
use egui_extras::{Column, TableBuilder};
use std::path::PathBuf;

pub fn show(app: &mut Basie64App, ctx: &egui::Context) {
    let t = Tokens::for_theme(app.settings.theme);

    let frame = egui::Frame::new()
        .fill(t.panel_glass)
        .shadow(t.shadow_lg)
        .inner_margin(egui::Margin::same(12));

    egui::TopBottomPanel::bottom("batch_panel")
        .frame(frame)
        .resizable(true)
        .default_height(320.0)
        .min_height(180.0)
        .max_height(540.0)
        .show(ctx, |ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.strong("Batch Operation");
                    if app.is_batch_running() {
                        ui.label(format!(
                            "({}/{} processed)",
                            app.batch_progress.processed, app.batch_progress.total
                        ));
                    } else if let Some(result) = &app.batch_result {
                        ui.label(format!(
                            "({} processed, {} succeeded, {} failed, {} skipped)",
                            result.processed(),
                            result.succeeded(),
                            result.failed(),
                            result.skipped()
                        ));
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let close_button = ui.add_enabled(
                            !app.is_batch_running(),
                            egui::Button::new(egui::RichText::new("✕").small()),
                        );
                        if close_button.clicked() {
                            app.clear_batch_results();
                        }
                    });
                });

                ui.add_space(6.0);

                if app.is_batch_running() {
                    show_batch_progress(app, ui);
                } else if app.batch_pending_confirmation.is_some() {
                    show_batch_confirmation(app, ui);
                } else if app.batch_result.is_some() {
                    show_batch_results(app, ui);
                } else {
                    ui.centered_and_justified(|ui| {
                        ui.label("No batch operation in progress.");
                    });
                }
            });
        });
}

struct BatchConfirmInfo {
    root: PathBuf,
    source_kind: BatchSourceKind,
    selection_count: usize,
    file_count: usize,
    eligible_count: usize,
    total_size: u64,
    operation: BatchOp,
    output_dir: Option<PathBuf>,
}

fn show_batch_confirmation(app: &mut Basie64App, ui: &mut egui::Ui) {
    let pending_info = app
        .batch_pending_confirmation
        .as_ref()
        .map(|pending| BatchConfirmInfo {
            root: pending.preview.root.clone(),
            source_kind: pending.preview.source_kind,
            selection_count: pending.preview.selection_count,
            file_count: pending.preview.file_count,
            eligible_count: pending.preview.eligible_count,
            total_size: pending.preview.total_size,
            operation: pending.preview.operation,
            output_dir: pending.config.output_dir.clone(),
        });

    let Some(info) = pending_info else {
        return;
    };

    let subject_label = match info.source_kind {
        BatchSourceKind::Directory => "folder",
        BatchSourceKind::Files => "selection",
    };

    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("📁").size(20.0));
                ui.vertical(|ui| {
                    ui.strong(format!("Batch {} {}?", info.operation, subject_label));
                    ui.label(format!(
                        "{}  •  {} selected  •  {} items  •  {} eligible  •  {}",
                        info.root.display(),
                        info.selection_count,
                        info.file_count,
                        info.eligible_count,
                        format_size(info.total_size)
                    ));
                });
            });

            ui.add_space(8.0);

            ui.horizontal(|ui| {
                ui.label("Output:");
                if let Some(out_dir) = &info.output_dir {
                    ui.label(
                        egui::RichText::new(out_dir.display().to_string())
                            .small()
                            .monospace(),
                    );
                } else {
                    ui.label(
                        egui::RichText::new("(write sibling files in source locations)")
                            .small()
                            .weak(),
                    );
                }

                if ui.small_button("Change…").clicked() {
                    if let Some(dir) = rfd::FileDialog::new().pick_folder() {
                        app.set_pending_batch_output_dir(Some(dir));
                    }
                }

                if info.output_dir.is_some() && ui.small_button("Use source folders").clicked() {
                    app.set_pending_batch_output_dir(None);
                }
            });

            ui.add_space(8.0);

            ui.horizontal(|ui| {
                if ui.button(format!("{} all", info.operation)).clicked() {
                    app.execute_batch();
                }
                if ui.button("Cancel").clicked() {
                    app.cancel_batch();
                    app.show_batch_panel = false;
                }

                let toggle_op = match info.operation {
                    BatchOp::Encode => BatchOp::Decode,
                    BatchOp::Decode => BatchOp::Encode,
                };
                if ui.button(format!("Switch to {}", toggle_op)).clicked() {
                    app.set_pending_batch_operation(toggle_op);
                }
            });
        });
    });
}

fn show_batch_progress(app: &mut Basie64App, ui: &mut egui::Ui) {
    let total = app.batch_progress.total.max(1);
    let fraction = app.batch_progress.processed as f32 / total as f32;

    ui.label(format!(
        "Processing {} of {}",
        app.batch_progress.processed, app.batch_progress.total
    ));
    ui.add(
        egui::ProgressBar::new(fraction)
            .desired_width(f32::INFINITY)
            .show_percentage(),
    );
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.label(format!("✓ {}", app.batch_progress.succeeded));
        ui.label(format!("✗ {}", app.batch_progress.failed));
        ui.label(format!("○ {}", app.batch_progress.skipped));
    });
    if let Some(current) = &app.batch_progress.current {
        ui.label(
            egui::RichText::new(format!("Current: {}", current.display()))
                .small()
                .monospace(),
        );
    }
}

fn show_batch_results(app: &mut Basie64App, ui: &mut egui::Ui) {
    let Some(result) = app.batch_result.clone() else {
        return;
    };

    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(format!("✓ {}", result.succeeded())).color(
                if result.failed() == 0 {
                    egui::Color32::GREEN
                } else {
                    egui::Color32::YELLOW
                },
            ),
        );
        ui.label(egui::RichText::new(format!("✗ {}", result.failed())).color(egui::Color32::RED));
        ui.label(
            egui::RichText::new(format!("○ {}", result.skipped())).color(egui::Color32::YELLOW),
        );

        if let Some(output_dir) = &result.output_dir {
            ui.add_space(12.0);
            ui.label(
                egui::RichText::new(format!("Output root: {}", output_dir.display()))
                    .small()
                    .monospace(),
            );
        }

        ui.add_space(12.0);

        if ui.button("📋 Export Manifest…").clicked() {
            if let Some(path) = export_manifest_dialog(&result.timestamp, result.operation) {
                match result.manifest_json() {
                    Ok(json) => {
                        if let Err(err) = std::fs::write(&path, json) {
                            app.error = Some(format!("Failed to export manifest: {}", err));
                            app.error_hint = None;
                        }
                    }
                    Err(err) => {
                        app.error = Some(format!("Failed to serialize manifest: {}", err));
                        app.error_hint = None;
                    }
                }
            }
        }

        if ui.button("Clear").clicked() {
            app.clear_batch_results();
        }
    });

    ui.add_space(6.0);
    ui.separator();
    ui.add_space(4.0);

    if result.files.is_empty() {
        ui.centered_and_justified(|ui| {
            ui.label("No files processed.");
        });
        return;
    }

    let rows: Vec<_> = result
        .files
        .iter()
        .map(|file| {
            let (icon, color, detail) = match &file.status {
                BatchStatus::Ok {
                    input_size,
                    output_size,
                } => (
                    "✓",
                    egui::Color32::GREEN,
                    format!(
                        "{} → {}",
                        format_size(*input_size),
                        format_size(*output_size)
                    ),
                ),
                BatchStatus::Skipped { reason } => {
                    ("○", egui::Color32::YELLOW, format!("Skipped: {}", reason))
                }
                BatchStatus::Error { error } => {
                    ("✗", egui::Color32::RED, format!("Error: {}", error))
                }
            };
            (
                icon.to_string(),
                color,
                file.input.display().to_string(),
                file.output
                    .as_ref()
                    .map(|path| path.display().to_string())
                    .unwrap_or_else(|| "—".to_string()),
                detail,
            )
        })
        .collect();

    TableBuilder::new(ui)
        .striped(true)
        .resizable(true)
        .column(Column::exact(28.0))
        .column(Column::remainder())
        .column(Column::remainder())
        .column(Column::remainder())
        .min_scrolled_height(180.0)
        .header(20.0, |mut header| {
            header.col(|ui| {
                ui.strong("");
            });
            header.col(|ui| {
                ui.strong("Input");
            });
            header.col(|ui| {
                ui.strong("Output");
            });
            header.col(|ui| {
                ui.strong("Details");
            });
        })
        .body(|body| {
            body.rows(22.0, rows.len(), |mut row| {
                let (icon, color, input, output, detail) = &rows[row.index()];
                row.col(|ui| {
                    ui.label(egui::RichText::new(icon).color(*color));
                });
                row.col(|ui| {
                    ui.label(egui::RichText::new(input).monospace());
                });
                row.col(|ui| {
                    ui.label(egui::RichText::new(output).small().monospace());
                });
                row.col(|ui| {
                    ui.label(egui::RichText::new(detail).small().weak());
                });
            });
        });
}

fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

fn export_manifest_dialog(timestamp: &str, operation: BatchOp) -> Option<PathBuf> {
    let filename = format!("batch_{}_{}", operation, timestamp.replace([':', '-'], "_"));
    rfd::FileDialog::new()
        .set_file_name(format!("{}.json", filename))
        .save_file()
}
