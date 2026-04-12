use base64::{engine::general_purpose, Engine as _};
use data_encoding::{BASE32, HEXLOWER, HEXLOWER_PERMISSIVE};
use percent_encoding::{percent_decode_str, utf8_percent_encode, NON_ALPHANUMERIC};
use std::fmt;

/// The supported encoding formats for conversion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Format {
    #[default]
    Base64,
    Hex,
    Base32,
    Base58,
    PercentEncoded,
}

impl Format {
    /// All selectable formats in display order.
    pub fn all() -> &'static [Format] {
        &[
            Format::Base64,
            Format::Hex,
            Format::Base32,
            Format::Base58,
            Format::PercentEncoded,
        ]
    }

    pub fn parse(label: &str) -> Option<Self> {
        match label.trim() {
            "Base64" => Some(Format::Base64),
            "Hex" => Some(Format::Hex),
            "Base32" => Some(Format::Base32),
            "Base58" => Some(Format::Base58),
            "Percent-Encoded" => Some(Format::PercentEncoded),
            _ => None,
        }
    }
}

impl fmt::Display for Format {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Format::Base64 => write!(f, "Base64"),
            Format::Hex => write!(f, "Hex"),
            Format::Base32 => write!(f, "Base32"),
            Format::Base58 => write!(f, "Base58"),
            Format::PercentEncoded => write!(f, "Percent-Encoded"),
        }
    }
}

/// Errors that can occur during a conversion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConvertError {
    DecodeError(String),
    Utf8Required,
}

impl fmt::Display for ConvertError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DecodeError(msg) => write!(f, "{}", msg),
            Self::Utf8Required => write!(
                f,
                "Percent-encoding requires text — the bytes are not valid UTF-8"
            ),
        }
    }
}

// ─── Decode-to-bytes ──────────────────────────────────────────────────────────

/// Decode a lowercase or uppercase hex string to raw bytes.
/// Whitespace in the input is stripped before decoding.
pub fn hex_to_bytes(s: &str) -> Result<Vec<u8>, ConvertError> {
    let stripped: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    HEXLOWER_PERMISSIVE
        .decode(stripped.as_bytes())
        .map_err(|e| ConvertError::DecodeError(format!("Invalid hex: {}", e)))
}

/// Decode a Base32-encoded string (RFC 4648, uppercase + padding) to raw bytes.
pub fn base32_to_bytes(s: &str) -> Result<Vec<u8>, ConvertError> {
    BASE32
        .decode(s.trim().as_bytes())
        .map_err(|e| ConvertError::DecodeError(format!("Invalid Base32: {}", e)))
}

/// Decode a Base58-encoded string to raw bytes.
pub fn base58_to_bytes(s: &str) -> Result<Vec<u8>, ConvertError> {
    bs58::decode(s.trim())
        .into_vec()
        .map_err(|e| ConvertError::DecodeError(format!("Invalid Base58: {}", e)))
}

/// Decode a percent-encoded string to raw bytes (infallible — invalid sequences are
/// passed through as-is, matching browser behaviour).
pub fn percent_to_bytes(s: &str) -> Result<Vec<u8>, ConvertError> {
    Ok(percent_decode_str(s).collect())
}

/// Decode a Base64 string to raw bytes. Tries STANDARD, URL_SAFE, and URL_SAFE_NO_PAD
/// in order (same strategy as the main decode path).
pub fn base64_to_bytes(s: &str) -> Result<Vec<u8>, ConvertError> {
    let trimmed = s.trim();
    general_purpose::STANDARD
        .decode(trimmed)
        .or_else(|_| general_purpose::URL_SAFE.decode(trimmed))
        .or_else(|_| general_purpose::URL_SAFE_NO_PAD.decode(trimmed))
        .map_err(|e| ConvertError::DecodeError(format!("Invalid Base64: {}", e)))
}

// ─── Bytes-to-encode ─────────────────────────────────────────────────────────

/// Encode raw bytes as lowercase hex.
pub fn bytes_to_hex(bytes: &[u8]) -> String {
    HEXLOWER.encode(bytes)
}

/// Encode raw bytes as Base32 (RFC 4648, uppercase with `=` padding).
pub fn bytes_to_base32(bytes: &[u8]) -> String {
    BASE32.encode(bytes)
}

/// Encode raw bytes as Base58.
pub fn bytes_to_base58(bytes: &[u8]) -> String {
    bs58::encode(bytes).into_string()
}

/// Encode raw bytes as percent-encoded text. Returns `Utf8Required` if the bytes
/// are not valid UTF-8 (percent-encoding is defined over UTF-8 text).
pub fn bytes_to_percent(bytes: &[u8]) -> Result<String, ConvertError> {
    let s = std::str::from_utf8(bytes).map_err(|_| ConvertError::Utf8Required)?;
    Ok(utf8_percent_encode(s, NON_ALPHANUMERIC).to_string())
}

