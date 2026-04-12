use base64::{engine::general_purpose, Engine as _};

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

/// Structured decode output — no egui, no app state.
#[derive(Debug, Clone)]
pub enum DecodeOutput {
    Jwt { formatted: String },
    Text(String),
    Binary { bytes: Vec<u8>, summary: String },
}

#[derive(Debug, Clone)]
pub struct DecodeError {
    pub message: String,
    pub hint: Option<DecodeHint>,
}

/// Pure Base64 decode. Returns `(output, variant_name)` on success.
///
/// Handles: data URI prefix stripping, JWT detection, STANDARD / URL_SAFE /
/// URL_SAFE_NO_PAD fallback, JSON pretty-printing for text output.
pub fn decode_base64(input: &str) -> Result<(DecodeOutput, &'static str), DecodeError> {
    let clean = input.replace(|c: char| c.is_whitespace(), "");
    let b64_content = if let Some(idx) = clean.find("base64,") {
        &clean[idx + 7..]
    } else {
        clean.as_str()
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
                    formatted
                        .push_str(&serde_json::to_string_pretty(&p_json).unwrap_or(payload_str));
                } else {
                    formatted.push_str(&payload_str);
                }
                formatted.push_str(&format!("\n\nSignature:\n{}\n", parts[2]));

                return Ok((DecodeOutput::Jwt { formatted }, "jwt"));
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
            let output = match String::from_utf8(bytes.clone()) {
                Ok(s) => {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&s) {
                        DecodeOutput::Text(serde_json::to_string_pretty(&json).unwrap_or(s))
                    } else {
                        DecodeOutput::Text(s)
                    }
                }
                Err(_) => DecodeOutput::Binary {
                    summary: format!("Decoded {} binary bytes (Not valid UTF-8).", bytes.len()),
                    bytes,
                },
            };
            Ok((output, variant))
        }
        Err(e) => Err(DecodeError {
            message: format!("Invalid Base64: {}", e),
            hint: infer_hint(input),
        }),
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

    #[test]
    fn decode_valid_text() {
        let (output, variant) = decode_base64("SGVsbG8sIHdvcmxkIQ==").unwrap();
        assert_eq!(variant, "standard");
        match output {
            DecodeOutput::Text(s) => assert_eq!(s, "Hello, world!"),
            _ => panic!("expected Text"),
        }
    }

    #[test]
    fn decode_jwt() {
        let header =
            general_purpose::URL_SAFE_NO_PAD.encode(b"{\"alg\":\"HS256\",\"typ\":\"JWT\"}");
        let payload = general_purpose::URL_SAFE_NO_PAD
            .encode(b"{\"sub\":\"1234567890\",\"name\":\"John Doe\",\"iat\":1516239022}");
        let signature = "SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";
        let jwt = format!("{}.{}.{}", header, payload, signature);
        let (output, variant) = decode_base64(&jwt).unwrap();
        assert_eq!(variant, "jwt");
        match output {
            DecodeOutput::Jwt { formatted } => {
                assert!(formatted.contains("JWT Detected"));
                assert!(formatted.contains("John Doe"));
            }
            _ => panic!("expected Jwt"),
        }
    }

    #[test]
    fn decode_data_uri() {
        let (output, _) = decode_base64("data:text/plain;base64,SGVsbG8sIHdvcmxkIQ==").unwrap();
        match output {
            DecodeOutput::Text(s) => assert_eq!(s, "Hello, world!"),
            _ => panic!("expected Text"),
        }
    }

    #[test]
    fn decode_binary() {
        let (output, _) = decode_base64("////").unwrap();
        match output {
            DecodeOutput::Binary { summary, .. } => {
                assert!(summary.contains("binary bytes"));
            }
            _ => panic!("expected Binary"),
        }
    }

    #[test]
    fn decode_invalid() {
        let err = decode_base64("not_valid_b64!!_").unwrap_err();
        assert!(err.message.contains("Invalid Base64"));
    }

    #[test]
    fn decode_url_safe() {
        // Use bytes that produce `-` and `_` in URL-safe encoding (not valid STANDARD).
        let data: Vec<u8> = (0..=255).collect();
        let url_safe = general_purpose::URL_SAFE.encode(&data);
        assert!(url_safe.contains('-') || url_safe.contains('_'));
        let (output, variant) = decode_base64(&url_safe).unwrap();
        assert_eq!(variant, "url-safe");
        match output {
            DecodeOutput::Binary { bytes, .. } => assert_eq!(bytes, data),
            _ => panic!("expected Binary"),
        }
    }
}
