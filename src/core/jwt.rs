//! JWT inspection: structured parse, claim explanations, humanized timestamps,
//! warnings for common issues (alg:none, expired, not-yet-valid), and local
//! HMAC signature verification (HS256/384/512). Pure logic, no egui.

use base64::{engine::general_purpose, Engine as _};
use hmac::{Hmac, Mac};
use sha2::{Sha256, Sha384, Sha512};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JwtPart {
    Header,
    Payload,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JwtParseError {
    NotThreeParts,
    BadBase64(JwtPart),
    BadUtf8(JwtPart),
    BadJson(JwtPart),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JwtWarning {
    AlgNone,
    Expired { exp: i64, ago_secs: i64 },
    NotYetValid { nbf: i64, in_secs: i64 },
    IssuedInFuture { iat: i64, in_secs: i64 },
    MissingExp,
    MalformedTimestamp { claim: &'static str },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerificationResult {
    Ok,
    Mismatch,
    UnsupportedAlg(String),
    InvalidSignatureEncoding,
    EmptySecret,
}

#[derive(Debug, Clone)]
pub struct JwtHeader {
    pub alg: String,
    pub typ: Option<String>,
    pub kid: Option<String>,
    pub extra: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Clone)]
pub struct JwtInspection {
    pub raw: String,
    pub header: JwtHeader,
    pub header_raw_json: String,
    pub payload: serde_json::Map<String, serde_json::Value>,
    pub payload_raw_json: String,
    pub signature_b64: String,
    pub signing_input: String,
    pub warnings: Vec<JwtWarning>,
}

pub fn inspect(token: &str) -> Result<JwtInspection, JwtParseError> {
    inspect_at(token, SystemTime::now())
}

pub fn inspect_at(token: &str, now: SystemTime) -> Result<JwtInspection, JwtParseError> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return Err(JwtParseError::NotThreeParts);
    }

    let decode_part = |part: &str, which: JwtPart| -> Result<Vec<u8>, JwtParseError> {
        general_purpose::URL_SAFE_NO_PAD
            .decode(part)
            .or_else(|_| general_purpose::URL_SAFE.decode(part))
            .map_err(|_| JwtParseError::BadBase64(which))
    };

    let header_bytes = decode_part(parts[0], JwtPart::Header)?;
    let payload_bytes = decode_part(parts[1], JwtPart::Payload)?;

    let header_str =
        String::from_utf8(header_bytes).map_err(|_| JwtParseError::BadUtf8(JwtPart::Header))?;
    let payload_str =
        String::from_utf8(payload_bytes).map_err(|_| JwtParseError::BadUtf8(JwtPart::Payload))?;

    let header_value: serde_json::Value =
        serde_json::from_str(&header_str).map_err(|_| JwtParseError::BadJson(JwtPart::Header))?;
    let payload_value: serde_json::Value =
        serde_json::from_str(&payload_str).map_err(|_| JwtParseError::BadJson(JwtPart::Payload))?;

    let header_obj = header_value
        .as_object()
        .ok_or(JwtParseError::BadJson(JwtPart::Header))?
        .clone();
    let payload_obj = payload_value
        .as_object()
        .ok_or(JwtParseError::BadJson(JwtPart::Payload))?
        .clone();

    let header = build_header(&header_obj);
    let header_raw_json =
        serde_json::to_string_pretty(&serde_json::Value::Object(header_obj)).unwrap_or(header_str);
    let payload_raw_json =
        serde_json::to_string_pretty(&serde_json::Value::Object(payload_obj.clone()))
            .unwrap_or(payload_str);

    let warnings = compute_warnings(&header, &payload_obj, now);

    Ok(JwtInspection {
        raw: token.to_string(),
        header,
        header_raw_json,
        payload: payload_obj,
        payload_raw_json,
        signature_b64: parts[2].to_string(),
        signing_input: format!("{}.{}", parts[0], parts[1]),
        warnings,
    })
}

fn build_header(obj: &serde_json::Map<String, serde_json::Value>) -> JwtHeader {
    let alg = obj
        .get("alg")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let typ = obj
        .get("typ")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let kid = obj
        .get("kid")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let mut extra = obj.clone();
    extra.remove("alg");
    extra.remove("typ");
    extra.remove("kid");
    JwtHeader {
        alg,
        typ,
        kid,
        extra,
    }
}

