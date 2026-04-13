//! Image metadata inspection and lossless metadata stripping.
//!
//! `inspect` reads format / dimensions / EXIF from raw image bytes without
//! fully decoding the pixel data (fast for large payloads). `strip_metadata`
//! removes EXIF and text annotations for JPEG and PNG **losslessly** —
//! no re-encoding, pixel data untouched.
//!
//! Pure Rust. Zero egui imports. Shared by the GUI and the CLI.

use std::io::Cursor;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageKind {
    Png,
    Jpeg,
    Gif,
    WebP,
    Bmp,
    Tiff,
    Other,
}

impl ImageKind {
    pub fn from_mime(mime: &str) -> Self {
        match mime {
            "image/png" => Self::Png,
            "image/jpeg" => Self::Jpeg,
            "image/gif" => Self::Gif,
            "image/webp" => Self::WebP,
            "image/bmp" => Self::Bmp,
            "image/tiff" => Self::Tiff,
            _ => Self::Other,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Png => "PNG",
            Self::Jpeg => "JPEG",
            Self::Gif => "GIF",
            Self::WebP => "WebP",
            Self::Bmp => "BMP",
            Self::Tiff => "TIFF",
            Self::Other => "Image",
        }
    }

    pub fn extension(&self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Jpeg => "jpg",
            Self::Gif => "gif",
            Self::WebP => "webp",
            Self::Bmp => "bmp",
            Self::Tiff => "tiff",
            Self::Other => "bin",
        }
    }

    pub fn strip_supported(&self) -> bool {
        matches!(self, Self::Jpeg | Self::Png)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExifField {
    pub tag: String,
    pub value: String,
}

#[derive(Debug, Clone)]
pub struct ImageMeta {
    pub kind: ImageKind,
    pub mime: &'static str,
    pub width: u32,
    pub height: u32,
    pub size_bytes: usize,
    pub exif: Vec<ExifField>,
    pub strip_supported: bool,
    /// Whether the bytes contain anything `strip_metadata` would actually
    /// remove. Independent of `exif` — a PNG with only `tEXt` chunks or a
    /// JPEG with only XMP/IPTC will have an empty `exif` but still set this
    /// to `true`, so the Export dialog can offer to scrub them.
    pub has_strippable_metadata: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StripError {
    Unsupported,
    Malformed(&'static str),
}

impl std::fmt::Display for StripError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unsupported => {
                write!(f, "Lossless metadata strip not supported for this format")
            }
            Self::Malformed(s) => write!(f, "Malformed image: {}", s),
        }
    }
}

impl std::error::Error for StripError {}

/// Inspect raw bytes and return metadata if they look like an image.
///
/// Reads dimensions via a header-only parse (fast), detects MIME via `infer`,
/// and parses EXIF via `kamadak-exif` when present. Returns `None` for
/// non-image bytes or formats the `image` crate can't read.
pub fn inspect(bytes: &[u8]) -> Option<ImageMeta> {
    let inferred = infer::get(bytes)?;
    let mime = inferred.mime_type();
    if !mime.starts_with("image/") {
        return None;
    }

    let (width, height) = image::ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .ok()?
        .into_dimensions()
        .ok()?;

    let kind = ImageKind::from_mime(mime);
    let exif = read_exif_fields(bytes);
    let has_strippable_metadata = has_strippable_metadata(bytes, kind);

    Some(ImageMeta {
        kind,
        mime,
        width,
        height,
        size_bytes: bytes.len(),
        exif,
        strip_supported: kind.strip_supported(),
        has_strippable_metadata,
    })
}

/// Scan `bytes` for any segment/chunk that `strip_metadata` would remove.
/// Returns `false` for unsupported formats, malformed headers, or clean files.
pub fn has_strippable_metadata(bytes: &[u8], kind: ImageKind) -> bool {
    match kind {
        ImageKind::Jpeg => scan_jpeg_strippable(bytes),
        ImageKind::Png => scan_png_strippable(bytes),
        _ => false,
    }
}

