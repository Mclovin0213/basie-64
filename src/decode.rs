use crate::app::Basie64App;
use crate::core::decode::{decode_base64, DecodeOutput};
use crate::core::history::{HistoryEntry, HistoryOp};
use eframe::egui;

impl Basie64App {
    pub fn decode_input_str(&mut self, ctx: &egui::Context, b64: &str) {
        match decode_base64(b64) {
            Ok((output, variant)) => {
                match &output {
                    DecodeOutput::Jwt(inspection) => {
                        self.output = inspection.to_display_string();
                        self.jwt_inspection = Some((**inspection).clone());
                        self.jwt_verification = None;
                        self.clear_image_state();
                    }
                    DecodeOutput::Text(s) => {
                        self.output = s.clone();
                        self.jwt_inspection = None;
                        self.jwt_secret_input.clear();
                        self.jwt_verification = None;
                        self.clear_image_state();
                    }
                    DecodeOutput::Binary { bytes, summary } => {
                        self.output = summary.clone();
                        self.jwt_inspection = None;
                        self.jwt_secret_input.clear();
                        self.jwt_verification = None;

                        if let Ok(img) = image::load_from_memory(bytes) {
                            let size = [img.width() as _, img.height() as _];
                            let image_buffer = img.into_rgba8();
                            let pixels = image_buffer.as_flat_samples();
                            let color_image =
                                egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice());
                            self.image_preview = Some(ctx.load_texture(
                                "preview",
                                color_image,
                                egui::TextureOptions::LINEAR,
                            ));
                            self.image_meta = crate::core::image_meta::inspect(bytes);
                            self.image_bytes = Some(bytes.clone());
                            self.export_image_dialog = None;
                        } else {
                            self.clear_image_state();
                        }
                    }
                }

                self.error = None;
                self.error_hint = None;
                self.encoded_data_uri = None;

                self.history_store.append(HistoryEntry::new(
                    HistoryOp::Decode,
                    b64,
                    &self.output,
                    variant,
                ));
            }
            Err(e) => {
                self.error = Some(e.message);
                self.error_hint = e.hint;
                self.clear_image_state();
                self.encoded_data_uri = None;
                self.jwt_inspection = None;
                self.jwt_secret_input.clear();
                self.jwt_verification = None;
            }
        }
    }
}
