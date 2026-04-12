use base64::{engine::general_purpose, Engine as _};

pub fn encode_base64(input: &str) -> String {
    general_purpose::STANDARD.encode(input)
}

pub fn encode_base64_bytes(input: &[u8]) -> String {
    general_purpose::STANDARD.encode(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_text() {
        assert_eq!(encode_base64("Hello, world!"), "SGVsbG8sIHdvcmxkIQ==");
    }

    #[test]
    fn encode_empty() {
        assert_eq!(encode_base64(""), "");
    }

    #[test]
    fn encode_bytes_matches_str() {
        let input = "Hello";
        assert_eq!(encode_base64(input), encode_base64_bytes(input.as_bytes()));
    }
}
