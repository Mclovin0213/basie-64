use crate::app::Basie64App;
use crate::core::history::{HistoryEntry, HistoryOp};
use base64::{engine::general_purpose, Engine as _};
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
            save_to_file(app);
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

fn save_to_file(app: &mut Basie64App) {
    let b64 = app.input.trim();
    let clean_b64 = b64.replace(|c: char| c.is_whitespace(), "");
    let b64_content = if let Some(idx) = clean_b64.find("base64,") {
        &clean_b64[idx + 7..]
    } else {
        clean_b64.as_str()
    };

    let decode_result = general_purpose::STANDARD
        .decode(b64_content)
        .or_else(|_| general_purpose::URL_SAFE.decode(b64_content))
        .or_else(|_| general_purpose::URL_SAFE_NO_PAD.decode(b64_content));

    match decode_result {
        Ok(bytes) => {
            let extension = infer::get(&bytes).map(|k| k.extension()).unwrap_or("bin");
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Decoded", &[extension])
                .save_file()
            {
                match std::fs::write(&path, &bytes) {
                    Ok(()) => {
                        app.output = format!("Saved successfully to {}", path.display());
                        app.error = None;
                        app.error_hint = None;
                        app.settings.push_recent_file(path);
                        app.settings.save();
                    }
                    Err(e) => {
                        app.error = Some(format!("Failed to save file: {}", e));
                    }
                }
            }
        }
        Err(_) => {
            app.error = Some("Invalid Base64 for file decoding".into());
            app.error_hint = crate::core::decode::infer_hint(b64);
        }
    }
}
