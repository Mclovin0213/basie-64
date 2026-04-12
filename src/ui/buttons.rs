use crate::app::Basie64App;
use crate::core::history::{HistoryEntry, HistoryOp};
use eframe::egui;

pub fn show(app: &mut Basie64App, ctx: &egui::Context, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        if ui
            .button("Encode → Base64")
            .on_hover_text("Encode the input as standard Base64")
            .clicked()
        {
            app.run_encode();
            app.history_store.append(HistoryEntry::new(
                HistoryOp::Encode,
                &app.input,
                &app.output,
                "standard",
            ));
        }

        let decode_btn = ui
            .button("Decode → Text / Image")
            .on_hover_text("Decode input (⌘↵)");
        if decode_btn.clicked() {
            app.request_decode(ctx);
        }

        if ui
            .button("Add comparison")
            .on_hover_text("Open diff mode and compare with a second Base64 string (⌘D)")
            .clicked()
        {
            app.open_diff_view_from_input();
        }

        if ui
            .button("Save as File...")
            .on_hover_text("Decode input and save to disk")
            .clicked()
        {
            app.save_to_file();
        }

        if ui
            .button("Clear")
            .on_hover_text("Clear input and output (Esc)")
            .clicked()
        {
            app.clear();
        }

        ui.add_space(8.0);

        let batch_enabled = !app.is_batch_running();

        if ui
            .add_enabled(batch_enabled, egui::Button::new("📁 Batch Encode Folder…"))
            .on_hover_text("Select a folder to batch encode all files")
            .clicked()
        {
            if let Some(dir) = rfd::FileDialog::new().pick_folder() {
                app.start_batch_encode(dir, None);
            }
        }

        if ui
            .add_enabled(batch_enabled, egui::Button::new("📁 Batch Decode Folder…"))
            .on_hover_text("Select a folder to batch decode .b64 files")
            .clicked()
        {
            if let Some(dir) = rfd::FileDialog::new().pick_folder() {
                app.start_batch_decode(dir, None);
            }
        }

        if ui
            .add_enabled(batch_enabled, egui::Button::new("🗂 Batch Encode Files…"))
            .on_hover_text("Select multiple files to batch encode")
            .clicked()
        {
            if let Some(files) = rfd::FileDialog::new().pick_files() {
                app.start_batch_encode_files(files, None);
            }
        }

        if ui
            .add_enabled(batch_enabled, egui::Button::new("🗂 Batch Decode Files…"))
            .on_hover_text("Select multiple .b64 files to batch decode")
            .clicked()
        {
            if let Some(files) = rfd::FileDialog::new().pick_files() {
                app.start_batch_decode_files(files, None);
            }
        }
    });
}