/// Encode raw bytes as standard Base64.
pub fn bytes_to_base64(bytes: &[u8]) -> String {
    general_purpose::STANDARD.encode(bytes)
}

// ─── Router ──────────────────────────────────────────────────────────────────

/// Convert `input` from `from` format to `to` format. Returns the input string
/// unchanged when `from == to`.
pub fn convert(input: &str, from: Format, to: Format) -> Result<String, ConvertError> {
    if from == to {
        return Ok(input.to_string());
    }

    let bytes = match from {
        Format::Hex => hex_to_bytes(input),
        Format::Base32 => base32_to_bytes(input),
        Format::Base58 => base58_to_bytes(input),
        Format::PercentEncoded => percent_to_bytes(input),
        Format::Base64 => base64_to_bytes(input),
    }?;

    Ok(match to {
        Format::Base64 => bytes_to_base64(&bytes),
        Format::Hex => bytes_to_hex(&bytes),
        Format::Base32 => bytes_to_base32(&bytes),
        Format::Base58 => bytes_to_base58(&bytes),
        Format::PercentEncoded => bytes_to_percent(&bytes)?,
    })
}

pub fn parse_conversion_variant(variant: &str) -> Option<(Format, Format)> {
    let (from, to) = variant.split_once("→")?;
    Some((Format::parse(from)?, Format::parse(to)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    // ─── Round-trips ───────────────────────────────────────────────────────

    #[test]
    fn hex_roundtrip() {
        let original = b"Hello, World!";
        let encoded = bytes_to_hex(original);
        let decoded = hex_to_bytes(&encoded).expect("round-trip decode");
        assert_eq!(decoded, original);
    }

    #[test]
    fn base32_roundtrip() {
        let original = b"Hello";
        let encoded = bytes_to_base32(original);
        // "Hello" base32-encodes to "JBSWY3DP" (8 chars, no padding needed)
        assert_eq!(encoded, "JBSWY3DP");
        let decoded = base32_to_bytes(&encoded).expect("round-trip decode");
        assert_eq!(decoded, original);
    }

    #[test]
    fn base58_roundtrip() {
        let original = b"Hello, World!";
        let encoded = bytes_to_base58(original);
        let decoded = base58_to_bytes(&encoded).expect("round-trip decode");
        assert_eq!(decoded, original);
    }

    #[test]
    fn percent_roundtrip() {
        // Encode text bytes → percent string → decode → same bytes
        let text = "Hello World";
        let encoded = bytes_to_percent(text.as_bytes()).expect("encode");
        assert!(encoded.contains("%20"));
        let decoded = percent_to_bytes(&encoded).expect("decode");
        assert_eq!(decoded, text.as_bytes());
    }

    #[test]
    fn base64_roundtrip() {
        let original = b"Hello, World!";
        let encoded = bytes_to_base64(original);
        let decoded = base64_to_bytes(&encoded).expect("round-trip decode");
        assert_eq!(decoded, original);
    }

    // ─── Cross-format ──────────────────────────────────────────────────────

    #[test]
    fn hex_to_base64_known_value() {
        // hex("Hello") → base64 → "SGVsbG8="
        let result = convert("48656c6c6f", Format::Hex, Format::Base64).expect("convert");
        assert_eq!(result, "SGVsbG8=");
    }

    #[test]
    fn hex_to_base64_to_hex_is_identity() {
        let hex_input = "48656c6c6f";
        let b64 = convert(hex_input, Format::Hex, Format::Base64).expect("hex→b64");
        let back = convert(&b64, Format::Base64, Format::Hex).expect("b64→hex");
        assert_eq!(back, hex_input);
    }

    #[test]
    fn convert_same_format_is_identity() {
        let s = "SGVsbG8=";
        assert_eq!(convert(s, Format::Base64, Format::Base64).unwrap(), s);
        assert_eq!(convert(s, Format::Hex, Format::Hex).unwrap(), s);
    }

    // ─── Error cases ───────────────────────────────────────────────────────

    #[test]
    fn bytes_to_percent_rejects_non_utf8() {
        let non_utf8 = &[0xFF, 0xFE, 0x00];
        let err = bytes_to_percent(non_utf8).unwrap_err();
        assert_eq!(err, ConvertError::Utf8Required);
    }

    #[test]
    fn hex_to_bytes_rejects_odd_length() {
        assert!(hex_to_bytes("abc").is_err());
    }

    #[test]
    fn hex_to_bytes_rejects_non_hex() {
        assert!(hex_to_bytes("zzzz").is_err());
    }

    #[test]
    fn hex_strips_whitespace_before_decode() {
        // "48 65 6c 6c 6f" with spaces → "Hello"
        let result = hex_to_bytes("48 65 6c 6c 6f").expect("whitespace-tolerant decode");
        assert_eq!(result, b"Hello");
    }
}
