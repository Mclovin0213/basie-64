use crate::app::Basie64App;
use crate::core::history::{HistoryEntry, HistoryOp};
use crate::theme::icons;
use crate::ui::widgets;
use eframe::egui::{self, Sense};

pub fn show(app: &mut Basie64App, ctx: &egui::Context, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;

        // Primary/secondary flip: whichever action was last is now secondary;
        // the opposite is primary (next likely click).
        if app.last_action_was_encode {
            // Last was encode → Decode is the primary next action
            if widgets::secondary_button(ui, "Encode", Some(icons::LAYERS)).clicked() {
                app.run_encode();
                app.history_store.append(HistoryEntry::new(
                    HistoryOp::Encode,
                    &app.input,
                    &app.output,
                    "standard",
                ));
                app.last_action_was_encode = true;
            }
            if widgets::primary_button(ui, "Decode", Some(icons::SCAN_EYE)).clicked() {
                app.request_decode(ctx);
                app.last_action_was_encode = false;
            }
        } else {
            // Last was decode → Encode is the primary next action
            if widgets::primary_button(ui, "Encode", Some(icons::LAYERS)).clicked() {
                app.run_encode();
                app.history_store.append(HistoryEntry::new(
                    HistoryOp::Encode,
                    &app.input,
                    &app.output,
                    "standard",
                ));
                app.last_action_was_encode = true;
            }
            if widgets::secondary_button(ui, "Decode", Some(icons::SCAN_EYE)).clicked() {
                app.request_decode(ctx);
                app.last_action_was_encode = false;
            }
        }

        if widgets::ghost_button(ui, "Diff", Some(icons::COLUMNS_2)).clicked() {
            app.open_diff_view_from_input();
        }

        // Vertical 1px divider between core actions and file operations
        vertical_divider(ui);

        if widgets::ghost_button(ui, "Save as File", Some(icons::DOWNLOAD)).clicked() {
            app.save_to_file();
        }

        if widgets::ghost_button(ui, "Clear", Some(icons::TRASH_2)).clicked() {
            app.clear();
        }

        // Push batch buttons flush right
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            let batch_enabled = !app.is_batch_running();

            if ui
                .add_enabled(
                    batch_enabled,
                    egui::Button::new("Batch Decode Files…"),
                )
                .on_hover_text("Select multiple .b64 files to batch decode")
                .clicked()
            {
                if let Some(files) = rfd::FileDialog::new().pick_files() {
                    app.start_batch_decode_files(files, None);
                }
            }

            if ui
                .add_enabled(
                    batch_enabled,
                    egui::Button::new("Batch Encode Files…"),
                )
                .on_hover_text("Select multiple files to batch encode")
                .clicked()
            {
                if let Some(files) = rfd::FileDialog::new().pick_files() {
                    app.start_batch_encode_files(files, None);
                }
            }

            if ui
                .add_enabled(
                    batch_enabled,
                    egui::Button::new("Batch Decode Folder…"),
                )
                .on_hover_text("Select a folder to batch decode .b64 files")
                .clicked()
            {
                if let Some(dir) = rfd::FileDialog::new().pick_folder() {
                    app.start_batch_decode(dir, None);
                }
            }

            if ui
                .add_enabled(
                    batch_enabled,
                    egui::Button::new("Batch Encode Folder…"),
                )
                .on_hover_text("Select a folder to batch encode all files")
                .clicked()
            {
                if let Some(dir) = rfd::FileDialog::new().pick_folder() {
                    app.start_batch_encode(dir, None);
                }
            }
        });
    });
}

fn vertical_divider(ui: &mut egui::Ui) {
    let t = widgets::tokens(ui);
    let (rect, _) = ui.allocate_exact_size(egui::Vec2::new(1.0, 20.0), Sense::hover());
    ui.painter_at(rect)
        .rect_filled(rect, 0.0, t.border_subtle);
    // breathing room either side
    let _ = ui.allocate_exact_size(egui::Vec2::new(2.0, 1.0), Sense::hover());
}
