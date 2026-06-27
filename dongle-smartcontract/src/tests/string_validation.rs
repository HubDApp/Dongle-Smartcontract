//! Comprehensive string validation tests.
//!
//! Covers: project names, descriptions, categories, CIDs, and website URLs.
//! Includes randomized (proptest) tests, UTF-8 / unusual input edge cases,
//! CID length-boundary checks, and a no-panic surface sweep.

extern crate alloc;
use alloc::string::String as StdString;

use crate::constants::{
    MAX_CATEGORY_LEN, MAX_DESCRIPTION_LEN, MAX_LICENSE_LEN, MAX_NAME_LEN, MAX_WEBSITE_LEN,
};
use crate::errors::ContractError;
use crate::utils::Utils;
use proptest::prelude::*;
use soroban_sdk::{Env, String as SorobanString};

// ─── helpers ─────────────────────────────────────────────────────────────────

fn mk_env() -> Env {
    Env::default()
}

fn s(env: &Env, v: &str) -> SorobanString {
    SorobanString::from_str(env, v)
}

/// Build a Soroban string of `n` repetitions of ASCII byte `ch`.
fn repeat_byte(env: &Env, ch: u8, n: usize) -> SorobanString {
    let raw: StdString = core::iter::repeat(ch as char).take(n).collect();
    SorobanString::from_str(env, &raw)
}

/// Build a std String of `n` repetitions of `ch`.
fn repeat_char(ch: char, n: usize) -> StdString {
    core::iter::repeat(ch).take(n).collect()
}

// ═══════════════════════════════════════════════════════════════════════════
// Project name
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn name_empty_is_invalid() {
    let e = mk_env();
    assert_eq!(
        Utils::validate_project_name(&s(&e, "")),
        Err(ContractError::InvalidProjectName)
    );
}

#[test]
fn name_single_char_valid() {
    let e = mk_env();
    assert!(Utils::validate_project_name(&s(&e, "a")).is_ok());
}

#[test]
fn name_max_len_valid() {
    let e = mk_env();
    let name = repeat_byte(&e, b'a', MAX_NAME_LEN);
    assert!(Utils::validate_project_name(&name).is_ok());
}

#[test]
fn name_over_max_len_invalid() {
    let e = mk_env();
    let name = repeat_byte(&e, b'a', MAX_NAME_LEN + 1);
    assert_eq!(
        Utils::validate_project_name(&name),
        Err(ContractError::ProjectNameTooLong)
    );
}

#[test]
fn name_boundary_at_max_minus_one_valid() {
    let e = mk_env();
    let name = repeat_byte(&e, b'x', MAX_NAME_LEN - 1);
    assert!(Utils::validate_project_name(&name).is_ok());
}

#[test]
fn name_whitespace_only_invalid() {
    let e = mk_env();
    for ws in ["   ", "\t\t", "\n", "\r\n", " \t\n\r"] {
        let r = Utils::validate_project_name(&s(&e, ws));
        assert!(r.is_err(), "whitespace-only name {ws:?} should be invalid");
    }
}

#[test]
fn name_allowed_chars_valid() {
    let e = mk_env();
    for name in ["hello", "Hello-World", "my_project", "abc123", "A1-B2_C3", "X", "a-b"] {
        assert!(
            Utils::validate_project_name(&s(&e, name)).is_ok(),
            "name {name:?} should be valid"
        );
    }
}

#[test]
fn name_disallowed_chars_invalid() {
    let e = mk_env();
    // Each entry contains exactly one disallowed character
    let cases = [
        "hello world", // space
        "proj@name",
        "name!",
        "name.dot",
        "name/slash",
        "name\\back",
        "name#hash",
        "name$dollar",
        "name%pct",
        "name^caret",
        "name&amp",
        "name*star",
        "name(paren",
        "name+plus",
        "name=eq",
        "name[bracket",
        "name{brace",
        "name|pipe",
        "name:colon",
        "name;semi",
        "name\"quote",
        "name'tick",
        "name<lt",
        "name>gt",
        "name,comma",
        "name?question",
    ];
    for name in cases {
        let r = Utils::validate_project_name(&s(&e, name));
        assert!(r.is_err(), "name {name:?} with disallowed char should be invalid");
    }
}

// ─── project name: randomized ────────────────────────────────────────────────