fn compute_warnings(
    header: &JwtHeader,
    payload: &serde_json::Map<String, serde_json::Value>,
    now: SystemTime,
) -> Vec<JwtWarning> {
    let mut warnings = Vec::new();

    if header.alg.eq_ignore_ascii_case("none") {
        warnings.push(JwtWarning::AlgNone);
    }

    let now_secs = now
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    let claim_as_i64 =
        |name: &str| -> Option<Result<i64, ()>> { payload.get(name).map(|v| v.as_i64().ok_or(())) };

    match claim_as_i64("exp") {
        None => warnings.push(JwtWarning::MissingExp),
        Some(Err(())) => warnings.push(JwtWarning::MalformedTimestamp { claim: "exp" }),
        Some(Ok(exp)) => {
            if exp < now_secs {
                warnings.push(JwtWarning::Expired {
                    exp,
                    ago_secs: now_secs - exp,
                });
            }
        }
    }

    match claim_as_i64("nbf") {
        None => {}
        Some(Err(())) => warnings.push(JwtWarning::MalformedTimestamp { claim: "nbf" }),
        Some(Ok(nbf)) if nbf > now_secs => {
            warnings.push(JwtWarning::NotYetValid {
                nbf,
                in_secs: nbf - now_secs,
            });
        }
        Some(Ok(_)) => {}
    }

    match claim_as_i64("iat") {
        None => {}
        Some(Err(())) => warnings.push(JwtWarning::MalformedTimestamp { claim: "iat" }),
        Some(Ok(iat)) if iat > now_secs => {
            warnings.push(JwtWarning::IssuedInFuture {
                iat,
                in_secs: iat - now_secs,
            });
        }
        Some(Ok(_)) => {}
    }

    warnings
}

pub fn verify_hmac(inspection: &JwtInspection, secret: &[u8]) -> VerificationResult {
    if secret.is_empty() {
        return VerificationResult::EmptySecret;
    }
    let sig_bytes = match general_purpose::URL_SAFE_NO_PAD
        .decode(&inspection.signature_b64)
        .or_else(|_| general_purpose::URL_SAFE.decode(&inspection.signature_b64))
    {
        Ok(b) => b,
        Err(_) => return VerificationResult::InvalidSignatureEncoding,
    };
    let signing_input = inspection.signing_input.as_bytes();
    match inspection.header.alg.as_str() {
        "HS256" => {
            let Ok(mut mac) = <Hmac<Sha256> as Mac>::new_from_slice(secret) else {
                return VerificationResult::Mismatch;
            };
            mac.update(signing_input);
            if mac.verify_slice(&sig_bytes).is_ok() {
                VerificationResult::Ok
            } else {
                VerificationResult::Mismatch
            }
        }
        "HS384" => {
            let Ok(mut mac) = <Hmac<Sha384> as Mac>::new_from_slice(secret) else {
                return VerificationResult::Mismatch;
            };
            mac.update(signing_input);
            if mac.verify_slice(&sig_bytes).is_ok() {
                VerificationResult::Ok
            } else {
                VerificationResult::Mismatch
            }
        }
        "HS512" => {
            let Ok(mut mac) = <Hmac<Sha512> as Mac>::new_from_slice(secret) else {
                return VerificationResult::Mismatch;
            };
            mac.update(signing_input);
            if mac.verify_slice(&sig_bytes).is_ok() {
                VerificationResult::Ok
            } else {
                VerificationResult::Mismatch
            }
        }
        other => VerificationResult::UnsupportedAlg(other.to_string()),
    }
}

/// Format an epoch-seconds value as `YYYY-MM-DD HH:MM:SS UTC`.
/// Uses Howard Hinnant's civil-from-days algorithm — no `chrono` dep.
pub fn format_epoch_utc(epoch_secs: i64) -> String {
    let days = epoch_secs.div_euclid(86_400);
    let time_of_day = epoch_secs.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    let hour = time_of_day / 3600;
    let minute = (time_of_day % 3600) / 60;
    let second = time_of_day % 60;
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02} UTC",
        year, month, day, hour, minute, second
    )
}