fn read_exif_fields(bytes: &[u8]) -> Vec<ExifField> {
    let mut cursor = Cursor::new(bytes);
    let reader = exif::Reader::new();
    let exif = match reader.read_from_container(&mut cursor) {
        Ok(exif) => exif,
        Err(_) => return Vec::new(),
    };

    exif.fields()
        .filter(|f| matches!(f.ifd_num, exif::In::PRIMARY | exif::In::THUMBNAIL))
        .map(|f| {
            let mut value = f.display_value().with_unit(&exif).to_string();
            // Truncate obscenely long values (MakerNote, etc.) so the UI stays tidy.
            if value.len() > 200 {
                value.truncate(197);
                value.push_str("...");
            }
            ExifField {
                tag: f.tag.to_string(),
                value,
            }
        })
        .collect()
}

/// Losslessly strip metadata from `bytes`. For JPEG this drops EXIF (APP1)
/// and Photoshop/IPTC (APP13); for PNG it drops `tEXt` / `zTXt` / `iTXt` /
/// `eXIf` chunks. Pixel data is never touched.
pub fn strip_metadata(bytes: &[u8], kind: ImageKind) -> Result<Vec<u8>, StripError> {
    match kind {
        ImageKind::Jpeg => strip_jpeg(bytes),
        ImageKind::Png => strip_png(bytes),
        _ => Err(StripError::Unsupported),
    }
}

// ---------------------------------------------------------------------------
// JPEG
// ---------------------------------------------------------------------------

// JPEG layout: SOI (FFD8) + series of marker segments + entropy-coded image
// data + EOI (FFD9). Each segment starts with 0xFF followed by a marker byte.
// Most segments have a 2-byte big-endian length field immediately after the
// marker (length includes those two bytes). Standalone markers (no length):
// D0..=D7 (RSTn), D8 (SOI), D9 (EOI), 01 (TEM). After an SOS (DA) segment,
// entropy-coded data runs until the next non-RST marker.

fn strip_jpeg(bytes: &[u8]) -> Result<Vec<u8>, StripError> {
    if bytes.len() < 4 || bytes[0] != 0xFF || bytes[1] != 0xD8 {
        return Err(StripError::Malformed("missing JPEG SOI"));
    }

    let mut out = Vec::with_capacity(bytes.len());
    out.extend_from_slice(&bytes[0..2]); // SOI

    let mut i = 2usize;
    while i < bytes.len() {
        if bytes[i] != 0xFF {
            return Err(StripError::Malformed("expected JPEG marker"));
        }
        // Skip fill bytes (multiple 0xFF) before the actual marker byte.
        let mut marker_idx = i + 1;
        while marker_idx < bytes.len() && bytes[marker_idx] == 0xFF {
            marker_idx += 1;
        }
        if marker_idx >= bytes.len() {
            return Err(StripError::Malformed("truncated JPEG marker"));
        }
        let marker = bytes[marker_idx];

        // Standalone markers (no length+payload).
        match marker {
            0xD9 => {
                // EOI — copy and finish.
                out.extend_from_slice(&bytes[i..=marker_idx]);
                return Ok(out);
            }
            0xD0..=0xD7 | 0xD8 | 0x01 => {
                out.extend_from_slice(&bytes[i..=marker_idx]);
                i = marker_idx + 1;
                continue;
            }
            _ => {}
        }

        // Segment with a 2-byte length.
        if marker_idx + 2 >= bytes.len() {
            return Err(StripError::Malformed("truncated JPEG segment length"));
        }
        let seg_len = ((bytes[marker_idx + 1] as usize) << 8) | (bytes[marker_idx + 2] as usize);
        if seg_len < 2 {
            return Err(StripError::Malformed("invalid JPEG segment length"));
        }
        let seg_end = marker_idx + 1 + seg_len; // length field + payload
        if seg_end > bytes.len() {
            return Err(StripError::Malformed("JPEG segment overruns file"));
        }

        let payload_start = marker_idx + 3;
        let payload = &bytes[payload_start..seg_end];

        let drop = match marker {
            0xE1 => {
                // APP1: EXIF or XMP.
                payload.starts_with(b"Exif\0\0")
                    || payload.starts_with(b"http://ns.adobe.com/xap/1.0/\0")
            }
            0xED => true, // APP13: Photoshop / IPTC / Ducky.
            _ => false,
        };

        if !drop {
            out.extend_from_slice(&bytes[i..seg_end]);
        }
        i = seg_end;

        // After SOS, entropy-coded data follows verbatim until the next
        // non-stuffed, non-RST marker. Copy it as-is (never dropped).
        if marker == 0xDA {
            let entropy_start = i;
            while i < bytes.len() {
                if bytes[i] == 0xFF {
                    if i + 1 >= bytes.len() {
                        break;
                    }
                    let next = bytes[i + 1];
                    if next == 0x00 || (0xD0..=0xD7).contains(&next) {
                        // Byte-stuffing or restart marker — still entropy data.
                        i += 2;
                        continue;
                    }
                    break;
                }
                i += 1;
            }
            out.extend_from_slice(&bytes[entropy_start..i]);
        }
    }
    Ok(out)
}

