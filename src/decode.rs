use crate::app::Basie64App;
use crate::core::history::{HistoryEntry, HistoryOp};
use base64::{engine::general_purpose, Engine as _};
use eframe::egui;

/// A hint suggested to the user when decoding fails, pointing at a fix.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeHint {
    /// Input contains `-` or `_`; suggest the URL-safe variant.
    TryUrlSafe,
    /// Input length isn't a multiple of 4; suggest the no-padding variant.
    TryNoPadding,
    /// Input contains whitespace; offer a strip-and-retry action.
    StripWhitespace,
}

impl DecodeHint {
    pub fn message(&self) -> &'static str {
        match self {
            DecodeHint::TryUrlSafe => "This looks like URL-safe Base64. Try the URL-safe variant.",
            DecodeHint::TryNoPadding => {
                "Input length isn't a multiple of 4 — it may be using no-padding encoding."
            }
            DecodeHint::StripWhitespace => "Input contains whitespace. Strip it and retry?",
        }
    }

    pub fn action_label(&self) -> Option<&'static str> {
        match self {
            DecodeHint::StripWhitespace => Some("Strip whitespace & retry"),
            _ => None,
        }
    }
}

pub fn infer_hint(raw: &str) -> Option<DecodeHint> {
    let has_whitespace = raw.chars().any(|c| c.is_whitespace());
    let cleaned: String = raw.chars().filter(|c| !c.is_whitespace()).collect();

    if cleaned.contains('-') || cleaned.contains('_') {
        return Some(DecodeHint::TryUrlSafe);
    }
    if !cleaned.is_empty() && !cleaned.len().is_multiple_of(4) {
        return Some(DecodeHint::TryNoPadding);
    }
    if has_whitespace {
        return Some(DecodeHint::StripWhitespace);
    }
    None
}

impl Basie64App {
    pub fn decode_input_str(&mut self, ctx: &egui::Context, b64: &str) {
        let clean_b64 = b64.replace(|c: char| c.is_whitespace(), "");
        let b64_content = if let Some(idx) = clean_b64.find("base64,") {
            &clean_b64[idx + 7..]
        } else {
            clean_b64.as_str()
        };

        // Attempt JWT first
        let parts: Vec<&str> = b64_content.split('.').collect();
        if parts.len() == 3 {
            if let (Ok(header), Ok(payload)) = (
                general_purpose::URL_SAFE_NO_PAD
                    .decode(parts[0])
                    .or_else(|_| general_purpose::URL_SAFE.decode(parts[0])),
                general_purpose::URL_SAFE_NO_PAD
                    .decode(parts[1])
                    .or_else(|_| general_purpose::URL_SAFE.decode(parts[1])),
            ) {
                if let (Ok(header_str), Ok(payload_str)) =
                    (String::from_utf8(header), String::from_utf8(payload))
                {
                    let mut formatted = String::from("JWT Detected:\n\nHeader:\n");
                    if let Ok(h_json) = serde_json::from_str::<serde_json::Value>(&header_str) {
                        formatted
                            .push_str(&serde_json::to_string_pretty(&h_json).unwrap_or(header_str));
                    } else {
                        formatted.push_str(&header_str);
                    }
                    formatted.push_str("\n\nPayload:\n");
                    if let Ok(p_json) = serde_json::from_str::<serde_json::Value>(&payload_str) {
                        formatted.push_str(
                            &serde_json::to_string_pretty(&p_json).unwrap_or(payload_str),
                        );
                    } else {
                        formatted.push_str(&payload_str);
                    }
                    formatted.push_str(&format!("\n\nSignature:\n{}\n", parts[2]));

                    self.output = formatted;
                    self.error = None;
                    self.error_hint = None;
                    self.image_preview = None;
                    self.encoded_data_uri = None;
                    self.history_store.append(HistoryEntry::new(
                        HistoryOp::Decode,
                        b64,
                        &self.output,
                        "jwt",
                    ));
                    return;
                }
            }
        }

        let decode_result = general_purpose::STANDARD
            .decode(b64_content)
            .map(|bytes| (bytes, "standard"))
            .or_else(|_| {
                general_purpose::URL_SAFE
                    .decode(b64_content)
                    .map(|bytes| (bytes, "url-safe"))
            })
            .or_else(|_| {
                general_purpose::URL_SAFE_NO_PAD
                    .decode(b64_content)
                    .map(|bytes| (bytes, "url-safe-no-pad"))
            });

        match decode_result {
            Ok((bytes, variant)) => {
                match String::from_utf8(bytes.clone()) {
                    Ok(s) => {
                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&s) {
                            self.output = serde_json::to_string_pretty(&json).unwrap_or(s);
                        } else {
                            self.output = s;
                        }
                        self.error = None;
                        self.error_hint = None;
                    }
                    Err(_) => {
                        self.output =
                            format!("Decoded {} binary bytes (Not valid UTF-8).", bytes.len());
                        self.error = None;
                        self.error_hint = None;
                    }
                }

                if let Ok(img) = image::load_from_memory(&bytes) {
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

                self.encoded_data_uri = None;

                // Log to history
                self.history_store.append(HistoryEntry::new(
                    HistoryOp::Decode,
                    b64,
                    &self.output,
                    variant,
                ));
            }
            Err(e) => {
                self.error = Some(format!("Invalid Base64: {}", e));
                self.error_hint = infer_hint(b64);
                self.image_preview = None;
                self.encoded_data_uri = None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn infer_hint_url_safe() {
        assert_eq!(infer_hint("abc-_xyz"), Some(DecodeHint::TryUrlSafe));
    }

    #[test]
    fn infer_hint_no_padding() {
        // length 5 — not a multiple of 4
        assert_eq!(infer_hint("abcde"), Some(DecodeHint::TryNoPadding));
    }

    #[test]
    fn infer_hint_whitespace() {
        assert_eq!(infer_hint("abcd efgh"), Some(DecodeHint::StripWhitespace));
    }

    #[test]
    fn infer_hint_clean() {
        assert_eq!(infer_hint("abcdefgh"), None);
    }
}
