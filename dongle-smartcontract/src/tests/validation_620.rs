//! Tests for issue #620 – Project Metadata Validation Gaps.
//!
//! Covers:
//! - `validate_website`: https-only enforcement and host-presence check.
//! - `is_valid_ipfs_cid`: character-set validation for CIDv0 (base58btc) and
//!   CIDv1 (base32 lowercase), plus length boundary correctness.
//! - Property-based tests that verify the boundaries hold for all inputs.

#![cfg(test)]

extern crate alloc;
use alloc::string::{String as StdString, ToString};

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

fn repeat_char(ch: char, n: usize) -> StdString {
    core::iter::repeat(ch).take(n).collect()
}

// ─── Known-good CIDs ─────────────────────────────────────────────────────────

/// A real CIDv0 (SHA-256 multihash, base58btc, exactly 46 chars).
const VALID_CIDV0: &str = "QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG";

/// A real CIDv1 (base32-lowercase, SHA-256, 59 chars).
const VALID_CIDV1: &str = "bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi";

// ═══════════════════════════════════════════════════════════════════════════
// Website URL – https-only enforcement
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn url_https_valid() {
    let e = mk_env();
    assert!(Utils::validate_website(&s(&e, "https://example.com")).is_ok());
}

#[test]
fn url_https_with_path_valid() {
    let e = mk_env();
    assert!(Utils::validate_website(&s(&e, "https://example.com/path?q=1")).is_ok());
}

#[test]
fn url_http_rejected() {
    // Plain http:// is no longer allowed — only https://.
    let e = mk_env();
    assert_eq!(
        Utils::validate_website(&s(&e, "http://example.com")),
        Err(ContractError::InvalidInput),
        "http:// must be rejected in favour of https://"
    );
}

#[test]
fn url_bare_https_scheme_only_rejected() {
    // "https://" with no host component must be rejected.
    let e = mk_env();
    assert_eq!(
        Utils::validate_website(&s(&e, "https://")),
        Err(ContractError::InvalidInput),
        "https:// with no host must be rejected"
    );
}

#[test]
fn url_ftp_rejected() {
    let e = mk_env();
    assert_eq!(
        Utils::validate_website(&s(&e, "ftp://example.com")),
        Err(ContractError::InvalidInput)
    );
}

#[test]
fn url_ws_rejected() {
    let e = mk_env();
    assert_eq!(
        Utils::validate_website(&s(&e, "ws://example.com")),
        Err(ContractError::InvalidInput)
    );
}

#[test]
fn url_ipfs_scheme_rejected() {
    let e = mk_env();
    assert_eq!(
        Utils::validate_website(&s(&e, "ipfs://bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi")),
        Err(ContractError::InvalidInput)
    );
}

#[test]
fn url_no_scheme_rejected() {
    let e = mk_env();
    for bad in ["example.com", "//example.com", "www.example.com"] {
        assert_eq!(
            Utils::validate_website(&s(&e, bad)),
            Err(ContractError::InvalidInput),
            "{bad:?} should be rejected"
        );
    }
}

#[test]
fn url_empty_rejected() {
    let e = mk_env();
    assert_eq!(
        Utils::validate_website(&s(&e, "")),
        Err(ContractError::InvalidInput)
    );
}