/// Does this JPEG contain any segment `strip_jpeg` would drop? Walks markers
/// up to (but not into) the SOS entropy data — the strip function also never
/// touches post-SOS bytes, so this stays consistent with `strip_metadata`.
fn scan_jpeg_strippable(bytes: &[u8]) -> bool {
    if bytes.len() < 4 || bytes[0] != 0xFF || bytes[1] != 0xD8 {
        return false;
    }
    let mut i = 2usize;
    while i < bytes.len() {
        if bytes[i] != 0xFF {
            return false;
        }
        let mut marker_idx = i + 1;
        while marker_idx < bytes.len() && bytes[marker_idx] == 0xFF {
            marker_idx += 1;
        }
        if marker_idx >= bytes.len() {
            return false;
        }
        let marker = bytes[marker_idx];
        match marker {
            0xD9 => return false, // EOI
            0xD0..=0xD7 | 0xD8 | 0x01 => {
                i = marker_idx + 1;
                continue;
            }
            _ => {}
        }
        if marker_idx + 2 >= bytes.len() {
            return false;
        }
        let seg_len = ((bytes[marker_idx + 1] as usize) << 8) | (bytes[marker_idx + 2] as usize);
        if seg_len < 2 {
            return false;
        }
        let seg_end = marker_idx + 1 + seg_len;
        if seg_end > bytes.len() {
            return false;
        }
        let payload = &bytes[marker_idx + 3..seg_end];
        let strippable = match marker {
            0xE1 => {
                payload.starts_with(b"Exif\0\0")
                    || payload.starts_with(b"http://ns.adobe.com/xap/1.0/\0")
            }
            0xED => true,
            _ => false,
        };
        if strippable {
            return true;
        }
        i = seg_end;
        if marker == 0xDA {
            // Entropy data after SOS — strip function never walks past here.
            return false;
        }
    }
    false
}

// ---------------------------------------------------------------------------
// PNG
// ---------------------------------------------------------------------------

// PNG layout: 8-byte signature + series of chunks. Each chunk is
// length (4 BE) + type (4 ASCII) + data + CRC (4). Ancillary chunks we
// drop for metadata scrubbing: tEXt, zTXt, iTXt, eXIf.

const PNG_SIG: &[u8; 8] = &[137, 80, 78, 71, 13, 10, 26, 10];

/// Does this PNG contain any chunk `strip_png` would drop? Walks chunks
/// non-destructively — never allocates a rewritten copy.
fn scan_png_strippable(bytes: &[u8]) -> bool {
    if bytes.len() < 8 || &bytes[0..8] != PNG_SIG {
        return false;
    }
    let mut i = 8usize;
    while i + 8 <= bytes.len() {
        let len = u32::from_be_bytes([bytes[i], bytes[i + 1], bytes[i + 2], bytes[i + 3]]) as usize;
        let type_start = i + 4;
        let data_start = type_start + 4;
        let Some(chunk_end) = data_start.checked_add(len).and_then(|n| n.checked_add(4)) else {
            return false;
        };
        if chunk_end > bytes.len() {
            return false;
        }
        let chunk_type = &bytes[type_start..type_start + 4];
        if matches!(chunk_type, b"tEXt" | b"zTXt" | b"iTXt" | b"eXIf") {
            return true;
        }
        if chunk_type == b"IEND" {
            break;
        }
        i = chunk_end;
    }
    false
}

