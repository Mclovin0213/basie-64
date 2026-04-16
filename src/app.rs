use crate::core::batch::{
    preview_batch, process_batch_with_progress, BatchConfig, BatchOp, BatchPreview, BatchProgress,
    BatchResult, BatchSource,
};
use crate::core::convert::Format;
use crate::core::decode::DecodeHint;
use crate::core::diff::{diff_binary, diff_text, DiffResult};
use crate::core::history::{history_path, HistoryOp, HistoryStore};
use crate::settings::Settings;
use crate::{detect, theme, ui};
use base64::{engine::general_purpose, Engine as _};
use eframe::egui;
use regex::Regex;
use std::fs;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};
use std::thread;

pub const LARGE_PASTE_THRESHOLD: usize = 1_000_000;

pub struct Basie64App {
    pub(crate) input: String,
    pub(crate) last_input: String,
    pub(crate) output: String,
    pub(crate) error: Option<String>,
    pub(crate) error_hint: Option<DecodeHint>,
    pub(crate) show_banner: bool,
    pub(crate) banner_message: String,
    pub(crate) base64_regex: Regex,
    pub(crate) mixed_matches: Vec<String>,
    pub(crate) image_preview: Option<egui::TextureHandle>,
    pub(crate) encoded_data_uri: Option<String>,

    /// Raw decoded image bytes, kept alive for the Export Image flow.
    pub(crate) image_bytes: Option<Vec<u8>>,
    /// Format / dimensions / EXIF metadata for the decoded image.
    pub(crate) image_meta: Option<crate::core::image_meta::ImageMeta>,
    /// Modal state for the Export Image dialog (`None` = closed).
    pub(crate) export_image_dialog: Option<ExportImageDialog>,

    /// Structured JWT inspection data, populated when the current decode
    /// result is a JWT. Cleared on every other decode/encode/clear path.
    pub(crate) jwt_inspection: Option<crate::core::jwt::JwtInspection>,
    /// Secret input for HMAC verification in the JWT inspector.
    pub(crate) jwt_secret_input: String,
    /// Latest HMAC verification result for the currently-displayed JWT.
    pub(crate) jwt_verification: Option<crate::core::jwt::VerificationResult>,

    /// Non-Base64 format detected in the current input.
    pub(crate) detected_format: Option<Format>,
    /// Target format for the Convert action (persists across detections).
    pub(crate) convert_target: Format,
    /// Whether the "Detected X — Convert to Y?" hint banner is visible.
    pub(crate) show_convert_banner: bool,

    pub(crate) settings: Settings,
    pub(crate) applied_theme: Option<crate::theme::Theme>,

    pub(crate) now: f64,
    pub(crate) copy_pulse_at: Option<f64>,
    pub(crate) banner_fade_start: Option<f64>,
    pub(crate) large_paste_confirmed: bool,

    /// History of recent encode/decode operations.
    pub(crate) history_store: HistoryStore,
    /// Whether the history panel is currently visible.
    pub(crate) show_history_panel: bool,
    /// Tracks whether the last triggered action was Encode (true) or Decode (false).
    /// Drives the primary/secondary button swap in the action row.
    pub(crate) last_action_was_encode: bool,
    /// Search query for the history panel.
    pub(crate) history_query: String,
    /// Currently selected history entry id.
    pub(crate) selected_history_entry: Option<String>,

    /// Diff view state.
    pub(crate) show_diff_view: bool,
    pub(crate) diff_input_a: String,
    pub(crate) diff_input_b: String,
    pub(crate) diff_last_a: String,
    pub(crate) diff_last_b: String,
    pub(crate) diff_result: Option<DiffResult>,
    pub(crate) diff_is_binary: bool,
    pub(crate) diff_error: Option<String>,

    /// Command palette state.
    pub(crate) show_command_palette: bool,
    pub(crate) command_palette_query: String,
    pub(crate) command_palette_selected: usize,
    pub(crate) command_palette_just_opened: bool,

    /// Batch operation state.
    pub(crate) batch_result: Option<BatchResult>,
    pub(crate) show_batch_panel: bool,
    pub(crate) batch_pending_confirmation: Option<BatchPending>,
    pub(crate) batch_progress: BatchProgress,
    pub(crate) batch_receiver: Option<Receiver<BatchWorkerMessage>>,
}

/// State for a batch operation awaiting user confirmation.
#[derive(Clone)]
pub struct BatchPending {
    pub config: BatchConfig,
    pub preview: BatchPreview,
}

/// Modal state for the Export Image dialog.
#[derive(Clone, Debug)]
pub struct ExportImageDialog {
    /// Whether to strip EXIF / text metadata before saving.
    pub strip_metadata: bool,
    /// Whether the detailed EXIF field list is currently expanded.
    pub exif_expanded: bool,
}

pub enum BatchWorkerMessage {
    Progress(BatchProgress),
    Finished(BatchResult),
}