/// Howard Hinnant, "chrono-Compatible Low-Level Date Algorithms",
/// http://howardhinnant.github.io/date_algorithms.html#civil_from_days.
/// Input: days since 1970-01-01 (can be negative). Output: (year, month, day).
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = if m <= 2 { y + 1 } else { y };
    (year, m, d)
}

pub fn format_relative(delta_secs: i64) -> String {
    let abs = delta_secs.unsigned_abs();
    let future = delta_secs > 0;

    fn unit(future: bool, n: u64, singular: &str, plural: &str) -> String {
        let noun = if n == 1 { singular } else { plural };
        if future {
            format!("in {n} {noun}")
        } else {
            format!("{n} {noun} ago")
        }
    }

    if abs < 60 {
        return "just now".to_string();
    }
    if abs < 3600 {
        return unit(future, abs / 60, "minute", "minutes");
    }
    if abs < 86_400 {
        return unit(future, abs / 3600, "hour", "hours");
    }
    if abs < 2_592_000 {
        return unit(future, abs / 86_400, "day", "days");
    }
    if abs < 31_536_000 {
        return unit(future, abs / 2_592_000, "month", "months");
    }
    unit(future, abs / 31_536_000, "year", "years")
}

pub fn explain_claim(name: &str) -> Option<&'static str> {
    match name {
        "iss" => Some("Issuer — principal that issued the JWT (RFC 7519 §4.1.1)"),
        "sub" => Some("Subject — principal that is the subject of the JWT (§4.1.2)"),
        "aud" => Some("Audience — recipients the JWT is intended for (§4.1.3)"),
        "exp" => Some("Expiration Time — after which the JWT MUST NOT be accepted (§4.1.4)"),
        "nbf" => Some("Not Before — before which the JWT MUST NOT be accepted (§4.1.5)"),
        "iat" => Some("Issued At — time at which the JWT was issued (§4.1.6)"),
        "jti" => Some("JWT ID — unique identifier for the JWT (§4.1.7)"),
        _ => None,
    }
}