fn strip_png(bytes: &[u8]) -> Result<Vec<u8>, StripError> {
    if bytes.len() < 8 || &bytes[0..8] != PNG_SIG {
        return Err(StripError::Malformed("not a PNG"));
    }

    let mut out = Vec::with_capacity(bytes.len());
    out.extend_from_slice(PNG_SIG);

    let mut i = 8usize;
    while i + 8 <= bytes.len() {
        let len = u32::from_be_bytes([bytes[i], bytes[i + 1], bytes[i + 2], bytes[i + 3]]) as usize;
        let type_start = i + 4;
        let data_start = type_start + 4;
        // Guard against overflow in `data_start + len`.
        let chunk_end = data_start
            .checked_add(len)
            .and_then(|n| n.checked_add(4))
            .ok_or(StripError::Malformed("PNG chunk length overflow"))?;
        if chunk_end > bytes.len() {
            return Err(StripError::Malformed("PNG chunk overruns file"));
        }

        let chunk_type = &bytes[type_start..type_start + 4];
        let drop = matches!(chunk_type, b"tEXt" | b"zTXt" | b"iTXt" | b"eXIf");
        if !drop {
            out.extend_from_slice(&bytes[i..chunk_end]);
        }

        i = chunk_end;
        if chunk_type == b"IEND" {
            break;
        }
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageFormat, RgbaImage};

    fn crc32_ieee(data: &[u8]) -> u32 {
        const POLY: u32 = 0xedb88320;
        let mut crc: u32 = 0xffffffff;
        for &byte in data {
            crc ^= byte as u32;
            for _ in 0..8 {
                crc = if crc & 1 == 1 {
                    POLY ^ (crc >> 1)
                } else {
                    crc >> 1
                };
            }
        }
        crc ^ 0xffffffff
    }

    fn build_png_bytes(width: u32, height: u32) -> Vec<u8> {
        let img = RgbaImage::from_pixel(width, height, image::Rgba([200, 100, 50, 255]));
        let mut buf = Vec::new();
        img.write_to(&mut Cursor::new(&mut buf), ImageFormat::Png)
            .expect("png encode");
        buf
    }

    fn build_jpeg_bytes(width: u32, height: u32) -> Vec<u8> {
        let img = image::RgbImage::from_pixel(width, height, image::Rgb([200, 100, 50]));
        let mut buf = Vec::new();
        img.write_to(&mut Cursor::new(&mut buf), ImageFormat::Jpeg)
            .expect("jpeg encode");
        buf
    }

    /// Insert a chunk (with valid CRC) immediately before the IEND chunk.
    fn inject_png_chunk(png: &[u8], chunk_type: &[u8; 4], payload: &[u8]) -> Vec<u8> {
        // Locate IEND to insert before it.
        let mut iend_offset = None;
        let mut i = 8usize;
        while i + 8 <= png.len() {
            let len = u32::from_be_bytes([png[i], png[i + 1], png[i + 2], png[i + 3]]) as usize;
            if &png[i + 4..i + 8] == b"IEND" {
                iend_offset = Some(i);
                break;
            }
            i += 4 + 4 + len + 4;
        }
        let iend_offset = iend_offset.expect("IEND present in png");

        let mut chunk = Vec::with_capacity(12 + payload.len());
        chunk.extend_from_slice(&(payload.len() as u32).to_be_bytes());
        chunk.extend_from_slice(chunk_type);
        chunk.extend_from_slice(payload);
        let mut crc_input = Vec::with_capacity(4 + payload.len());
        crc_input.extend_from_slice(chunk_type);
        crc_input.extend_from_slice(payload);
        chunk.extend_from_slice(&crc32_ieee(&crc_input).to_be_bytes());

        let mut out = Vec::with_capacity(png.len() + chunk.len());
        out.extend_from_slice(&png[..iend_offset]);
        out.extend_from_slice(&chunk);
        out.extend_from_slice(&png[iend_offset..]);
        out
    }

    /// Insert a JPEG APP1 segment containing "Exif\0\0" + body right after SOI.
    fn inject_jpeg_app1_exif(jpeg: &[u8], body: &[u8]) -> Vec<u8> {
        assert!(jpeg.starts_with(&[0xFF, 0xD8]));
        let payload_len = 2 + 6 + body.len(); // length bytes + "Exif\0\0" + body
        assert!(payload_len <= u16::MAX as usize);
        let mut segment = Vec::with_capacity(4 + payload_len);
        segment.push(0xFF);
        segment.push(0xE1);
        segment.push(((payload_len >> 8) & 0xFF) as u8);
        segment.push((payload_len & 0xFF) as u8);
        segment.extend_from_slice(b"Exif\0\0");
        segment.extend_from_slice(body);

        let mut out = Vec::with_capacity(jpeg.len() + segment.len());
        out.extend_from_slice(&jpeg[..2]);
        out.extend_from_slice(&segment);
        out.extend_from_slice(&jpeg[2..]);
        out
    }

    #[test]
    fn inspect_returns_none_for_non_image() {
        assert!(inspect(b"hello world").is_none());
        assert!(inspect(&[]).is_none());
        assert!(inspect(&[0u8; 4]).is_none());
    }

    #[test]
    fn inspect_reads_png_dimensions() {
        let png = build_png_bytes(7, 11);
        let meta = inspect(&png).expect("inspect png");
        assert_eq!(meta.kind, ImageKind::Png);
        assert_eq!(meta.width, 7);
        assert_eq!(meta.height, 11);
        assert_eq!(meta.size_bytes, png.len());
        assert!(meta.strip_supported);
        assert!(meta.exif.is_empty());
        assert!(
            !meta.has_strippable_metadata,
            "a freshly-encoded clean PNG should have nothing to strip"
        );
    }

    #[test]
    fn inspect_flags_png_text_chunk_without_exif_as_strippable() {
        // Regression test for the Codex review finding: a PNG with only
        // tEXt chunks has `exif.is_empty() == true`, but strip_metadata
        // would still scrub it. The dialog must still offer to strip.
        let png = build_png_bytes(4, 4);
        let with_text = inject_png_chunk(&png, b"tEXt", b"Author\0leaked-username");

        let meta = inspect(&with_text).expect("inspect png+tEXt");
        assert!(meta.exif.is_empty(), "tEXt is not EXIF");
        assert!(
            meta.has_strippable_metadata,
            "tEXt chunk must be flagged as strippable metadata"
        );
        assert!(meta.strip_supported);
    }

    #[test]
    fn inspect_flags_jpeg_xmp_without_exif_as_strippable() {
        // JPEG APP1 XMP payload (not EXIF) must also be detected as
        // strippable, otherwise the dialog silently refuses to scrub it.
        let jpg = build_jpeg_bytes(4, 4);
        let xmp_body = b"http://ns.adobe.com/xap/1.0/\0<x:xmpmeta>leaked</x:xmpmeta>";

        // Reuse the inject helper but feed an XMP-style payload directly.
        // We hand-build the segment since inject_jpeg_app1_exif forces the
        // "Exif\0\0" prefix.
        assert!(jpg.starts_with(&[0xFF, 0xD8]));
        let payload_len = 2 + xmp_body.len();
        let mut with_xmp = Vec::with_capacity(jpg.len() + 4 + xmp_body.len());
        with_xmp.extend_from_slice(&jpg[..2]);
        with_xmp.push(0xFF);
        with_xmp.push(0xE1);
        with_xmp.push(((payload_len >> 8) & 0xFF) as u8);
        with_xmp.push((payload_len & 0xFF) as u8);
        with_xmp.extend_from_slice(xmp_body);
        with_xmp.extend_from_slice(&jpg[2..]);

        let meta = inspect(&with_xmp).expect("inspect jpeg+xmp");
        assert!(
            meta.exif.is_empty(),
            "kamadak-exif should not parse XMP as EXIF"
        );
        assert!(
            meta.has_strippable_metadata,
            "JPEG APP1 XMP must be flagged as strippable"
        );

        // And the actual strip pass removes it.
        let stripped = strip_metadata(&with_xmp, ImageKind::Jpeg).expect("strip");
        assert!(!stripped.windows(xmp_body.len()).any(|w| w == xmp_body));
    }

    #[test]
    fn inspect_reads_jpeg_dimensions() {
        let jpg = build_jpeg_bytes(8, 12);
        let meta = inspect(&jpg).expect("inspect jpeg");
        assert_eq!(meta.kind, ImageKind::Jpeg);
        assert_eq!(meta.width, 8);
        assert_eq!(meta.height, 12);
        assert!(meta.strip_supported);
    }

    #[test]
    fn strip_clean_png_preserves_pixels() {
        let png = build_png_bytes(4, 3);
        let stripped = strip_metadata(&png, ImageKind::Png).expect("strip png");
        // Clean PNG has nothing to drop; bytes should round-trip to the same image.
        let original = image::load_from_memory(&png).unwrap().to_rgba8();
        let after = image::load_from_memory(&stripped).unwrap().to_rgba8();
        assert_eq!(original, after);
    }

    #[test]
    fn strip_removes_png_text_chunks_losslessly() {
        let png = build_png_bytes(5, 5);
        let with_text = inject_png_chunk(&png, b"tEXt", b"Comment\0sensitive note");
        let with_text_and_itxt =
            inject_png_chunk(&with_text, b"iTXt", b"Author\0\0\0\0\0leaked-username");

        // Sanity: injection actually inflated the file.
        assert!(with_text_and_itxt.len() > png.len());

        let stripped = strip_metadata(&with_text_and_itxt, ImageKind::Png).expect("strip png");

        // tEXt/iTXt bytes must be gone.
        assert!(
            !stripped.windows(4).any(|w| w == b"tEXt"),
            "tEXt chunk still present"
        );
        assert!(
            !stripped.windows(4).any(|w| w == b"iTXt"),
            "iTXt chunk still present"
        );
        assert!(!stripped
            .windows(b"sensitive note".len())
            .any(|w| w == b"sensitive note"));

        // Pixels must be identical.
        let original = image::load_from_memory(&png).unwrap().to_rgba8();
        let after = image::load_from_memory(&stripped).unwrap().to_rgba8();
        assert_eq!(original, after);
    }

    #[test]
    fn strip_removes_jpeg_exif_losslessly() {
        let jpg = build_jpeg_bytes(6, 6);
        let body = b"fake-exif-body-with-gps-0x01\x02\x03\x04";
        let with_exif = inject_jpeg_app1_exif(&jpg, body);

        // Sanity: Exif\0\0 and our body are present.
        assert!(with_exif.windows(6).any(|w| w == b"Exif\0\0"));
        assert!(with_exif.windows(body.len()).any(|w| w == body));

        let stripped = strip_metadata(&with_exif, ImageKind::Jpeg).expect("strip jpeg");

        assert!(
            !stripped.windows(6).any(|w| w == b"Exif\0\0"),
            "Exif marker still present"
        );
        assert!(
            !stripped.windows(body.len()).any(|w| w == body),
            "Exif body still present"
        );

        // Pixels must be identical (re-decoded comparison — JPEG is lossy
        // in general, but we only removed a segment, didn't re-encode).
        let original = image::load_from_memory(&jpg).unwrap().to_rgba8();
        let after = image::load_from_memory(&stripped).unwrap().to_rgba8();
        assert_eq!(original, after);
    }

    #[test]
    fn strip_rejects_unsupported_kinds() {
        let png = build_png_bytes(2, 2);
        let err = strip_metadata(&png, ImageKind::WebP).unwrap_err();
        assert_eq!(err, StripError::Unsupported);
    }

    #[test]
    fn strip_malformed_png_returns_error() {
        let err = strip_metadata(b"not a png", ImageKind::Png).unwrap_err();
        assert!(matches!(err, StripError::Malformed(_)));
    }

    #[test]
    fn strip_malformed_jpeg_returns_error() {
        let err = strip_metadata(b"not a jpeg", ImageKind::Jpeg).unwrap_err();
        assert!(matches!(err, StripError::Malformed(_)));
    }

    #[test]
    fn inspect_truncated_bytes_returns_none_without_panic() {
        let png = build_png_bytes(4, 4);
        assert!(inspect(&png[..10]).is_none());
    }
}