impl Default for Basie64App {
    fn default() -> Self {
        let settings = Settings::load();
        let private_mode = settings.private_mode;
        Self {
            input: String::new(),
            last_input: String::new(),
            output: String::new(),
            error: None,
            error_hint: None,
            show_banner: false,
            banner_message: String::new(),
            base64_regex: Regex::new(
                r"(?x) (?:[A-Za-z0-9+/]{4}){4,} (?:[A-Za-z0-9+/]{2}== | [A-Za-z0-9+/]{3}=)?",
            )
            .expect("static regex must compile"),
            mixed_matches: Vec::new(),
            image_preview: None,
            encoded_data_uri: None,
            image_bytes: None,
            image_meta: None,
            export_image_dialog: None,
            jwt_inspection: None,
            jwt_secret_input: String::new(),
            jwt_verification: None,
            detected_format: None,
            convert_target: Format::Base64,
            show_convert_banner: false,
            settings,
            applied_theme: None,
            now: 0.0,
            copy_pulse_at: None,
            banner_fade_start: None,
            large_paste_confirmed: false,
            history_store: HistoryStore::load(history_path().unwrap_or_default(), private_mode),
            show_history_panel: false,
            last_action_was_encode: true,
            history_query: String::new(),
            selected_history_entry: None,
            show_diff_view: false,
            diff_input_a: String::new(),
            diff_input_b: String::new(),
            diff_last_a: String::new(),
            diff_last_b: String::new(),
            diff_result: None,
            diff_is_binary: false,
            diff_error: None,
            show_command_palette: false,
            command_palette_query: String::new(),
            command_palette_selected: 0,
            command_palette_just_opened: false,
            batch_result: None,
            show_batch_panel: false,
            batch_pending_confirmation: None,
            batch_progress: BatchProgress::default(),
            batch_receiver: None,
        }
    }
}

impl Basie64App {
    /// Reset every image-related piece of state at once.
    /// Called alongside the existing `clear`/`decode`/`restore` paths so
    /// the preview, cached bytes, metadata, and dialog never drift apart.
    pub(crate) fn clear_image_state(&mut self) {
        self.image_preview = None;
        self.image_bytes = None;
        self.image_meta = None;
        self.export_image_dialog = None;
    }

    pub fn clear(&mut self) {
        self.input.clear();
        self.output.clear();
        self.error = None;
        self.error_hint = None;
        self.show_banner = false;
        self.mixed_matches.clear();
        self.clear_image_state();
        self.encoded_data_uri = None;
        self.jwt_inspection = None;
        self.jwt_secret_input.clear();
        self.jwt_verification = None;
        self.large_paste_confirmed = false;
        self.detected_format = None;
        self.show_convert_banner = false;
        // convert_target is intentionally kept — it is a user preference
        self.show_diff_view = false;
        self.diff_input_a.clear();
        self.diff_input_b.clear();
        self.diff_last_a.clear();
        self.diff_last_b.clear();
        self.diff_result = None;
        self.diff_error = None;
        self.show_command_palette = false;
        self.command_palette_query.clear();
    }

