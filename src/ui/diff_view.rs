use crate::app::Basie64App;
use crate::core::diff::DiffKind;
use eframe::egui;

/// Render the full diff view, replacing the standard CentralPanel.
/// The top bar is rendered separately by app.rs before this is called.
pub fn show(app: &mut Basie64App, ctx: &egui::Context) {
    egui::CentralPanel::default()
        .frame(egui::Frame::NONE)
        .show(ctx, |ui| {
            // Header row
            ui.horizontal(|ui| {
                ui.heading("Diff");
                if let Some(result) = &app.diff_result {
                    let summary = format!(
                        "{} addition{}, {} removal{}, {} unchanged",
                        result.additions,
                        if result.additions == 1 { "" } else { "s" },
                        result.removals,
                        if result.removals == 1 { "" } else { "s" },
                        result.unchanged,
                    );
                    ui.label(egui::RichText::new(summary).weak().small());
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button("✕").clicked() {
                        app.show_diff_view = false;
                    }
                    ui.label(egui::RichText::new("⌘D to toggle").weak().small());
                });
            });

            ui.add_space(8.0);

            // Two input boxes — auto-compare on change
            let inputs_changed = show_inputs(app, ui);
            if inputs_changed
                || (app.diff_input_a != app.diff_last_a || app.diff_input_b != app.diff_last_b)
            {
                app.run_diff();
            }

            ui.add_space(8.0);

            // Diff output
            if let Some(result) = app.diff_result.clone() {
                if result.lines.is_empty()
                    && app.diff_input_a.is_empty()
                    && app.diff_input_b.is_empty()
                {
                    show_empty_hint(ui);
                } else if result.additions == 0 && result.removals == 0 {
                    ui.vertical_centered(|ui| {
                        ui.add_space(20.0);
                        ui.label(
                            egui::RichText::new(
                                "No differences — inputs decode to identical content.",
                            )
                            .weak()
                            .italics(),
                        );
                    });
                } else {
                    let mode_label = if app.diff_is_binary {
                        "Binary diff (hex dump)"
                    } else {
                        "Text diff"
                    };
                    ui.label(egui::RichText::new(mode_label).weak().small());
                    ui.add_space(4.0);

                    egui::ScrollArea::vertical()
                        .id_salt("diff_output_scroll")
                        .show(ui, |ui| {
                            ui.columns(2, |cols| {
                                show_column(&result.lines, Side::Left, &mut cols[0]);
                                show_column(&result.lines, Side::Right, &mut cols[1]);
                            });
                        });
                }
            } else if let Some(error) = &app.diff_error {
                ui.add_space(12.0);
                ui.colored_label(egui::Color32::LIGHT_RED, format!("⚠️ {}", error));
            } else {
                show_empty_hint(ui);
            }
        });
}

fn show_empty_hint(ui: &mut egui::Ui) {
    ui.vertical_centered(|ui| {
        ui.add_space(20.0);
        ui.label(
            egui::RichText::new(
                "Paste Base64 strings above, or separate them with --- in the main input.",
            )
            .weak()
            .italics(),
        );
        ui.add_space(6.0);
        ui.label(
            egui::RichText::new("⌘K opens the command palette.")
                .weak()
                .small(),
        );
    });
}

/// Render the two side-by-side input boxes. Returns `true` if either input changed.
fn show_inputs(app: &mut Basie64App, ui: &mut egui::Ui) -> bool {
    let mut changed = false;
    ui.columns(2, |cols| {
        cols[0].vertical(|ui| {
            ui.label(egui::RichText::new("String A (Base64)").small().weak());
            ui.add_space(2.0);
            let resp = ui.add(
                egui::TextEdit::multiline(&mut app.diff_input_a)
                    .desired_rows(4)
                    .desired_width(f32::INFINITY)
                    .font(egui::TextStyle::Monospace),
            );
            if resp.changed() {
                changed = true;
            }
        });
        cols[1].vertical(|ui| {
            ui.label(egui::RichText::new("String B (Base64)").small().weak());
            ui.add_space(2.0);
            let resp = ui.add(
                egui::TextEdit::multiline(&mut app.diff_input_b)
                    .desired_rows(4)
                    .desired_width(f32::INFINITY)
                    .font(egui::TextStyle::Monospace),
            );
            if resp.changed() {
                changed = true;
            }
        });
    });
    changed
}

enum Side {
    Left,
    Right,
}

fn show_column(lines: &[crate::core::diff::DiffLine], side: Side, ui: &mut egui::Ui) {
    let add_color = egui::Color32::from_rgba_premultiplied(0, 80, 0, 60);
    let remove_color = egui::Color32::from_rgba_premultiplied(80, 0, 0, 60);

    for diff_line in lines {
        let (content, line_num) = match side {
            Side::Left => (diff_line.line_a.as_deref(), diff_line.num_a),
            Side::Right => (diff_line.line_b.as_deref(), diff_line.num_b),
        };

        let bg_color = match diff_line.kind {
            DiffKind::Added => Some(add_color),
            DiffKind::Removed => Some(remove_color),
            DiffKind::Unchanged => None,
        };

        let text = content.unwrap_or("");
        let num_str = line_num
            .map(|n| format!("{:4} ", n))
            .unwrap_or_else(|| "     ".to_string());

        egui::Frame::new()
            .fill(bg_color.unwrap_or(egui::Color32::TRANSPARENT))
            .inner_margin(egui::Margin::symmetric(2_i8, 1_i8))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(&num_str).monospace().weak().small());
                    ui.label(egui::RichText::new(text).monospace().small());
                });
            });
    }
}
