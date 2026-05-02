// Test module for optional field validation
// This file can be added to src/tests/ or integrated into the test suite

#[cfg(test)]
mod optional_field_validation_tests {
    use crate::utils::Utils;
    use soroban_sdk::{Env, String as SorobanString};

    // ─────────────────────────────────────────────────────────────
    // Tests for is_valid_url()
    // ─────────────────────────────────────────────────────────────

    #[test]
    fn test_valid_https_url() {
        let env = Env::default();
        let url = SorobanString::from_slice(&env, b"https://example.com");
        assert!(Utils::is_valid_url(&url));
    }

    #[test]
    fn test_valid_http_url() {
        let env = Env::default();
        let url = SorobanString::from_slice(&env, b"http://www.example.com");
        assert!(Utils::is_valid_url(&url));
    }

    #[test]
    fn test_url_with_path() {
        let env = Env::default();
        let url = SorobanString::from_slice(&env, b"https://example.com/path/to/resource");
        assert!(Utils::is_valid_url(&url));
    }

    #[test]
    fn test_url_without_protocol() {
        let env = Env::default();
        let url = SorobanString::from_slice(&env, b"example.com");
        assert!(!Utils::is_valid_url(&url));
    }

    #[test]
    fn test_url_with_invalid_protocol() {
        let env = Env::default();
        let url = SorobanString::from_slice(&env, b"ftp://example.com");
        assert!(!Utils::is_valid_url(&url));
    }

    #[test]
    fn test_empty_url() {
        let env = Env::default();
        let url = SorobanString::from_slice(&env, b"");
        assert!(!Utils::is_valid_url(&url));
    }

    #[test]
    fn test_url_only_protocol() {
        let env = Env::default();
        let url = SorobanString::from_slice(&env, b"https://");
        assert!(!Utils::is_valid_url(&url));
    }

    #[test]
    fn test_url_exceeds_max_length() {
        let env = Env::default();
        let long_url = format!("https://{}", "a".repeat(300));
        let url = SorobanString::from_slice(&env, long_url.as_bytes());
        assert!(!Utils::is_valid_url(&url));
    }

    #[test]
    fn test_url_at_max_boundary() {
        let env = Env::default();
        // Create a 256-character URL (exactly at MAX_WEBSITE_LEN)
        let url_str = "https://example.com/".to_string() + &"a".repeat(226);
        let url = SorobanString::from_slice(&env, url_str.as_bytes());
        assert!(Utils::is_valid_url(&url));
    }

    // ─────────────────────────────────────────────────────────────
    // Tests for is_valid_ipfs_cid()
    // ─────────────────────────────────────────────────────────────

    #[test]
    fn test_valid_cidv0_standard() {
        let env = Env::default();
        // Standard Qm pattern CIDv0
        let cid = SorobanString::from_slice(
            &env,
            b"QmXwGvnZ9zLo3vwU5r8G8Q8QvH1qvKrW9d8X8X8X8X8X8X8X8",
        );
        assert!(Utils::is_valid_ipfs_cid(&cid));
    }

    #[test]
    fn test_cidv0_wrong_second_char() {
        let env = Env::default();
        // CID with Q but not Qm pattern
        let cid = SorobanString::from_slice(
            &env,
            b"QnXwGvnZ9zLo3vwU5r8G8Q8QvH1qvKrW9d8X8X8X8X8X8X8X8",
        );
        assert!(!Utils::is_valid_ipfs_cid(&cid));
    }

    #[test]
    fn test_cid_too_short() {
        let env = Env::default();
        // Less than 46 characters
        let cid = SorobanString::from_slice(&env, b"QmShort123");
        assert!(!Utils::is_valid_ipfs_cid(&cid));
    }

    #[test]
    fn test_cid_too_long() {
        let env = Env::default();
        // More than 128 characters
        let long_cid = format!("Qm{}", "a".repeat(200));
        let cid = SorobanString::from_slice(&env, long_cid.as_bytes());
        assert!(!Utils::is_valid_ipfs_cid(&cid));
    }