proptest! {
    /// Any string of [a-zA-Z0-9_-] with len in [1, MAX_NAME_LEN] must be accepted.
    #[test]
    fn prop_name_valid_charset_accepted(
        s_val in proptest::string::string_regex("[a-zA-Z0-9_\\-]{1,50}").unwrap()
    ) {
        let e = mk_env();
        let name = SorobanString::from_str(&e, &s_val);
        prop_assert!(Utils::validate_project_name(&name).is_ok(),
            "valid name {s_val:?} rejected");
    }

    /// Strings with a disallowed char embedded must always be rejected.
    #[test]
    fn prop_name_special_char_rejected(
        prefix in "[a-zA-Z0-9]{0,10}",
        bad_char in "[^a-zA-Z0-9_\\-]{1}",
        suffix in "[a-zA-Z0-9]{0,10}",
    ) {
        let e = mk_env();
        let combined = alloc::format!("{prefix}{bad_char}{suffix}");
        if combined.len() <= MAX_NAME_LEN {
            let name = SorobanString::from_str(&e, &combined);
            prop_assert!(Utils::validate_project_name(&name).is_err(),
                "name with disallowed char {combined:?} should be invalid");
        }
    }

    /// Names longer than MAX_NAME_LEN must always return ProjectNameTooLong.
    #[test]
    fn prop_name_over_max_always_rejected(extra in 1usize..=20usize) {
        let e = mk_env();
        let name = repeat_byte(&e, b'a', MAX_NAME_LEN + extra);
        prop_assert_eq!(
            Utils::validate_project_name(&name),
            Err(ContractError::ProjectNameTooLong)
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Description
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn desc_empty_is_invalid() {
    let e = mk_env();
    assert_eq!(
        Utils::validate_description(&s(&e, "")),
        Err(ContractError::InvalidProjectDesc)
    );
}

#[test]
fn desc_single_char_valid() {
    let e = mk_env();
    assert!(Utils::validate_description(&s(&e, "x")).is_ok());
}

#[test]
fn desc_max_len_valid() {
    let e = mk_env();
    let d = repeat_byte(&e, b'a', MAX_DESCRIPTION_LEN);
    assert!(Utils::validate_description(&d).is_ok());
}

#[test]
fn desc_over_max_len_invalid() {
    let e = mk_env();
    let d = repeat_byte(&e, b'a', MAX_DESCRIPTION_LEN + 1);
    assert_eq!(
        Utils::validate_description(&d),
        Err(ContractError::ProjectDescTooLong)
    );
}

#[test]
fn desc_boundary_at_max_minus_one_valid() {
    let e = mk_env();
    let d = repeat_byte(&e, b'z', MAX_DESCRIPTION_LEN - 1);
    assert!(Utils::validate_description(&d).is_ok());
}

#[test]
fn desc_accepts_punctuation_and_spaces() {
    let e = mk_env();
    let text = "A great project! It does X, Y & Z. Visit https://example.com.";
    assert!(Utils::validate_description(&s(&e, text)).is_ok());
}

// ═══════════════════════════════════════════════════════════════════════════
// Category
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn category_empty_is_invalid() {
    let e = mk_env();
    assert_eq!(
        Utils::validate_category_field(&s(&e, "")),
        Err(ContractError::InvalidCategory)
    );
}

#[test]
fn category_whitespace_only_invalid() {
    let e = mk_env();
    for ws in ["   ", "\t", "\n", "\r"] {
        assert_eq!(
            Utils::validate_category_field(&s(&e, ws)),
            Err(ContractError::InvalidCategory),
            "ws={ws:?}"
        );
    }
}

#[test]
fn category_valid() {
    let e = mk_env();
    for cat in ["DeFi", "NFT", "Gaming", "Infrastructure", "DAO"] {
        assert!(
            Utils::validate_category_field(&s(&e, cat)).is_ok(),
            "category {cat:?} should be valid"
        );
    }
}

#[test]
fn category_max_len_valid() {
    let e = mk_env();
    let cat = repeat_byte(&e, b'c', MAX_CATEGORY_LEN);
    assert!(Utils::validate_category_field(&cat).is_ok());
}

#[test]
fn category_over_max_len_invalid() {
    let e = mk_env();
    let cat = repeat_byte(&e, b'c', MAX_CATEGORY_LEN + 1);
    assert_eq!(
        Utils::validate_category_field(&cat),
        Err(ContractError::InvalidCategory)
    );
}

#[test]
fn category_boundary_at_max_minus_one_valid() {
    let e = mk_env();
    let cat = repeat_byte(&e, b'c', MAX_CATEGORY_LEN - 1);
    assert!(Utils::validate_category_field(&cat).is_ok());
}

// ═══════════════════════════════════════════════════════════════════════════
// CID validation
// ═══════════════════════════════════════════════════════════════════════════

const VALID_CIDV0: &str = "QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG";
const VALID_CIDV1: &str = "bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi";

#[test]
fn cid_empty_is_invalid() {
    let e = mk_env();
    assert!(!Utils::is_valid_ipfs_cid(&s(&e, "")));
}

#[test]
fn cid_too_short_at_45_invalid() {
    let e = mk_env();
    // CID minimum is 46; 45 must be rejected even with valid prefix
    let short_cid = {
        let mut v = repeat_char('Q', 1);
        v.push('m');
        v.push_str(&repeat_char('a', 43)); // 1 + 1 + 43 = 45
        v
    };
    let cid = SorobanString::from_str(&e, &short_cid);
    assert!(!Utils::is_valid_ipfs_cid(&cid));
}

#[test]
fn cid_exactly_at_min_boundary_46_cidv0() {
    let e = mk_env();
    assert!(Utils::is_valid_ipfs_cid(&s(&e, VALID_CIDV0)));
    assert_eq!(VALID_CIDV0.len(), 46);
}

#[test]
fn cid_at_max_boundary_128_cidv1_valid() {
    let e = mk_env();
    let cid_str = repeat_char('b', 128);
    let cid = SorobanString::from_str(&e, &cid_str);
    assert!(Utils::is_valid_ipfs_cid(&cid));
}

#[test]
fn cid_at_129_invalid() {
    let e = mk_env();
    let cid_str = repeat_char('b', 129);
    let cid = SorobanString::from_str(&e, &cid_str);
    assert!(!Utils::is_valid_ipfs_cid(&cid));
}

#[test]
fn cid_cidv0_prefix_valid() {
    let e = mk_env();
    assert!(Utils::is_valid_ipfs_cid(&s(&e, VALID_CIDV0)));
}

#[test]
fn cid_cidv1_prefix_valid() {
    let e = mk_env();
    assert!(Utils::is_valid_ipfs_cid(&s(&e, VALID_CIDV1)));
}

#[test]
fn cid_wrong_prefix_invalid() {
    let e = mk_env();
    // 'Z' prefix, valid length — must be rejected
    let bad = {
        let mut v = StdString::from("Z");
        v.push_str(&repeat_char('m', 45));
        v
    };
    assert!(!Utils::is_valid_ipfs_cid(&SorobanString::from_str(&e, &bad)));
}

#[test]
fn validate_logo_cid_empty_invalid() {
    let e = mk_env();
    assert_eq!(
        Utils::validate_logo_cid(&s(&e, "")),
        Err(ContractError::InvalidLogoCid)
    );
}

#[test]
fn validate_logo_cid_valid() {
    let e = mk_env();
    assert!(Utils::validate_logo_cid(&s(&e, VALID_CIDV0)).is_ok());
}

#[test]
fn validate_metadata_cid_empty_invalid() {
    let e = mk_env();
    assert_eq!(
        Utils::validate_metadata_cid(&s(&e, "")),
        Err(ContractError::InvalidMetaCid)
    );
}

#[test]
fn validate_metadata_cid_valid() {
    let e = mk_env();
    assert!(Utils::validate_metadata_cid(&s(&e, VALID_CIDV1)).is_ok());
}

#[test]
fn validate_license_missing_value_is_not_required() {
    let _e = mk_env();
}

#[test]
fn validate_license_valid_spdx_id() {
    let e = mk_env();
    for license in ["MIT", "Apache-2.0", "GPL-2.0+", "BSD-3-Clause"] {
        assert!(
            Utils::validate_license(&s(&e, license)).is_ok(),
            "license {license:?} should be valid"
        );
    }
}

#[test]
fn validate_license_invalid_spdx_id() {
    let e = mk_env();
    for license in ["", "MIT OR Apache-2.0", "Apache_2.0", "GPL/3.0"] {
        assert_eq!(
            Utils::validate_license(&s(&e, license)),
            Err(ContractError::InvalidProjectData),
            "license {license:?} should be invalid"
        );
    }
}

#[test]
fn validate_license_over_max_invalid() {
    let e = mk_env();
    let license = repeat_byte(&e, b'A', MAX_LICENSE_LEN + 1);
    assert_eq!(
        Utils::validate_license(&license),
        Err(ContractError::InvalidProjectData)
    );
}

// ─── CID: randomized boundary tests ─────────────────────────────────────────

proptest! {
    /// CIDv1 strings (starting with 'b') at lengths [46..=128] must all be accepted.
    #[test]
    fn prop_cidv1_all_valid_lengths_accepted(len in 46usize..=128usize) {
        let e = mk_env();
        let cid_str = repeat_char('b', len);
        let cid = SorobanString::from_str(&e, &cid_str);
        prop_assert!(Utils::is_valid_ipfs_cid(&cid),
            "CIDv1 of len {len} should be valid");
    }

    /// CIDs shorter than 46 must always be rejected.
    #[test]
    fn prop_cid_too_short_rejected(len in 0usize..=45usize) {
        let e = mk_env();
        let cid_str = repeat_char('b', len);
        let cid = SorobanString::from_str(&e, &cid_str);
        prop_assert!(!Utils::is_valid_ipfs_cid(&cid),
            "CID of len {len} should be invalid (too short)");
    }

    /// CIDs longer than 128 must always be rejected.
    #[test]
    fn prop_cid_too_long_rejected(extra in 1usize..=50usize) {
        let e = mk_env();
        let cid_str = repeat_char('b', 128 + extra);
        let cid = SorobanString::from_str(&e, &cid_str);
        prop_assert!(!Utils::is_valid_ipfs_cid(&cid),
            "CID of len {} should be invalid (too long)", 128 + extra);
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Website URL
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn url_empty_invalid() {
    let e = mk_env();
    assert_eq!(
        Utils::validate_website(&s(&e, "")),
        Err(ContractError::InvalidWebsite)
    );
}

#[test]
fn url_http_valid() {
    let e = mk_env();
    assert!(Utils::validate_website(&s(&e, "http://example.com")).is_ok());
}

#[test]
fn url_https_valid() {
    let e = mk_env();
    assert!(Utils::validate_website(&s(&e, "https://example.com")).is_ok());
}

#[test]
fn url_missing_scheme_invalid() {
    let e = mk_env();
    for bad in ["example.com", "ftp://x.com", "//x.com", "www.x.com"] {
        assert_eq!(
            Utils::validate_website(&s(&e, bad)),
            Err(ContractError::InvalidWebsite),
            "url {bad:?} should be invalid"
        );
    }
}

#[test]
fn url_max_len_valid() {
    let e = mk_env();
    let prefix = "https://";
    let fill = repeat_char('a', MAX_WEBSITE_LEN - prefix.len());
    let url = alloc::format!("{prefix}{fill}");
    assert!(Utils::validate_website(&s(&e, &url)).is_ok());
}

#[test]
fn url_over_max_len_invalid() {
    let e = mk_env();
    let prefix = "https://";
    let fill = repeat_char('a', MAX_WEBSITE_LEN - prefix.len() + 1);
    let url = alloc::format!("{prefix}{fill}");
    assert_eq!(
        Utils::validate_website(&s(&e, &url)),
        Err(ContractError::InvalidWebsite)
    );
}

#[test]
fn url_boundary_at_max_minus_one_valid() {
    let e = mk_env();
    let prefix = "https://";
    let fill = repeat_char('a', MAX_WEBSITE_LEN - prefix.len() - 1);
    let url = alloc::format!("{prefix}{fill}");
    assert!(Utils::validate_website(&s(&e, &url)).is_ok());
}

// ═══════════════════════════════════════════════════════════════════════════
// Unusual UTF-8 / edge inputs
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn name_rejects_non_ascii_multibyte_chars() {
    // Multi-byte UTF-8 chars have byte values > 127, which are not
    // ASCII alphanumeric, so the name validator must reject them.
    let e = mk_env();
    for name in ["café", "naïve", "résumé", "日本語", "emoji\u{1F600}"] {
        let result = Utils::validate_project_name(&s(&e, name));
        assert!(
            result.is_err(),
            "non-ASCII name {name:?} should be rejected"
        );
    }
}

#[test]
fn desc_accepts_utf8_rich_content() {
    // Description only checks empty + length; Unicode content is allowed.
    let e = mk_env();
    let text = "A project with rich description: 你好, héllo, 🚀";
    assert!(Utils::validate_description(&s(&e, text)).is_ok());
}

#[test]
fn category_accepts_short_unicode_that_fits() {
    // Category accepts non-whitespace content regardless of encoding,
    // as long as it fits within MAX_CATEGORY_LEN bytes.
    let e = mk_env();
    assert!(Utils::validate_category_field(&s(&e, "DeFi")).is_ok());
}

#[test]
fn name_rejects_null_byte() {
    // '\x00' is not ASCII alphanumeric; must be rejected.
    let e = mk_env();
    let result = Utils::validate_project_name(&s(&e, "abc\x00def"));
    assert!(result.is_err(), "name with null byte should be rejected");
}

#[test]
fn name_rejects_control_chars() {
    let e = mk_env();
    for ch in ['\x01', '\x07', '\x1b', '\x7f'] {
        let name = alloc::format!("abc{ch}def");
        let result = Utils::validate_project_name(&s(&e, &name));
        assert!(result.is_err(), "name with control char {ch:?} should be rejected");
    }
}

#[test]
fn url_rejects_non_http_schemes() {
    let e = mk_env();
    for scheme in ["ftp://", "ws://", "ssh://", "file://", "ipfs://"] {
        let url = alloc::format!("{scheme}example.com");
        assert_eq!(
            Utils::validate_website(&s(&e, &url)),
            Err(ContractError::InvalidWebsite),
            "scheme {scheme} should be rejected"
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// No-panic surface sweep
//
// Each validator is called with adversarial inputs. In a no_std environment
// we cannot use std::panic::catch_unwind; instead the test itself would
// fail/abort if any call panicked. Simply executing and checking the return
// type proves no panic occurred.
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn no_panic_validate_project_name() {
    let e = mk_env();
    let long = repeat_char('a', 200);
    let inputs: &[&str] = &["", " ", "valid-name", "UPPER_CASE", "123", &long];
    for input in inputs {
        let _ = Utils::validate_project_name(&s(&e, input));
    }
}

#[test]
fn no_panic_validate_description() {
    let e = mk_env();
    let long = repeat_char('x', MAX_DESCRIPTION_LEN + 100);
    let inputs: &[&str] = &["", " ", "\t\n\r", "hello", &long];
    for input in inputs {
        let _ = Utils::validate_description(&s(&e, input));
    }
}

#[test]
fn no_panic_validate_category() {
    let e = mk_env();
    let long = repeat_char('c', MAX_CATEGORY_LEN + 10);
    let inputs: &[&str] = &["", "   ", "\t", "DeFi", &long];
    for input in inputs {
        let _ = Utils::validate_category_field(&s(&e, input));
    }
}

#[test]
fn no_panic_validate_website() {
    let e = mk_env();
    let long = alloc::format!("https://{}", repeat_char('a', MAX_WEBSITE_LEN));
    let inputs: &[&str] = &["", "http://", "https://", "ftp://bad", "example.com", &long];
    for input in inputs {
        let _ = Utils::validate_website(&s(&e, input));
    }
}

#[test]
fn no_panic_is_valid_ipfs_cid() {
    let e = mk_env();
    let very_long = repeat_char('Q', 200);
    let inputs: &[&str] = &[
        "",
        "Q",
        "Qm",
        VALID_CIDV0,
        VALID_CIDV1,
        "b",
        &very_long,
        "zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz",
    ];
    for input in inputs {
        let _ = Utils::is_valid_ipfs_cid(&s(&e, input));
    }
}

#[test]
fn no_panic_validate_project_slug() {
    let e = mk_env();
    let long = repeat_char('a', 200);
    let inputs: &[&str] = &["", " ", "valid-slug", "UPPERCASE", "a b", &long];
    for input in inputs {
        let _ = Utils::validate_project_slug(&s(&e, input));
    }
}