    /// Decode input and prompt the user to save as a file.
    pub fn save_to_file(&mut self) {
        let b64 = self.input.trim();
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
                            self.output = format!("Saved successfully to {}", path.display());
                            self.error = None;
                            self.error_hint = None;
                            self.settings.push_recent_file(path);
                            self.settings.save();
                        }
                        Err(e) => {
                            self.error = Some(format!("Failed to save file: {}", e));
                        }
                    }
                }
            }
            Err(_) => {
                self.error = Some("Invalid Base64 for file decoding".into());
                self.error_hint = crate::core::decode::infer_hint(b64);
            }
        }
    }

    /// If no image is currently cached, try to decode `self.input` and
    /// populate `image_bytes` / `image_meta` / `image_preview` on the fly.
    /// Returns `true` if an image is ready after the call.
    ///
    /// This keeps the command-palette Export path working even when the
    /// user pastes a Base64 image and jumps straight to Export without
    /// clicking Decode first — matching the pre-refactor behavior.
    fn ensure_image_cached(&mut self, ctx: &egui::Context) -> bool {
        if self.image_bytes.is_some() && self.image_meta.is_some() {
            return true;
        }

        let clean_b64: String = self
            .input
            .trim()
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect();
        let b64_content = if let Some(idx) = clean_b64.find("base64,") {
            &clean_b64[idx + 7..]
        } else {
            clean_b64.as_str()
        };

        let bytes = general_purpose::STANDARD
            .decode(b64_content)
            .or_else(|_| general_purpose::URL_SAFE.decode(b64_content))
            .or_else(|_| general_purpose::URL_SAFE_NO_PAD.decode(b64_content))
            .ok();
        let Some(bytes) = bytes else {
            return false;
        };

        let Ok(img) = image::load_from_memory(&bytes) else {
            return false;
        };

        let size = [img.width() as _, img.height() as _];
        let image_buffer = img.into_rgba8();
        let pixels = image_buffer.as_flat_samples();
        let color_image = egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice());
        self.image_preview =
            Some(ctx.load_texture("preview", color_image, egui::TextureOptions::LINEAR));
        self.image_meta = crate::core::image_meta::inspect(&bytes);
        self.image_bytes = Some(bytes);
        true
    }

    /// Open the Export Image dialog for the currently-decoded image. If no
    /// image is cached, attempt to decode `self.input` first; only set a
    /// friendly error banner if that fails too.
    pub fn open_export_image_dialog(&mut self, ctx: &egui::Context) {
        if !self.ensure_image_cached(ctx) {
            self.error = Some(
                "Current input doesn't decode to an image — paste a Base64-encoded image first."
                    .into(),
            );
            self.error_hint = None;
            return;
        }

        let strip_default = self
            .image_meta
            .as_ref()
            .map(|m| m.has_strippable_metadata && m.strip_supported)
            .unwrap_or(false);

        self.export_image_dialog = Some(ExportImageDialog {
            strip_metadata: strip_default,
            exif_expanded: false,
        });
    }

    /// Close the Export Image dialog without saving.
    pub fn close_export_image_dialog(&mut self) {
        self.export_image_dialog = None;
    }

    /// Run the Export Image dialog's Save action: optionally strip metadata,
    /// then prompt for a destination path and write the bytes.
    pub fn execute_export_image_save(&mut self) {
        let Some(dialog) = self.export_image_dialog.clone() else {
            return;
        };
        let Some(meta) = self.image_meta.clone() else {
            return;
        };
        let Some(source_bytes) = self.image_bytes.clone() else {
            return;
        };

        let bytes_to_write = if dialog.strip_metadata && meta.strip_supported {
            match crate::core::image_meta::strip_metadata(&source_bytes, meta.kind) {
                Ok(stripped) => stripped,
                Err(e) => {
                    self.error = Some(format!("Failed to strip metadata: {}", e));
                    self.error_hint = None;
                    return;
                }
            }
        } else {
            source_bytes
        };

        let extension = meta.kind.extension();
        if let Some(path) = rfd::FileDialog::new()
            .add_filter(meta.kind.label(), &[extension])
            .set_file_name(format!("decoded.{}", extension))
            .save_file()
        {
            match std::fs::write(&path, &bytes_to_write) {
                Ok(()) => {
                    self.output = format!("Saved image to {}", path.display());
                    self.error = None;
                    self.error_hint = None;
                    self.settings.push_recent_file(path);
                    self.settings.save();
                    self.export_image_dialog = None;
                }
                Err(e) => {
                    self.error = Some(format!("Failed to save file: {}", e));
                    self.error_hint = None;
                }
            }
        }
    }

    pub fn mark_copy_pulse(&mut self) {
        self.copy_pulse_at = Some(self.now);
    }

    pub fn run_encode(&mut self) {
        self.output = crate::core::encode::encode_base64(&self.input);
        self.error = None;
        self.error_hint = None;
        self.clear_image_state();
        self.encoded_data_uri = Some(format!("data:text/plain;base64,{}", self.output));
        self.jwt_inspection = None;
        self.jwt_verification = None;
        self.last_action_was_encode = true;
    }

    /// Convert the current input from its detected format to `self.convert_target`.
    /// Writes the result to `self.output`, or sets `self.error` on failure.
    pub fn run_convert(&mut self) {
        use crate::core::convert::convert;
        let Some(from) = self.detected_format else {
            return;
        };
        let to = self.convert_target;
        let input = self.input.trim().to_string();
        match convert(&input, from, to) {
            Ok(result) => {
                self.output = result;
                self.error = None;
                self.error_hint = None;
                self.clear_image_state();
                self.encoded_data_uri = None;
                self.jwt_inspection = None;
                self.jwt_verification = None;
            }
            Err(e) => {
                self.error = Some(format!("Conversion failed: {}", e));
                self.error_hint = None;
            }
        }
    }

    pub fn replay_convert(&mut self, variant: &str) -> bool {
        use crate::core::convert::parse_conversion_variant;

        let Some((from, to)) = parse_conversion_variant(variant) else {
            return false;
        };

        self.detected_format = Some(from);
        self.convert_target = to;
        self.run_convert();
        true
    }

    /// Decode both diff inputs and compute the diff result.
    pub fn run_diff(&mut self) {
        use crate::core::convert::base64_to_bytes;
        self.diff_last_a = self.diff_input_a.clone();
        self.diff_last_b = self.diff_input_b.clone();

        if self.diff_input_a.trim().is_empty() && self.diff_input_b.trim().is_empty() {
            self.diff_result = None;
            self.diff_error = None;
            self.diff_is_binary = false;
            return;
        }

        if self.diff_input_a.trim().is_empty() || self.diff_input_b.trim().is_empty() {
            self.diff_result = None;
            self.diff_error = None;
            self.diff_is_binary = false;
            return;
        }

        let bytes_a = base64_to_bytes(self.diff_input_a.trim());
        let bytes_b = base64_to_bytes(self.diff_input_b.trim());

        match (bytes_a, bytes_b) {
            (Ok(a), Ok(b)) => {
                let text_a = std::str::from_utf8(&a);
                let text_b = std::str::from_utf8(&b);
                if let (Ok(ta), Ok(tb)) = (text_a, text_b) {
                    self.diff_is_binary = false;
                    self.diff_result = Some(diff_text(ta, tb));
                } else {
                    self.diff_is_binary = true;
                    self.diff_result = Some(diff_binary(&a, &b));
                }
                self.diff_error = None;
            }
            (Err(_), _) | (_, Err(_)) => {
                self.diff_is_binary = false;
                self.diff_result = None;
                self.diff_error = Some("Enter valid Base64 in both comparison fields.".to_string());
            }
        }
    }

    pub fn open_diff_view_from_input(&mut self) {
        self.show_diff_view = true;
        self.diff_error = None;
        self.diff_result = None;
        self.diff_is_binary = false;

        let trimmed = self.input.trim();
        if crate::core::convert::base64_to_bytes(trimmed).is_ok() {
            self.diff_input_a = trimmed.to_string();
            self.diff_last_a = self.diff_input_a.clone();
            self.diff_input_b.clear();
            self.diff_last_b.clear();
        } else {
            self.diff_input_a.clear();
            self.diff_input_b.clear();
            self.diff_last_a.clear();
            self.diff_last_b.clear();
        }
    }

    pub fn toggle_command_palette(&mut self) {
        self.show_command_palette = !self.show_command_palette;
        if self.show_command_palette {
            self.command_palette_just_opened = true;
            self.command_palette_selected = 0;
        } else {
            self.command_palette_query.clear();
        }
    }

    pub fn set_private_mode(&mut self, enabled: bool) {
        self.settings.private_mode = enabled;
        self.settings.save();
        self.history_store.set_private_mode(enabled);
    }

    pub fn visible_history_ids(&self) -> Vec<String> {
        self.history_store
            .search(&self.history_query)
            .into_iter()
            .rev()
            .map(|entry| entry.id.clone())
            .collect()
    }

    pub fn ensure_selected_history_entry(&mut self) {
        let visible_ids = self.visible_history_ids();
        if visible_ids.is_empty() {
            self.selected_history_entry = None;
            return;
        }

        let selected_is_visible = self
            .selected_history_entry
            .as_ref()
            .is_some_and(|id| visible_ids.iter().any(|visible| visible == id));

        if !selected_is_visible {
            self.selected_history_entry = visible_ids.first().cloned();
        }
    }

    pub fn step_history_selection(&mut self, delta: isize) {
        let visible_ids = self.visible_history_ids();
        if visible_ids.is_empty() {
            self.selected_history_entry = None;
            return;
        }

        let current_index = self
            .selected_history_entry
            .as_ref()
            .and_then(|selected| visible_ids.iter().position(|id| id == selected))
            .unwrap_or(0);
        let next_index = (current_index as isize + delta).clamp(0, visible_ids.len() as isize - 1);
        self.selected_history_entry = Some(visible_ids[next_index as usize].clone());
    }

    pub fn restore_selected_history_entry(&mut self, ctx: &egui::Context) {
        let Some(selected_id) = self.selected_history_entry.clone() else {
            return;
        };
        let Some(entry) = self.history_store.get_by_id(&selected_id).cloned() else {
            self.ensure_selected_history_entry();
            return;
        };

        self.input = entry.reload_input().to_string();
        self.last_input = self.input.clone();
        self.output.clear();
        self.error = None;
        self.error_hint = None;
        self.show_banner = false;
        self.mixed_matches.clear();
        self.clear_image_state();
        self.encoded_data_uri = None;
        self.jwt_inspection = None;
        self.jwt_secret_input.clear();
        self.jwt_verification = None;
        self.large_paste_confirmed = false;
        self.detected_format = None;
        self.show_convert_banner = false;
        self.show_history_panel = false;

        match entry.op {
            HistoryOp::Decode => {
                let input = self.input.clone();
                self.decode_input_str(ctx, &input);
            }
            HistoryOp::Encode => self.run_encode(),
            HistoryOp::Convert => {
                if !self.replay_convert(&entry.variant) {
                    self.error = Some(format!(
                        "Unable to restore conversion history entry: {}",
                        entry.variant
                    ));
                }
            }
        }
    }

    pub fn delete_selected_history_entry(&mut self) {
        let Some(selected_id) = self.selected_history_entry.clone() else {
            return;
        };
        if self.history_store.remove_by_id(&selected_id) {
            self.ensure_selected_history_entry();
        }
    }

    pub fn request_decode(&mut self, ctx: &egui::Context) {
        if self.input != self.last_input {
            self.large_paste_confirmed = false;
            self.last_input = self.input.clone();
        }

        if self.input.len() > LARGE_PASTE_THRESHOLD && !self.large_paste_confirmed {
            self.error = Some(format!(
                "Input is {:.1} MB — click Decode again to confirm.",
                self.input.len() as f64 / 1_000_000.0
            ));
            self.error_hint = None;
            self.large_paste_confirmed = true;
            return;
        }

        let b64 = self.input.clone();
        self.decode_input_str(ctx, &b64);
        self.large_paste_confirmed = false;
        self.last_action_was_encode = false;
    }

    pub fn is_batch_running(&self) -> bool {
        self.batch_receiver.is_some()
    }

    fn queue_batch(&mut self, config: BatchConfig) {
        let preview = preview_batch(&config);
        self.batch_pending_confirmation = Some(BatchPending { config, preview });
        self.batch_result = None;
        self.batch_progress = BatchProgress::default();
        self.show_batch_panel = true;
    }

    /// Start a batch encode operation for a directory.
    pub fn start_batch_encode(&mut self, input_dir: PathBuf, output_dir: Option<PathBuf>) {
        self.queue_batch(BatchConfig {
            source: BatchSource::directory(input_dir, None),
            output_dir,
            operation: BatchOp::Encode,
            decode_b64_only: true,
        });
    }

    /// Start a batch decode operation for a directory.
    pub fn start_batch_decode(&mut self, input_dir: PathBuf, output_dir: Option<PathBuf>) {
        self.queue_batch(BatchConfig {
            source: BatchSource::directory(input_dir, None),
            output_dir,
            operation: BatchOp::Decode,
            decode_b64_only: true,
        });
    }

    pub fn start_batch_encode_files(&mut self, files: Vec<PathBuf>, output_dir: Option<PathBuf>) {
        self.queue_batch(BatchConfig {
            source: BatchSource::files(files),
            output_dir,
            operation: BatchOp::Encode,
            decode_b64_only: true,
        });
    }

    pub fn start_batch_decode_files(&mut self, files: Vec<PathBuf>, output_dir: Option<PathBuf>) {
        self.queue_batch(BatchConfig {
            source: BatchSource::files(files),
            output_dir,
            operation: BatchOp::Decode,
            decode_b64_only: true,
        });
    }

    pub fn set_pending_batch_output_dir(&mut self, output_dir: Option<PathBuf>) {
        if let Some(pending) = self.batch_pending_confirmation.as_mut() {
            pending.config.output_dir = output_dir;
            pending.preview = preview_batch(&pending.config);
        }
    }

    pub fn set_pending_batch_operation(&mut self, operation: BatchOp) {
        if let Some(pending) = self.batch_pending_confirmation.as_mut() {
            pending.config.operation = operation;
            pending.preview = preview_batch(&pending.config);
        }
    }

    /// Execute the pending batch operation.
    pub fn execute_batch(&mut self) {
        let Some(pending) = self.batch_pending_confirmation.take() else {
            return;
        };

        let config = pending.config;
        let total = pending.preview.file_count;
        let (tx, rx) = mpsc::channel();
        self.batch_receiver = Some(rx);
        self.batch_result = None;
        self.batch_progress = BatchProgress {
            total,
            ..BatchProgress::default()
        };
        self.show_batch_panel = true;

        thread::spawn(move || {
            let progress_tx = tx.clone();
            let result = process_batch_with_progress(&config, move |progress| {
                let _ = progress_tx.send(BatchWorkerMessage::Progress(progress));
            });
            let _ = tx.send(BatchWorkerMessage::Finished(result));
        });
    }

    /// Cancel the pending batch operation.
    pub fn cancel_batch(&mut self) {
        self.batch_pending_confirmation = None;
    }

    /// Clear batch results.
    pub fn clear_batch_results(&mut self) {
        self.batch_result = None;
        self.show_batch_panel = false;
        self.batch_pending_confirmation = None;
        self.batch_progress = BatchProgress::default();
    }

    fn poll_batch_updates(&mut self) {
        let mut finished = false;
        if let Some(receiver) = self.batch_receiver.as_ref() {
            while let Ok(message) = receiver.try_recv() {
                match message {
                    BatchWorkerMessage::Progress(progress) => {
                        self.batch_progress = progress;
                    }
                    BatchWorkerMessage::Finished(result) => {
                        self.batch_progress.processed = result.processed();
                        self.batch_progress.succeeded = result.succeeded();
                        self.batch_progress.failed = result.failed();
                        self.batch_progress.skipped = result.skipped();
                        self.batch_progress.total = result.processed();
                        self.batch_progress.current = None;
                        self.batch_result = Some(result);
                        finished = true;
                    }
                }
            }
        }

        if finished {
            self.batch_receiver = None;
        }
    }

    fn load_single_file(&mut self, path: &PathBuf) {
        if let Ok(bytes) = fs::read(path) {
            self.input = format!(
                "[File: {}]\n(Size: {} bytes)\n",
                path.display(),
                bytes.len()
            );
            self.output = general_purpose::STANDARD.encode(&bytes);
            let mime_type = infer::get(&bytes)
                .map(|t| t.mime_type())
                .unwrap_or("application/octet-stream");
            self.encoded_data_uri = Some(format!("data:{};base64,{}", mime_type, self.output));
            self.jwt_inspection = None;
            self.jwt_verification = None;
            self.error = None;
            self.error_hint = None;
            self.show_banner = false;
            self.mixed_matches.clear();
            self.clear_image_state();
            self.settings.push_recent_file(path.clone());
            self.settings.save();
        }
    }

    pub fn handle_dropped_paths(&mut self, mut paths: Vec<PathBuf>) {
        if self.is_batch_running() {
            self.error = Some("A batch operation is already running.".to_string());
            self.error_hint = None;
            return;
        }

        paths.retain(|path| path.exists());
        if paths.is_empty() {
            return;
        }

        let directories: Vec<_> = paths.iter().filter(|path| path.is_dir()).cloned().collect();
        let files: Vec<_> = paths
            .iter()
            .filter(|path| path.is_file())
            .cloned()
            .collect();

        match (directories.len(), files.len()) {
            (0, 1) => self.load_single_file(&files[0]),
            (0, _) => self.start_batch_encode_files(files, None),
            (1, 0) => self.start_batch_encode(directories[0].clone(), None),
            _ => {
                self.error = Some(
                    "Batch drop accepts one folder or one or more files, not a mixed selection."
                        .to_string(),
                );
                self.error_hint = None;
            }
        }
    }
}