    #[test]
    fn test_cidv0_at_min_boundary() {
        let env = Env::default();
        // Exactly 46 characters with Qm
        let cid = SorobanString::from_slice(&env, b"Qm123456789012345678901234567890123456789012");
        assert!(Utils::is_valid_ipfs_cid(&cid));
    }

    #[test]
    fn test_cidv0_at_max_boundary() {
        let env = Env::default();
        // Exactly 128 characters with Qm
        let long_cid = format!("Qm{}", "a".repeat(126));
        let cid = SorobanString::from_slice(&env, long_cid.as_bytes());
        assert!(Utils::is_valid_ipfs_cid(&cid));
    }

    #[test]
    fn test_cidv1_lowercase_alphanumeric() {
        let env = Env::default();
        // CIDv1 format (no Q, lowercase alphanumeric, 46+ chars)
        let cid = SorobanString::from_slice(&env, b"bagaaierahlcq5yduxj6ahjw5qhuhm47aaelzb3dqw67pyzwsaaaa");
        assert!(Utils::is_valid_ipfs_cid(&cid));
    }

    #[test]
    fn test_cidv1_with_uppercase_invalid() {
        let env = Env::default();
        // CIDv1 with uppercase (invalid)
        let cid = SorobanString::from_slice(&env, b"bagAAIERAHLCQ5YDUXJ6AHJW5QHUHM47AAELZB3DQW67PYZWSAAAA");
        assert!(!Utils::is_valid_ipfs_cid(&cid));
    }

    #[test]
    fn test_cid_with_special_chars_invalid() {
        let env = Env::default();
        // CID with special characters (invalid)
        let cid =
            SorobanString::from_slice(&env, b"Qm123456789@#$%^&*123456789012345678901234");
        assert!(!Utils::is_valid_ipfs_cid(&cid));
    }

    #[test]
    fn test_cidv1_base32_validity() {
        let env = Env::default();
        // CIDv1 base32 string starting with 'b' and valid characters
        let cid = SorobanString::from_slice(
            &env,
            b"bafybeigdyrztkowi2qrax2ajrlb67lsqz7s6q2hohwqg7y5sx3v7g3qu4e",
        );
        assert!(Utils::is_valid_ipfs_cid(&cid));
    }

    #[test]
    fn test_cidv1_base32_invalid_chars() {
        let env = Env::default();
        // CIDv1 base32 string with invalid uppercase characters
        let cid = SorobanString::from_slice(
            &env,
            b"bAFYBEIGDYRZTKOWI2QRAX2AJRLB67LSQZ7S6Q2HOHWQG7Y5SX3V7G3QU4E",
        );
        assert!(!Utils::is_valid_ipfs_cid(&cid));
    }

    // ─────────────────────────────────────────────────────────────
    // Tests for validate_optional_website()
    // ─────────────────────────────────────────────────────────────

