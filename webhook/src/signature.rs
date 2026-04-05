use hmac::{Hmac, Mac};
use sha2::Sha256;

use crate::Forge;

type HmacSha256 = Hmac<Sha256>;

/// Verify webhook HMAC-SHA256 signature.
///
/// GitHub sends `sha256=<hex>` in `x-hub-signature-256`.
/// Gitea sends raw `<hex>` in `x-gitea-signature`.
pub fn verify(signature: &str, secret: &str, body: &[u8], forge: Forge) -> bool {
    let hex_sig = match forge {
        Forge::GitHub => match signature.strip_prefix("sha256=") {
            Some(h) => h,
            None => return false,
        },
        Forge::Gitea => signature, // raw hex, no prefix
    };

    let sig_bytes = match hex::decode(hex_sig) {
        Ok(b) => b,
        Err(_) => return false,
    };

    let mut mac = match HmacSha256::new_from_slice(secret.as_bytes()) {
        Ok(m) => m,
        Err(_) => return false,
    };

    mac.update(body);
    mac.verify_slice(&sig_bytes).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_sig(secret: &str, body: &[u8]) -> String {
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(body);
        hex::encode(mac.finalize().into_bytes())
    }

    #[test]
    fn github_valid_signature() {
        let secret = "test-secret";
        let body = b"hello world";
        let sig = make_sig(secret, body);
        assert!(verify(&format!("sha256={sig}"), secret, body, Forge::GitHub));
    }

    #[test]
    fn github_invalid_signature() {
        assert!(!verify("sha256=deadbeef", "secret", b"body", Forge::GitHub));
    }

    #[test]
    fn github_missing_prefix() {
        assert!(!verify("deadbeef", "secret", b"body", Forge::GitHub));
    }

    #[test]
    fn gitea_valid_signature() {
        let secret = "test-secret";
        let body = b"hello world";
        let sig = make_sig(secret, body);
        assert!(verify(&sig, secret, body, Forge::Gitea));
    }

    #[test]
    fn gitea_invalid_signature() {
        assert!(!verify("deadbeef", "secret", b"body", Forge::Gitea));
    }
}