impl eframe::App for Basie64App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.now = ctx.input(|i| i.time);
        self.poll_batch_updates();

        // Apply theme if it changed (or first frame).
        if self.applied_theme != Some(self.settings.theme) {
            theme::apply(ctx, self.settings.theme);
            self.applied_theme = Some(self.settings.theme);
        }

        // Keyboard shortcuts. While the Export Image dialog is open it acts
        // as a true modal — only Escape is honored, everything else is
        // suppressed so the user can't mutate state behind the overlay.
        let dialog_modal_active = self.export_image_dialog.is_some();
        ctx.input(|i| {
            if i.key_pressed(egui::Key::Escape) {
                if self.show_command_palette {
                    self.show_command_palette = false;
                    self.command_palette_query.clear();
                } else if dialog_modal_active {
                    self.close_export_image_dialog();
                } else {
                    self.clear();
                }
            }

            if dialog_modal_active {
                return;
            }

            if i.modifiers.command && i.key_pressed(egui::Key::Enter) {
                self.request_decode(ctx);
            }
            if i.modifiers.command && i.key_pressed(egui::Key::K) {
                self.toggle_command_palette();
            }
            if i.modifiers.command
                && i.modifiers.shift
                && i.key_pressed(egui::Key::C)
                && !self.output.is_empty()
            {
                ctx.copy_text(self.output.clone());
                self.copy_pulse_at = Some(self.now);
            }
            if i.modifiers.command && i.key_pressed(egui::Key::H) {
                self.show_history_panel = !self.show_history_panel;
            }
            if i.modifiers.command && i.key_pressed(egui::Key::D) {
                if self.show_diff_view {
                    self.show_diff_view = false;
                } else {
                    self.open_diff_view_from_input();
                }
            }
            if self.show_history_panel {
                if i.key_pressed(egui::Key::ArrowDown) {
                    self.step_history_selection(1);
                }
                if i.key_pressed(egui::Key::ArrowUp) {
                    self.step_history_selection(-1);
                }
                if i.key_pressed(egui::Key::Enter) && !i.modifiers.command {
                    self.restore_selected_history_entry(ctx);
                }
                if i.key_pressed(egui::Key::Delete) {
                    self.delete_selected_history_entry();
                }
            }
        });

        // Drag-drop files
        let dropped_paths: Vec<_> = ctx.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .filter_map(|file| file.path.clone())
                .collect()
        });
        if !dropped_paths.is_empty() {
            self.handle_dropped_paths(dropped_paths);
        }

        detect::run_detection(self);

        ui::top_bar::show(self, ctx);
        show_status_footer(self, ctx);

        if self.show_diff_view {
            ui::diff_view::show(self, ctx);
        } else {
            egui::CentralPanel::default().show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.add_space(10.0);
                    ui::banner::show(self, ctx, ui);
                    ui::banner::show_convert_hint(self, ctx, ui);
                    ui::banner::show_mixed_matches(self, ctx, ui);
                    ui::input::show(self, ui);
                    ui.add_space(12.0);
                    ui::buttons::show(self, ctx, ui);
                    ui.add_space(12.0);
                    ui::output::show(self, ctx, ui);
                    ui::banner::show_error(self, ctx, ui);
                });
            });
        }

        // History panel (bottom panel)
        if self.show_history_panel {
            self.ensure_selected_history_entry();
            ui::history_panel::show(self, ctx);
        }

        // Batch panel (bottom panel)
        if self.show_batch_panel {
            ui::batch_panel::show(self, ctx);
        }

        if self.show_command_palette {
            ui::command_palette::show(self, ctx);
        }

        if self.export_image_dialog.is_some() {
            ui::export_image_dialog::show(self, ctx);
        }

        // Keep animations ticking
        if self.copy_pulse_at.is_some()
            || ui::banner::is_fade_active(self.banner_fade_start, self.now)
            || self.is_batch_running()
        {
            ctx.request_repaint();
        }

        if !ui::banner::is_fade_active(self.banner_fade_start, self.now) {
            self.banner_fade_start = None;
        }
    }
}