proptest! {
    /// Any https:// URL with a non-empty host (at least one char after the scheme)
    /// and total length ≤ MAX_WEBSITE_LEN must be accepted.
    #[test]
    fn prop_https_url_with_host_accepted(
        host in "[a-z]{1,30}\\.[a-z]{2,6}"
    ) {
        let e = mk_env();
        let url = alloc::format!("https://{host}");
        if url.len() <= crate::constants::MAX_WEBSITE_LEN {
            prop_assert!(
                Utils::validate_website(&s(&e, &url)).is_ok(),
                "https:// URL with host {host:?} should be valid"
            );
        }
    }

    /// Any http:// URL must always be rejected (regardless of host or path).
    #[test]
    fn prop_http_url_always_rejected(
        host in "[a-z]{1,20}\\.[a-z]{2,4}"
    ) {
        let e = mk_env();
        let url = alloc::format!("http://{host}");
        prop_assert_eq!(
            Utils::validate_website(&s(&e, &url)),
            Err(ContractError::InvalidInput),
            "http:// URL {:?} should always be rejected",
            url
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// CID – character-set validation (strict validator, #620)
// ═══════════════════════════════════════════════════════════════════════════

// ─── CIDv0 charset ───────────────────────────────────────────────────────────

#[test]
fn cidv0_real_cid_valid() {
    let e = mk_env();
    assert!(
        Utils::is_valid_ipfs_cid_strict(&s(&e, VALID_CIDV0)),
        "known-good CIDv0 must be valid"
    );
    assert_eq!(VALID_CIDV0.len(), 46);
}

#[test]
fn cidv0_must_be_exactly_46_chars() {
    let e = mk_env();
    // 45 chars with valid prefix and valid charset — must fail (too short).
    let short: StdString = "Qm".to_string() + &repeat_char('1', 43); // 2+43 = 45
    assert!(
        !Utils::is_valid_ipfs_cid_strict(&s(&e, &short)),
        "CIDv0 with 45 chars must be rejected"
    );
    // 47 chars with valid prefix — must fail (too long for CIDv0).
    let long: StdString = "Qm".to_string() + &repeat_char('1', 45); // 2+45 = 47
    assert!(
        !Utils::is_valid_ipfs_cid_strict(&s(&e, &long)),
        "CIDv0 with 47 chars must be rejected"
    );
}

#[test]
fn cidv0_invalid_base58_char_rejected() {
    let e = mk_env();
    // '0' (digit zero) is not in the base58btc alphabet.
    let bad: StdString = "Qm".to_string() + &repeat_char('1', 43) + "0"; // 46 chars, ends with '0'
    assert!(
        !Utils::is_valid_ipfs_cid_strict(&s(&e, &bad)),
        "CIDv0 containing '0' (not in base58btc) must be rejected"
    );
}

#[test]
fn cidv0_invalid_char_uppercase_o_rejected() {
    let e = mk_env();
    // 'O' (uppercase letter O) is not in the base58btc alphabet.
    let bad: StdString = "Qm".to_string() + &repeat_char('1', 43) + "O"; // 46 chars
    assert!(
        !Utils::is_valid_ipfs_cid_strict(&s(&e, &bad)),
        "CIDv0 containing 'O' (not in base58btc) must be rejected"
    );
}

#[test]
fn cidv0_invalid_char_uppercase_i_rejected() {
    let e = mk_env();
    // 'I' (uppercase letter I) is not in the base58btc alphabet.
    let bad: StdString = "Qm".to_string() + &repeat_char('1', 43) + "I"; // 46 chars
    assert!(
        !Utils::is_valid_ipfs_cid_strict(&s(&e, &bad)),
        "CIDv0 containing 'I' (not in base58btc) must be rejected"
    );
}

#[test]
fn cidv0_invalid_char_lowercase_l_rejected() {
    let e = mk_env();
    // 'l' (lowercase letter L) is not in the base58btc alphabet.
    let bad: StdString = "Qml".to_string() + &repeat_char('1', 43); // 46 chars, 'l' at position 2
    assert!(
        !Utils::is_valid_ipfs_cid_strict(&s(&e, &bad)),
        "CIDv0 containing 'l' (not in base58btc) must be rejected"
    );
}

#[test]
fn cidv0_all_valid_base58btc_chars_accepted() {
    let e = mk_env();
    // Build a 46-char string starting with 'Qm' using only valid base58btc chars.
    let body: StdString = repeat_char('1', 44); // '1' is valid base58btc
    let cid = alloc::format!("Qm{body}");
    assert_eq!(cid.len(), 46);
    assert!(
        Utils::is_valid_ipfs_cid_strict(&s(&e, &cid)),
        "CIDv0 with all valid base58btc chars must be accepted"
    );
}

// ─── CIDv1 charset ───────────────────────────────────────────────────────────

#[test]
fn cidv1_real_cid_valid() {
    let e = mk_env();
    assert!(
        Utils::is_valid_ipfs_cid_strict(&s(&e, VALID_CIDV1)),
        "known-good CIDv1 must be valid"
    );
}

#[test]
fn cidv1_invalid_char_uppercase_rejected() {
    let e = mk_env();
    // Base32 lowercase only allows a-z and 2-7. Uppercase 'A' is invalid.
    let bad: StdString = "bAFY".to_string() + &repeat_char('a', 55); // 'A' is not base32 lower
    assert!(
        !Utils::is_valid_ipfs_cid_strict(&s(&e, &bad)),
        "CIDv1 with uppercase char must be rejected"
    );
}

#[test]
fn cidv1_invalid_char_digit_0_rejected() {
    let e = mk_env();
    // '0' is not in the base32 lowercase alphabet (only 2-7 are valid digits).
    let bad: StdString = "b0".to_string() + &repeat_char('a', 57); // total 59 chars
    assert!(
        !Utils::is_valid_ipfs_cid_strict(&s(&e, &bad)),
        "CIDv1 with digit '0' (not base32) must be rejected"
    );
}

#[test]
fn cidv1_invalid_char_digit_1_rejected() {
    let e = mk_env();
    // '1' is not in the base32 lowercase alphabet.
    let bad: StdString = "b1".to_string() + &repeat_char('a', 57);
    assert!(
        !Utils::is_valid_ipfs_cid_strict(&s(&e, &bad)),
        "CIDv1 with digit '1' (not base32) must be rejected"
    );
}

#[test]
fn cidv1_invalid_char_digit_8_rejected() {
    let e = mk_env();
    // '8' is not in the base32 lowercase alphabet (only 2-7 allowed).
    let bad: StdString = "b8".to_string() + &repeat_char('a', 57);
    assert!(
        !Utils::is_valid_ipfs_cid_strict(&s(&e, &bad)),
        "CIDv1 with digit '8' (not base32) must be rejected"
    );
}

#[test]
fn cidv1_all_valid_base32_chars_accepted() {
    let e = mk_env();
    let body: StdString = repeat_char('a', 57); // all 'a' — valid base32
    let cid = alloc::format!("b{body}"); // total 58 chars, within [46, 128]
    assert!(
        Utils::is_valid_ipfs_cid_strict(&s(&e, &cid)),
        "CIDv1 with all-'a' body must be accepted"
    );
}

#[test]
fn cidv1_digits_2_through_7_accepted() {
    let e = mk_env();
    for digit in ['2', '3', '4', '5', '6', '7'] {
        let body: StdString = core::iter::repeat(digit).take(57).collect();
        let cid = alloc::format!("b{body}");
        assert!(
            Utils::is_valid_ipfs_cid_strict(&s(&e, &cid)),
            "CIDv1 body char '{digit}' should be a valid base32 digit"
        );
    }
}

// ─── CID length boundaries ────────────────────────────────────────────────────

#[test]
fn cid_below_min_len_rejected() {
    let e = mk_env();
    // MIN_CID_LEN = 46; 45-char CIDv1 must be rejected.
    let short = alloc::format!("b{}", repeat_char('a', 44)); // 45 chars
    assert!(
        !Utils::is_valid_ipfs_cid_strict(&s(&e, &short)),
        "CID below MIN_CID_LEN(46) must be rejected"
    );
}

#[test]
fn cid_at_min_len_cidv1_accepted() {
    let e = mk_env();
    // Exactly 46 chars, valid CIDv1 chars.
    let at_min = alloc::format!("b{}", repeat_char('a', 45)); // 46 chars
    assert!(
        Utils::is_valid_ipfs_cid_strict(&s(&e, &at_min)),
        "CIDv1 of exactly MIN_CID_LEN(46) must be accepted"
    );
}

#[test]
fn cid_at_max_len_128_accepted() {
    let e = mk_env();
    let at_max = alloc::format!("b{}", repeat_char('a', 127)); // 128 chars
    assert!(
        Utils::is_valid_ipfs_cid_strict(&s(&e, &at_max)),
        "CIDv1 of exactly MAX_CID_LEN(128) must be accepted"
    );
}

#[test]
fn cid_above_max_len_rejected() {
    let e = mk_env();
    let over_max = alloc::format!("b{}", repeat_char('a', 128)); // 129 chars
    assert!(
        !Utils::is_valid_ipfs_cid_strict(&s(&e, &over_max)),
        "CID of 129 chars must be rejected (exceeds MAX_CID_LEN)"
    );
}

// ─── Proptest: CID character-set boundaries ──────────────────────────────────

proptest! {
    /// CIDv1 strings with only base32-lowercase chars at valid lengths are accepted.
    #[test]
    fn prop_cidv1_valid_charset_and_length_accepted(
        len in 46usize..=128usize,
    ) {
        let e = mk_env();
        // Build body from 'a' repeated — always valid base32 lowercase.
        let body = repeat_char('a', len - 1);
        let cid = alloc::format!("b{body}");
        prop_assert!(Utils::is_valid_ipfs_cid_strict(&s(&e, &cid)),
            "CIDv1 len={len} with valid base32 chars must be accepted");
    }

    /// CIDv1 bodies containing digits 8 or 9 must always be rejected.
    #[test]
    fn prop_cidv1_invalid_digit_rejected(
        bad_digit in prop_oneof![Just('8'), Just('9')],
    ) {
        let e = mk_env();
        // 57-char body starting with the invalid digit.
        let body: StdString = core::iter::once(bad_digit)
            .chain(core::iter::repeat('a').take(56))
            .collect();
        let cid = alloc::format!("b{body}"); // 58 chars total
        prop_assert!(!Utils::is_valid_ipfs_cid_strict(&s(&e, &cid)),
            "CIDv1 with digit '{bad_digit}' in body must be rejected");
    }

    /// CIDv0 at exactly 46 chars with a valid base58btc body is accepted.
    #[test]
    fn prop_cidv0_valid_length_and_charset(
        // Use digits 1-9 as a safe valid-base58btc fill character.
        fill_char in prop_oneof![
            Just('1'), Just('2'), Just('3'), Just('4'), Just('5'),
            Just('6'), Just('7'), Just('8'), Just('9'),
        ],
    ) {
        let e = mk_env();
        let body: StdString = core::iter::repeat(fill_char).take(44).collect();
        let cid = alloc::format!("Qm{body}"); // exactly 46 chars
        prop_assert!(Utils::is_valid_ipfs_cid_strict(&s(&e, &cid)),
            "CIDv0 Qm+44×'{fill_char}' must be accepted");
    }

    /// CIDv0 bodies containing '0', 'O', 'I', or 'l' must always be rejected.
    #[test]
    fn prop_cidv0_excluded_chars_rejected(
        bad_char in prop_oneof![Just('0'), Just('O'), Just('I'), Just('l')],
    ) {
        let e = mk_env();
        // Insert the bad char at position 2 (after "Qm"), pad the rest with '1'.
        let rest: StdString = core::iter::once(bad_char)
            .chain(core::iter::repeat('1').take(43))
            .collect();
        let cid = alloc::format!("Qm{rest}"); // 46 chars total
        prop_assert!(!Utils::is_valid_ipfs_cid_strict(&s(&e, &cid)),
            "CIDv0 with excluded char '{bad_char}' must be rejected");
    }
}
