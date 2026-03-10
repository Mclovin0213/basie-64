use base64::{engine::general_purpose, Engine as _};
use eframe::egui;
use image::GenericImageView;

fn load_icon() -> egui::IconData {
    let icon_bytes = include_bytes!("../icon.png");
    let image = image::load_from_memory(icon_bytes).expect("Failed to load icon from memory");
    let (width, height) = image.dimensions();
    let rgba = image.into_rgba8().into_raw();

    egui::IconData {
        rgba,
        width,
        height,
    }
}

fn setup_custom_visuals(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    
    // Customize accent colors to be slightly more aesthetic (like a soft purple/blue)
    visuals.selection.bg_fill = egui::Color32::from_rgb(108, 113, 196);
    
    ctx.set_visuals(visuals);
}

fn main() -> eframe::Result {
    let icon = load_icon();

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([650.0, 500.0])
            .with_title("Basie-64")
            .with_icon(icon),
        ..Default::default()
    };

    eframe::run_native(
        "Basie-64",
        native_options,
        Box::new(|cc| {
            setup_custom_visuals(&cc.egui_ctx);
            // egui_extras::install_image_loaders(&cc.egui_ctx); // will need this later
            Ok(Box::new(Basie64App::default()))
        }),
    )
}

use std::fs;
use regex::Regex;

struct Basie64App {
    input: String,
    last_input: String,
    output: String,
    error: Option<String>,
    show_banner: bool,
    banner_message: String,
    base64_regex: Regex,
    mixed_matches: Vec<String>,
    image_preview: Option<egui::TextureHandle>,
}

impl Default for Basie64App {
    fn default() -> Self {
        Self {
            input: String::new(),
            last_input: String::new(),
            output: String::new(),
            error: None,
            show_banner: false,
            banner_message: String::new(),
            // Regex for a reasonable base64 string (at least 16 chars to reduce false positives on small words)
            base64_regex: Regex::new(r"(?x) (?:[A-Za-z0-9+/]{4}){4,} (?:[A-Za-z0-9+/]{2}== | [A-Za-z0-9+/]{3}=)?").unwrap(),
            mixed_matches: Vec::new(),
            image_preview: None,
        }
    }
}

impl Basie64App {
    fn decode_input_str(&mut self, ctx: &egui::Context, b64: &str) {
        match general_purpose::STANDARD.decode(b64.trim()) {
            Ok(bytes) => {
                // Try text
                match String::from_utf8(bytes.clone()) {
                    Ok(s) => {
                        self.output = s;
                        self.error = None;
                    }
                    Err(_) => {
                        self.output = format!("Decoded {} binary bytes (Not valid UTF-8).", bytes.len());
                        self.error = None;
                    }
                }
                
                // Try Image
                if let Ok(img) = image::load_from_memory(&bytes) {
                    let size = [img.width() as _, img.height() as _];
                    let image_buffer = img.into_rgba8();
                    let pixels = image_buffer.as_flat_samples();
                    let color_image = egui::ColorImage::from_rgba_unmultiplied(
                        size,
                        pixels.as_slice(),
                    );
                    self.image_preview = Some(ctx.load_texture(
                        "preview",
                        color_image,
                        egui::TextureOptions::LINEAR
                    ));
                } else {
                    self.image_preview = None;
                }
            }
            Err(e) => {
                self.error = Some(format!("Invalid Base64: {}", e));
                self.image_preview = None;
            }
        }
    }
}