impl JwtInspection {
    /// Text rendering used by the CLI. Byte-identical to the pre-refactor
    /// `formatted` string so `basie decode` output stays stable.
    pub fn to_display_string(&self) -> String {
        let mut out = String::from("JWT Detected:\n\nHeader:\n");
        out.push_str(&self.header_raw_json);
        out.push_str("\n\nPayload:\n");
        out.push_str(&self.payload_raw_json);
        out.push_str("\n\nSignature:\n");
        out.push_str(&self.signature_b64);
        out.push('\n');
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    /// Build a JWT compact token from a header JSON, payload JSON, and a raw
    /// signature string (already base64url-encoded — the caller decides).
    fn make_jwt(header_json: &str, payload_json: &str, sig: &str) -> String {
        let h = general_purpose::URL_SAFE_NO_PAD.encode(header_json.as_bytes());
        let p = general_purpose::URL_SAFE_NO_PAD.encode(payload_json.as_bytes());
        format!("{h}.{p}.{sig}")
    }

    fn at(secs: u64) -> SystemTime {
        UNIX_EPOCH + Duration::from_secs(secs)
    }

    // ---- inspect ----

    const JWT_IO_SAMPLE: &str = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";

    #[test]
    fn inspect_parses_standard_jwt() {
        let insp = inspect_at(JWT_IO_SAMPLE, at(1_700_000_000)).unwrap();
        assert_eq!(insp.header.alg, "HS256");
        assert_eq!(insp.header.typ.as_deref(), Some("JWT"));
        assert_eq!(
            insp.payload.get("name").and_then(|v| v.as_str()),
            Some("John Doe")
        );
        assert_eq!(
            insp.payload.get("sub").and_then(|v| v.as_str()),
            Some("1234567890")
        );
        assert_eq!(
            insp.signature_b64,
            "SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c"
        );
        assert!(insp
            .signing_input
            .starts_with("eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9."));
        assert!(
            insp.warnings.contains(&JwtWarning::MissingExp),
            "expected MissingExp, got {:?}",
            insp.warnings
        );
    }

    #[test]
    fn inspect_rejects_two_parts() {
        match inspect_at("a.b", at(1_700_000_000)) {
            Err(JwtParseError::NotThreeParts) => {}
            other => panic!("expected NotThreeParts, got {other:?}"),
        }
    }

    #[test]
    fn inspect_rejects_bad_base64_header() {
        // `@@@` is not valid base64url
        let token = "@@@.eyJhIjoxfQ.sig";
        match inspect_at(token, at(1_700_000_000)) {
            Err(JwtParseError::BadBase64(JwtPart::Header)) => {}
            other => panic!("expected BadBase64(Header), got {other:?}"),
        }
    }

    #[test]
    fn inspect_expired_token() {
        let token = make_jwt(
            r#"{"alg":"HS256","typ":"JWT"}"#,
            r#"{"exp":1000000000}"#,
            "sig",
        );
        let insp = inspect_at(&token, at(2_000_000_000)).unwrap();
        let has_expired = insp
            .warnings
            .iter()
            .any(|w| matches!(w, JwtWarning::Expired { .. }));
        assert!(has_expired, "got warnings: {:?}", insp.warnings);
    }

    #[test]
    fn inspect_not_yet_valid() {
        let token = make_jwt(
            r#"{"alg":"HS256"}"#,
            r#"{"nbf":2000000000,"exp":3000000000}"#,
            "sig",
        );
        let insp = inspect_at(&token, at(1_500_000_000)).unwrap();
        assert!(insp
            .warnings
            .iter()
            .any(|w| matches!(w, JwtWarning::NotYetValid { .. })));
    }

    #[test]
    fn inspect_iat_in_future() {
        let token = make_jwt(
            r#"{"alg":"HS256"}"#,
            r#"{"iat":2000000000,"exp":3000000000}"#,
            "sig",
        );
        let insp = inspect_at(&token, at(1_500_000_000)).unwrap();
        assert!(insp
            .warnings
            .iter()
            .any(|w| matches!(w, JwtWarning::IssuedInFuture { .. })));
    }

    #[test]
    fn inspect_missing_exp_warning() {
        let token = make_jwt(r#"{"alg":"HS256"}"#, r#"{"sub":"me"}"#, "sig");
        let insp = inspect_at(&token, at(1_700_000_000)).unwrap();
        assert!(insp.warnings.contains(&JwtWarning::MissingExp));
    }

    #[test]
    fn inspect_alg_none_warning() {
        let token = make_jwt(
            r#"{"alg":"none","typ":"JWT"}"#,
            r#"{"sub":"me","exp":9999999999}"#,
            "",
        );
        let insp = inspect_at(&token, at(1_700_000_000)).unwrap();
        assert!(insp.warnings.contains(&JwtWarning::AlgNone));
    }

    // ---- verify_hmac ----

    #[test]
    fn verify_hmac_hs256_success() {
        // Canonical jwt.io HS256 sample. Secret is "your-256-bit-secret".
        let insp = inspect_at(JWT_IO_SAMPLE, at(1_700_000_000)).unwrap();
        assert_eq!(
            verify_hmac(&insp, b"your-256-bit-secret"),
            VerificationResult::Ok
        );
    }

    #[test]
    fn verify_hmac_hs256_mismatch() {
        let insp = inspect_at(JWT_IO_SAMPLE, at(1_700_000_000)).unwrap();
        assert_eq!(
            verify_hmac(&insp, b"wrong-secret"),
            VerificationResult::Mismatch
        );
    }

    #[test]
    fn verify_hmac_hs384_roundtrip() {
        use hmac::{Hmac, Mac};
        use sha2::Sha384;
        let header_json = r#"{"alg":"HS384","typ":"JWT"}"#;
        let payload_json = r#"{"sub":"me"}"#;
        let h = general_purpose::URL_SAFE_NO_PAD.encode(header_json.as_bytes());
        let p = general_purpose::URL_SAFE_NO_PAD.encode(payload_json.as_bytes());
        let signing_input = format!("{h}.{p}");
        let secret = b"hs384-key";
        let mut mac = <Hmac<Sha384> as Mac>::new_from_slice(secret).unwrap();
        mac.update(signing_input.as_bytes());
        let sig_bytes = mac.finalize().into_bytes();
        let sig = general_purpose::URL_SAFE_NO_PAD.encode(sig_bytes);
        let token = format!("{signing_input}.{sig}");
        let insp = inspect_at(&token, at(1_700_000_000)).unwrap();
        assert_eq!(verify_hmac(&insp, secret), VerificationResult::Ok);
    }

    #[test]
    fn verify_hmac_hs512_roundtrip() {
        use hmac::{Hmac, Mac};
        use sha2::Sha512;
        let header_json = r#"{"alg":"HS512","typ":"JWT"}"#;
        let payload_json = r#"{"sub":"me"}"#;
        let h = general_purpose::URL_SAFE_NO_PAD.encode(header_json.as_bytes());
        let p = general_purpose::URL_SAFE_NO_PAD.encode(payload_json.as_bytes());
        let signing_input = format!("{h}.{p}");
        let secret = b"hs512-key";
        let mut mac = <Hmac<Sha512> as Mac>::new_from_slice(secret).unwrap();
        mac.update(signing_input.as_bytes());
        let sig_bytes = mac.finalize().into_bytes();
        let sig = general_purpose::URL_SAFE_NO_PAD.encode(sig_bytes);
        let token = format!("{signing_input}.{sig}");
        let insp = inspect_at(&token, at(1_700_000_000)).unwrap();
        assert_eq!(verify_hmac(&insp, secret), VerificationResult::Ok);
    }

    #[test]
    fn verify_hmac_rejects_rs256() {
        let token = make_jwt(r#"{"alg":"RS256"}"#, r#"{"sub":"me"}"#, "sig");
        let insp = inspect_at(&token, at(1_700_000_000)).unwrap();
        assert_eq!(
            verify_hmac(&insp, b"anything"),
            VerificationResult::UnsupportedAlg("RS256".to_string())
        );
    }

    #[test]
    fn verify_hmac_rejects_none() {
        let token = make_jwt(r#"{"alg":"none"}"#, r#"{"sub":"me"}"#, "");
        let insp = inspect_at(&token, at(1_700_000_000)).unwrap();
        assert_eq!(
            verify_hmac(&insp, b"anything"),
            VerificationResult::UnsupportedAlg("none".to_string())
        );
    }

    #[test]
    fn verify_hmac_empty_secret() {
        let insp = inspect_at(JWT_IO_SAMPLE, at(1_700_000_000)).unwrap();
        assert_eq!(verify_hmac(&insp, b""), VerificationResult::EmptySecret);
    }

    // ---- formatters ----

    #[test]
    fn format_relative_buckets() {
        assert_eq!(format_relative(0), "just now");
        assert_eq!(format_relative(30), "just now");
        assert_eq!(format_relative(-30), "just now");
        assert_eq!(format_relative(60), "in 1 minute");
        assert_eq!(format_relative(-60), "1 minute ago");
        assert_eq!(format_relative(3600), "in 1 hour");
        assert_eq!(format_relative(-3600), "1 hour ago");
        assert_eq!(format_relative(86400), "in 1 day");
        assert_eq!(format_relative(-86400), "1 day ago");
        assert_eq!(format_relative(-31_536_000), "1 year ago");
    }

    #[test]
    fn format_epoch_utc_known_value() {
        assert_eq!(format_epoch_utc(1_700_000_000), "2023-11-14 22:13:20 UTC");
        assert_eq!(format_epoch_utc(0), "1970-01-01 00:00:00 UTC");
    }

    // ---- explain_claim ----

    #[test]
    fn explain_claim_known_claims() {
        assert!(explain_claim("iss").is_some());
        assert!(explain_claim("sub").is_some());
        assert!(explain_claim("aud").is_some());
        assert!(explain_claim("exp").is_some());
        assert!(explain_claim("nbf").is_some());
        assert!(explain_claim("iat").is_some());
        assert!(explain_claim("jti").is_some());
    }

    #[test]
    fn explain_claim_unknown() {
        assert!(explain_claim("custom_field").is_none());
        assert!(explain_claim("").is_none());
    }

    // ---- to_display_string ----

    #[test]
    fn display_string_contains_parts() {
        let insp = inspect_at(JWT_IO_SAMPLE, at(1_700_000_000)).unwrap();
        let s = insp.to_display_string();
        assert!(s.contains("JWT Detected"));
        assert!(s.contains("John Doe"));
        assert!(s.contains("HS256"));
        assert!(s.contains("SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c"));
    }
}
