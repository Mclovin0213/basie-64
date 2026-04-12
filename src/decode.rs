use crate::app::Basie64App;
use crate::core::decode::{decode_base64, DecodeOutput};
use crate::core::history::{HistoryEntry, HistoryOp};
use eframe::egui;

impl Basie64App {
    pub fn decode_input_str(&mut self, ctx: &egui::Context, b64: &str) {
        match decode_base64(b64) {
            Ok((output, variant)) => {
                match &output {
                    DecodeOutput::Jwt { formatted } => {
                        self.output = formatted.clone();
                        self.image_preview = None;
                    }
                    DecodeOutput::Text(s) => {
                        self.output = s.clone();
                        self.image_preview = None;
                    }
                    DecodeOutput::Binary { bytes, summary } => {
                        self.output = summary.clone();

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
                        } else {
                            self.image_preview = None;
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
                self.image_preview = None;
                self.encoded_data_uri = None;
            }
        }
    }
}