impl eframe::App for Basie64App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Handle drag and drop files
        ctx.input(|i| {
            if let Some(file) = i.raw.dropped_files.first() {
                if let Some(path) = &file.path {
                    if let Ok(bytes) = fs::read(path) {
                        self.input = format!("[File: {}]\n(Size: {} bytes)\n", path.display(), bytes.len());
                        self.output = general_purpose::STANDARD.encode(&bytes);
                        self.error = None;
                        self.show_banner = false;
                        self.mixed_matches.clear();
                        self.image_preview = None;
                    }
                }
            }
        });

        // Run detection if input changed
        if self.input != self.last_input {
            self.last_input = self.input.clone();
            self.show_banner = false;
            self.mixed_matches.clear();
            
            let trimmed = self.input.trim();
            if !trimmed.is_empty() {
                // Check if the entire string might be plain base64
                let is_plain_b64 = self.base64_regex.is_match(trimmed) && trimmed.len() % 4 == 0 && !trimmed.contains(' ');
                
                if is_plain_b64 && general_purpose::STANDARD.decode(trimmed).is_ok() {
                    self.show_banner = true;
                    self.banner_message = "Looks like valid Base64!".to_string();
                } else {
                    // Check for mixed content
                    for mat in self.base64_regex.find_iter(trimmed) {
                        let matched_str = mat.as_str();
                        if general_purpose::STANDARD.decode(matched_str).is_ok() {
                            self.mixed_matches.push(matched_str.to_string());
                        }
                    }
                }
            }
        }

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.heading("🎷 Basie-64");
                ui.separator();
                ui.label("Encode / Decode Base64");
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.add_space(10.0);

                if self.show_banner {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("✨ ").color(egui::Color32::from_rgb(255, 204, 0)));
                        ui.label(egui::RichText::new(&self.banner_message).strong());
                        if ui.button("Yes, decode it!").clicked() {
                            let b64 = self.input.clone();
                            self.decode_input_str(ctx, &b64);
                        }
                    });
                    ui.add_space(10.0);
                }

                if !self.mixed_matches.is_empty() {
                    ui.group(|ui| {
                        ui.label(egui::RichText::new(format!("🔍 Found {} potential Base64 strings in text:", self.mixed_matches.len())).strong());
                        let mut match_to_decode = None;
                        for (i, m) in self.mixed_matches.iter().enumerate().take(5) {
                            ui.horizontal(|ui| {
                                ui.label(format!("{}: {}...", i + 1, &m.chars().take(20).collect::<String>()));
                                if ui.button("Decode").clicked() {
                                    match_to_decode = Some(m.clone());
                                }
                            });
                        }
                        if let Some(m) = match_to_decode {
                            self.input = m.clone();
                            self.decode_input_str(ctx, &m);
                        }
                        if self.mixed_matches.len() > 5 {
                            ui.label(format!("...and {} more", self.mixed_matches.len() - 5));
                        }
                    });
                    ui.add_space(10.0);
                }

                egui::Frame::group(ui.style()).show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.heading("Input");
                        ui.add_space(4.0);
                        ui.add(
                            egui::TextEdit::multiline(&mut self.input)
                                .hint_text("Enter text, plain Base64, or mixed logs...")
                                .desired_width(f32::INFINITY)
                                .desired_rows(6),
                        );
                    });
                });

                ui.add_space(12.0);

                ui.horizontal(|ui| {
                    if ui.button("Encode → Base64").clicked() {
                        self.output = general_purpose::STANDARD.encode(&self.input);
                        self.error = None;
                    }

                    if ui.button("Decode → Text / Image").clicked() {
                        let b64 = self.input.clone();
                        self.decode_input_str(ctx, &b64);
                    }

                    if ui.button("Save as File...").clicked() {
                        let b64 = self.input.trim();
                        // For dropped files, extracting real base64 would be ideal, but for now we decode simple base64 inputs to files
                        if let Ok(bytes) = general_purpose::STANDARD.decode(b64) {
                            let extension = infer::get(&bytes).map(|k| k.extension()).unwrap_or("bin");
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter("Decoded", &[extension])
                                .save_file() {
                                if let Err(e) = std::fs::write(&path, bytes) {
                                    self.error = Some(format!("Failed to save file: {}", e));
                                } else {
                                    self.output = format!("Saved successfully to {}", path.display());
                                    self.error = None;
                                }
                            }
                        } else {
                            self.error = Some("Invalid Base64 for file decoding".into());
                        }
                    }

                    if ui.button("Clear").clicked() {
                        self.input.clear();
                        self.output.clear();
                        self.error = None;
                        self.show_banner = false;
                        self.mixed_matches.clear();
                        self.image_preview = None;
                    }
                });

                ui.add_space(12.0);

                egui::Frame::group(ui.style()).show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.heading("Output");
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.button("📋 Copy").clicked() {
                                    ctx.copy_text(self.output.clone());
                                }
                            });
                        });
                        ui.add_space(4.0);
                        ui.add(
                            egui::TextEdit::multiline(&mut self.output)
                                .interactive(false)
                                .desired_width(f32::INFINITY)
                                .desired_rows(6),
                        );
                        
                        if let Some(texture) = &self.image_preview {
                            ui.add_space(8.0);
                            ui.label("Image Preview:");
                            ui.add(egui::Image::new(texture).max_width(ui.available_width()));
                        }
                    });
                });

                if let Some(err) = &self.error {
                    ui.add_space(8.0);
                    ui.colored_label(egui::Color32::LIGHT_RED, format!("⚠️ Error: {}", err));
                }
            });
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_regex_matching_valid() {
        let app = Basie64App::default();
        let valid_b64 = "SGVsbG8sIHdvcmxkIQ==";
        assert!(app.base64_regex.is_match(valid_b64));
    }

    #[test]
    fn test_encode_text() {
        // The encode function is directly bound to UI, but we can verify the same logic
        let input = "Hello, world!";
        let expected = "SGVsbG8sIHdvcmxkIQ==";
        assert_eq!(general_purpose::STANDARD.encode(input), expected);
    }

    #[test]
    fn test_regex_matching_mixed_content() {
        let app = Basie64App::default();
        let log = "Error at line 42: data=SGVsbG8sIHdvcmxkIQ== status=fail fallback=YW5vdGhlciBzdHJpbmc=";
        let matches: Vec<&str> = app.base64_regex.find_iter(log).map(|m| m.as_str()).collect();
        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0], "SGVsbG8sIHdvcmxkIQ==");
        assert_eq!(matches[1], "YW5vdGhlciBzdHJpbmc=");
    }

    #[test]
    fn test_decode_valid_text() {
        let mut app = Basie64App::default();
        let ctx = egui::Context::default();
        app.decode_input_str(&ctx, "SGVsbG8sIHdvcmxkIQ==");
        
        assert_eq!(app.output, "Hello, world!");
        assert!(app.error.is_none());
        assert!(app.image_preview.is_none());
    }

    #[test]
    fn test_decode_invalid() {
        let mut app = Basie64App::default();
        let ctx = egui::Context::default();
        app.decode_input_str(&ctx, "not_valid_b64!!_");
        
        assert!(app.error.is_some());
        assert!(app.error.as_ref().unwrap().contains("Invalid Base64"));
        assert!(app.image_preview.is_none());
    }

    #[test]
    fn test_decode_binary_not_utf8() {
        let mut app = Basie64App::default();
        let ctx = egui::Context::default();
        
        // This decodes to bytes: [255, 255, 255] which is invalid utf8
        app.decode_input_str(&ctx, "////");
        
        assert!(app.output.contains("Decoded 3 binary bytes"));
        assert!(app.error.is_none());
        assert!(app.image_preview.is_none());
    }
}