/// Persistent 32px bottom panel that surfaces the main keyboard shortcuts.
/// Renders `bg_surface` with an upward shadow for Arc-style floating depth.
fn show_status_footer(app: &Basie64App, ctx: &egui::Context) {
    let tokens = crate::theme::Tokens::for_theme(app.settings.theme);

    let frame = egui::Frame::new()
        .fill(tokens.bg_surface)
        .shadow(tokens.shadow_up)
        .inner_margin(egui::Margin {
            left: 16,
            right: 16,
            top: 0,
            bottom: 0,
        });

    egui::TopBottomPanel::bottom("status_footer")
        .frame(frame)
        .exact_height(32.0)
        .resizable(false)
        .show_separator_line(false)
        .show(ctx, |ui| {
            ui.horizontal_centered(|ui| {
                ui.spacing_mut().item_spacing.x = 16.0;
                ui::widgets::key_chip(ui, "⌘↵", "encode/decode", false);
                ui::widgets::key_chip(ui, "⌘D", "diff", app.show_diff_view);
                ui::widgets::key_chip(ui, "⌘K", "commands", app.show_command_palette);
                ui::widgets::key_chip(ui, "⌘H", "history", app.show_history_panel);
            });
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::batch::BatchSourceKind;
    use tempfile::TempDir;

    fn temp_history_store() -> (HistoryStore, tempfile::NamedTempFile) {
        let file = tempfile::NamedTempFile::new().expect("temp file");
        (HistoryStore::load(file.path().to_path_buf(), false), file)
    }

    #[test]
    fn regex_compiles_via_default() {
        let app = Basie64App::default();
        assert!(app.base64_regex.is_match("SGVsbG8sIHdvcmxkIQ=="));
    }

    fn tiny_png_bytes() -> Vec<u8> {
        let img = image::RgbaImage::from_pixel(2, 2, image::Rgba([10, 20, 30, 255]));
        let mut buf = Vec::new();
        img.write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)
            .expect("png encode");
        buf
    }

    #[test]
    fn open_export_image_dialog_decodes_fresh_input() {
        // Regression: the command-palette Export path must work when the
        // user pastes a Base64 image and jumps straight to Export without
        // clicking Decode first. Before the fix, `open_export_image_dialog`
        // errored because `image_bytes`/`image_meta` were None.
        let mut app = Basie64App::default();
        let ctx = egui::Context::default();
        let png = tiny_png_bytes();
        app.input = general_purpose::STANDARD.encode(&png);

        assert!(app.image_bytes.is_none());
        assert!(app.image_meta.is_none());
        assert!(app.export_image_dialog.is_none());

        app.open_export_image_dialog(&ctx);

        assert!(
            app.export_image_dialog.is_some(),
            "dialog should open after eager decode"
        );
        assert!(
            app.image_bytes.is_some(),
            "raw image bytes should now be cached"
        );
        assert!(app.image_meta.is_some(), "metadata should now be populated");
        assert!(app.error.is_none(), "no error banner expected");
    }

    #[test]
    fn open_export_image_dialog_rejects_non_image_input() {
        let mut app = Basie64App::default();
        let ctx = egui::Context::default();
        app.input = "SGVsbG8sIHdvcmxkIQ==".into(); // valid base64, not an image

        app.open_export_image_dialog(&ctx);

        assert!(
            app.export_image_dialog.is_none(),
            "dialog must not open for non-image input"
        );
        assert!(app.image_bytes.is_none());
        assert!(
            app.error.as_ref().is_some_and(|e| e.contains("image")),
            "error banner should explain the problem"
        );
    }

    #[test]
    fn open_export_image_dialog_reuses_cached_image_without_redecoding() {
        // If image state is already populated (e.g. after Decode), opening
        // the dialog should not touch self.input or re-run the decode.
        let mut app = Basie64App::default();
        let ctx = egui::Context::default();
        let png = tiny_png_bytes();
        app.input = general_purpose::STANDARD.encode(&png);
        app.decode_input_str(&ctx, &app.input.clone());
        assert!(app.image_bytes.is_some());

        // Now stomp the input — cached state should still drive the dialog.
        app.input = "garbage not base64".into();
        app.open_export_image_dialog(&ctx);

        assert!(app.export_image_dialog.is_some());
        assert!(app.error.is_none());
    }

    #[test]
    fn encode_text_roundtrip() {
        let input = "Hello, world!";
        let expected = "SGVsbG8sIHdvcmxkIQ==";
        assert_eq!(general_purpose::STANDARD.encode(input), expected);
    }

    #[test]
    fn decode_valid_text() {
        let mut app = Basie64App::default();
        let ctx = egui::Context::default();
        app.decode_input_str(&ctx, "SGVsbG8sIHdvcmxkIQ==");
        assert_eq!(app.output, "Hello, world!");
        assert!(app.error.is_none());
        assert!(app.image_preview.is_none());
    }

    #[test]
    fn decode_invalid() {
        let mut app = Basie64App::default();
        let ctx = egui::Context::default();
        app.decode_input_str(&ctx, "not_valid_b64!!_");
        assert!(app.error.is_some());
        assert!(app.error.as_ref().unwrap().contains("Invalid Base64"));
        assert!(app.image_preview.is_none());
    }

    #[test]
    fn decode_binary_not_utf8() {
        let mut app = Basie64App::default();
        let ctx = egui::Context::default();
        app.decode_input_str(&ctx, "////");
        assert!(app.output.contains("Decoded 3 binary bytes"));
        assert!(app.error.is_none());
    }

    #[test]
    fn decode_jwt() {
        let mut app = Basie64App::default();
        let ctx = egui::Context::default();
        let header =
            general_purpose::URL_SAFE_NO_PAD.encode(b"{\"alg\":\"HS256\",\"typ\":\"JWT\"}");
        let payload = general_purpose::URL_SAFE_NO_PAD
            .encode(b"{\"sub\":\"1234567890\",\"name\":\"John Doe\",\"iat\":1516239022}");
        let signature = "SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";
        let jwt = format!("{}.{}.{}", header, payload, signature);
        app.decode_input_str(&ctx, &jwt);
        assert!(app.output.contains("JWT Detected"));
        assert!(app.output.contains("John Doe"));
        assert!(app.error.is_none());
    }

    #[test]
    fn decode_data_uri() {
        let mut app = Basie64App::default();
        let ctx = egui::Context::default();
        let valid_b64 = "SGVsbG8sIHdvcmxkIQ==";
        let data_uri = format!("data:text/plain;base64,{}", valid_b64);
        app.decode_input_str(&ctx, &data_uri);
        assert_eq!(app.output, "Hello, world!");
        assert!(app.error.is_none());
    }

    #[test]
    fn decode_url_safe() {
        let mut app = Basie64App::default();
        let ctx = egui::Context::default();
        let url_safe = general_purpose::URL_SAFE.encode(b"hello world!?");
        app.decode_input_str(&ctx, &url_safe);
        assert_eq!(app.output, "hello world!?");
        assert!(app.error.is_none());
    }

    #[test]
    fn large_input_requires_confirmation() {
        let mut app = Basie64App::default();
        let ctx = egui::Context::default();

        app.input = "A".repeat(LARGE_PASTE_THRESHOLD + 4);

        app.request_decode(&ctx);
        assert!(app
            .error
            .as_ref()
            .is_some_and(|err| err.contains("click Decode again")));
        assert!(app.output.is_empty());
        assert!(app.large_paste_confirmed);

        app.request_decode(&ctx);
        assert!(!app.output.is_empty());
        assert!(app.error.is_none());
        assert!(!app.large_paste_confirmed);
    }

    #[test]
    fn run_diff_rejects_invalid_base64_inputs() {
        let mut app = Basie64App {
            diff_input_a: "SGVsbG8=".into(),
            diff_input_b: "not base64".into(),
            ..Default::default()
        };

        app.run_diff();

        assert!(app.diff_result.is_none());
        assert_eq!(
            app.diff_error.as_deref(),
            Some("Enter valid Base64 in both comparison fields.")
        );
    }

    #[test]
    fn open_diff_view_from_input_seeds_valid_base64_into_left_side() {
        let mut app = Basie64App {
            input: "SGVsbG8=".into(),
            ..Default::default()
        };

        app.open_diff_view_from_input();

        assert!(app.show_diff_view);
        assert_eq!(app.diff_input_a, "SGVsbG8=");
        assert!(app.diff_input_b.is_empty());
        assert!(app.diff_result.is_none());
        assert!(app.diff_error.is_none());
    }

    #[test]
    fn large_input_confirmation_resets_after_input_change() {
        let mut app = Basie64App::default();
        let ctx = egui::Context::default();

        app.input = "A".repeat(LARGE_PASTE_THRESHOLD + 4);
        app.request_decode(&ctx);
        assert!(app.large_paste_confirmed);

        app.input = "B".repeat(LARGE_PASTE_THRESHOLD + 4);
        app.request_decode(&ctx);

        assert!(app
            .error
            .as_ref()
            .is_some_and(|err| err.contains("click Decode again")));
        assert!(app.output.is_empty());
        assert!(app.large_paste_confirmed);
    }

    #[test]
    fn history_search_does_not_mutate_input() {
        let mut app = Basie64App::default();
        let (store, _file) = temp_history_store();
        app.history_store = store;
        app.input = "SGVsbG8=".into();
        app.history_query = "Hello".into();

        let visible = app.visible_history_ids();

        assert!(visible.is_empty());
        assert_eq!(app.input, "SGVsbG8=");
    }

    #[test]
    fn restore_decode_history_uses_full_input() {
        let mut app = Basie64App::default();
        let (store, _file) = temp_history_store();
        app.history_store = store;
        let ctx = egui::Context::default();
        let plain_text = "a".repeat(120);
        let input = general_purpose::STANDARD.encode(&plain_text);
        app.decode_input_str(&ctx, &input);
        let entry_id = app.history_store.entries()[0].id.clone();

        app.input.clear();
        app.output.clear();
        app.selected_history_entry = Some(entry_id);
        app.restore_selected_history_entry(&ctx);

        assert_eq!(app.input, input);
        assert_eq!(app.output, plain_text);
    }

    #[test]
    fn restore_encode_history_recomputes_output() {
        let mut app = Basie64App::default();
        let (store, _file) = temp_history_store();
        app.history_store = store;
        let original_input = "encode me".repeat(20);
        app.input = original_input.clone();
        app.run_encode();
        let expected_output = app.output.clone();
        let entry = crate::core::history::HistoryEntry::new(
            HistoryOp::Encode,
            &app.input,
            &app.output,
            "standard",
        );
        let entry_id = entry.id.clone();
        app.history_store.append(entry);

        app.input.clear();
        app.output.clear();
        app.selected_history_entry = Some(entry_id);
        app.restore_selected_history_entry(&egui::Context::default());

        assert_eq!(app.input, original_input);
        assert_eq!(app.output, expected_output);
    }

    #[test]
    fn restore_convert_history_recomputes_output() {
        let mut app = Basie64App::default();
        let (store, _file) = temp_history_store();
        app.history_store = store;
        app.input = "48656c6c6f".into();
        app.detected_format = Some(Format::Hex);
        app.convert_target = Format::Base64;
        app.run_convert();
        let expected_output = app.output.clone();
        let entry = crate::core::history::HistoryEntry::new(
            HistoryOp::Convert,
            &app.input,
            &app.output,
            "Hex → Base64",
        );
        let entry_id = entry.id.clone();
        app.history_store.append(entry);

        app.input.clear();
        app.output.clear();
        app.selected_history_entry = Some(entry_id);
        app.restore_selected_history_entry(&egui::Context::default());

        assert_eq!(app.input, "48656c6c6f");
        assert_eq!(app.output, expected_output);
    }

    #[test]
    fn switching_pending_batch_operation_updates_preview() {
        let tmp = TempDir::new().expect("temp dir");
        let root = tmp.path().to_path_buf();
        std::fs::write(root.join("one.txt"), "one").expect("write one");
        std::fs::write(root.join("two.txt.b64"), "dHdv").expect("write two");

        let mut app = Basie64App::default();
        app.start_batch_encode(root.clone(), None);
        assert_eq!(
            app.batch_pending_confirmation
                .as_ref()
                .map(|pending| pending.preview.operation),
            Some(BatchOp::Encode)
        );

        app.set_pending_batch_operation(BatchOp::Decode);

        let pending = app
            .batch_pending_confirmation
            .as_ref()
            .expect("pending batch");
        assert_eq!(pending.preview.operation, BatchOp::Decode);
        assert_eq!(pending.preview.eligible_count, 1);
    }

    #[test]
    fn dropping_multiple_files_starts_file_batch() {
        let tmp = TempDir::new().expect("temp dir");
        let root = tmp.path().to_path_buf();
        let first = root.join("first.txt");
        let second = root.join("second.txt");
        std::fs::write(&first, "first").expect("write first");
        std::fs::write(&second, "second").expect("write second");

        let mut app = Basie64App::default();
        app.handle_dropped_paths(vec![first, second]);

        let pending = app
            .batch_pending_confirmation
            .as_ref()
            .expect("pending batch");
        assert_eq!(pending.preview.source_kind, BatchSourceKind::Files);
        assert_eq!(pending.preview.selection_count, 2);
        assert_eq!(pending.preview.file_count, 2);
    }
}