    #[test]
    fn test_optional_website_none() {
        let result = Utils::validate_optional_website(&None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_optional_website_valid() {
        let env = Env::default();
        let url = SorobanString::from_slice(&env, b"https://example.com");
        let result = Utils::validate_optional_website(&Some(url));
        assert!(result.is_ok());
    }

    #[test]
    fn test_optional_website_invalid() {
        let env = Env::default();
        let url = SorobanString::from_slice(&env, b"invalid-url");
        let result = Utils::validate_optional_website(&Some(url));
        assert!(result.is_err());
    }

    // ─────────────────────────────────────────────────────────────
    // Tests for validate_optional_logo_cid()
    // ─────────────────────────────────────────────────────────────

    #[test]
    fn test_optional_logo_cid_none() {
        let result = Utils::validate_optional_logo_cid(&None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_optional_logo_cid_valid() {
        let env = Env::default();
        let cid = SorobanString::from_slice(
            &env,
            b"QmXwGvnZ9zLo3vwU5r8G8Q8QvH1qvKrW9d8X8X8X8X8X8X8X8",
        );
        let result = Utils::validate_optional_logo_cid(&Some(cid));
        assert!(result.is_ok());
    }

    #[test]
    fn test_optional_logo_cid_invalid() {
        let env = Env::default();
        let cid = SorobanString::from_slice(&env, b"invalid-cid");
        let result = Utils::validate_optional_logo_cid(&Some(cid));
        assert!(result.is_err());
    }

    // ─────────────────────────────────────────────────────────────
    // Tests for validate_optional_metadata_cid()
    // ─────────────────────────────────────────────────────────────

    #[test]
    fn test_optional_metadata_cid_none() {
        let result = Utils::validate_optional_metadata_cid(&None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_optional_metadata_cid_valid() {
        let env = Env::default();
        let cid = SorobanString::from_slice(
            &env,
            b"QmXwGvnZ9zLo3vwU5r8G8Q8QvH1qvKrW9d8X8X8X8X8X8X8X8",
        );
        let result = Utils::validate_optional_metadata_cid(&Some(cid));
        assert!(result.is_ok());
    }

    #[test]
    fn test_optional_metadata_cid_invalid() {
        let env = Env::default();
        let cid = SorobanString::from_slice(&env, b"too-short");
        let result = Utils::validate_optional_metadata_cid(&Some(cid));
        assert!(result.is_err());
    }

    // ─────────────────────────────────────────────────────────────
    // Tests for validate_optional_comment_cid()
    // ─────────────────────────────────────────────────────────────

    #[test]
    fn test_optional_comment_cid_none() {
        let result = Utils::validate_optional_comment_cid(&None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_optional_comment_cid_valid() {
        let env = Env::default();
        let cid = SorobanString::from_slice(
            &env,
            b"QmXwGvnZ9zLo3vwU5r8G8Q8QvH1qvKrW9d8X8X8X8X8X8X8X8",
        );
        let result = Utils::validate_optional_comment_cid(&Some(cid));
        assert!(result.is_ok());
    }

    #[test]
    fn test_optional_comment_cid_invalid() {
        let env = Env::default();
        let cid = SorobanString::from_slice(&env, b"Q");
        let result = Utils::validate_optional_comment_cid(&Some(cid));
        assert!(result.is_err());
    }

    // ─────────────────────────────────────────────────────────────
    // Integration Tests
    // ─────────────────────────────────────────────────────────────

    #[test]
    fn test_all_optional_fields_valid() {
        let env = Env::default();

        let website = SorobanString::from_slice(&env, b"https://example.com");
        let logo_cid = SorobanString::from_slice(
            &env,
            b"QmXwGvnZ9zLo3vwU5r8G8Q8QvH1qvKrW9d8X8X8X8X8X8X8X8",
        );
        let metadata_cid = SorobanString::from_slice(
            &env,
            b"QmYwGvnZ9zLo3vwU5r8G8Q8QvH1qvKrW9d8Y8Y8Y8Y8Y8Y8Y8",
        );

        assert!(Utils::validate_optional_website(&Some(website)).is_ok());
        assert!(Utils::validate_optional_logo_cid(&Some(logo_cid)).is_ok());
        assert!(Utils::validate_optional_metadata_cid(&Some(metadata_cid)).is_ok());
    }

    #[test]
    fn test_all_optional_fields_none() {
        assert!(Utils::validate_optional_website(&None).is_ok());
        assert!(Utils::validate_optional_logo_cid(&None).is_ok());
        assert!(Utils::validate_optional_metadata_cid(&None).is_ok());
        assert!(Utils::validate_optional_comment_cid(&None).is_ok());
    }

    #[test]
    fn test_mixed_valid_invalid_optional_fields() {
        let env = Env::default();

        let valid_website = SorobanString::from_slice(&env, b"https://example.com");
        let invalid_cid = SorobanString::from_slice(&env, b"invalid");

        assert!(Utils::validate_optional_website(&Some(valid_website)).is_ok());
        assert!(Utils::validate_optional_logo_cid(&Some(invalid_cid)).is_err());
    }
}
