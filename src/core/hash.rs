use base64::{engine::general_purpose, Engine as _};
use data_encoding::HEXLOWER;
use md5::{Digest, Md5};
use sha2::Sha256;

pub fn sha256_hex(data: &[u8]) -> String {
    let hash = Sha256::digest(data);
    HEXLOWER.encode(&hash)
}

pub fn md5_hex(data: &[u8]) -> String {
    let hash = Md5::digest(data);
    HEXLOWER.encode(&hash)
}

pub fn sha256_base64(data: &[u8]) -> String {
    let hash = Sha256::digest(data);
    general_purpose::STANDARD.encode(hash)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_hello() {
        let hash = sha256_hex(b"Hello");
        assert_eq!(
            hash,
            "185f8db32271fe25f561a6fc938b2e264306ec304eda518007d1764826381969"
        );
    }

    #[test]
    fn md5_hello() {
        let hash = md5_hex(b"Hello");
        assert_eq!(hash, "8b1a9953c4611296a827abf8c47804d7");
    }

    #[test]
    fn sha256_base64_not_empty() {
        let hash = sha256_base64(b"Hello");
        assert!(!hash.is_empty());
    }
}
