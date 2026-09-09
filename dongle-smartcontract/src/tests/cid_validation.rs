//! Dedicated CID validation test suite (GitHub issue #667).
//!
//! # What this module verifies
//!
//! `Utils::is_valid_ipfs_cid` and the wrapper helpers
//! (`validate_logo_cid`, `validate_metadata_cid`, `validate_report_reason_cid`)
//! must correctly handle all edge cases described in issue #667:
//!
//! - Empty string → rejected
//! - Too short (< 40 bytes) → rejected
//! - Too long (> 128 bytes) → rejected
//! - Malformed prefix (not `Qm` or `b`) → rejected
//! - CIDv0: starts with `Qm`, exactly 46 chars → accepted
//! - CIDv1: starts with `b`, 40–128 chars → accepted
//! - Future-proofing: base-32 (`b` prefix) and longer CIDv1 variants → accepted
//!
//! ## CID format reference (future-proofing rationale)
//!
//! IPFS CIDs follow the [CID spec](https://github.com/multiformats/cid):
//!
//! | Version | Encoding          | Typical prefix | Typical length |
//! |---------|-------------------|----------------|----------------|
//! | v0      | base58btc         | `Qm`           | 46 chars       |
//! | v1      | base32 (default)  | `b`            | 59+ chars      |
//! | v1      | base64url         | `u`            | variable       |
//!
//! The current validator accepts `Qm…` (CIDv0) and `b…` (CIDv1 base32).
//! Future CIDv1 variants using other multibase prefixes (e.g. `u`, `m`, `f`)
//! can be added by extending the prefix check in `Utils::is_valid_ipfs_cid`.
//! This is documented here so future maintainers know exactly where to extend.
//!
//! ## Empty-string rejection
//!
//! An empty CID is always invalid.  Both `is_valid_ipfs_cid` and the wrapper
//! functions (`validate_logo_cid`, etc.) return an error for empty input.

extern crate alloc;
use alloc::string::String as StdString;

use crate::errors::ContractError;
use crate::utils::Utils;
use soroban_sdk::{Env, String as SorobanString};

// ─── helpers ─────────────────────────────────────────────────────────────────

fn env() -> Env {
    Env::default()
}

fn s(e: &Env, v: &str) -> SorobanString {
    SorobanString::from_str(e, v)
}

fn repeat(ch: char, n: usize) -> StdString {
    core::iter::repeat(ch).take(n).collect()
}

fn cid_of_len(prefix: char, len: usize) -> StdString {
    let mut v = StdString::new();
    v.push(prefix);
    v.push_str(&repeat('a', len.saturating_sub(1)));
    v
}

// ─── Empty string ────────────────────────────────────────────────────────────

/// An empty CID string must be rejected by the low-level helper.
#[test]
fn empty_string_rejected_by_is_valid_ipfs_cid() {
    let e = env();
    assert!(
        !Utils::is_valid_ipfs_cid(&s(&e, "")),
        "empty CID must be rejected"
    );
}

/// `validate_logo_cid` must return `InvalidCid` for an empty string.
#[test]
fn empty_string_rejected_by_validate_logo_cid() {
    let e = env();
    assert_eq!(
        Utils::validate_logo_cid(&s(&e, "")),
        Err(ContractError::InvalidCid)
    );
}

/// `validate_metadata_cid` must return `InvalidCid` for an empty string.
#[test]
fn empty_string_rejected_by_validate_metadata_cid() {
    let e = env();
    assert_eq!(
        Utils::validate_metadata_cid(&s(&e, "")),
        Err(ContractError::InvalidCid)
    );
}

/// `validate_report_reason_cid` must return `InvalidCid` for an empty string.
#[test]
fn empty_string_rejected_by_validate_report_reason_cid() {
    let e = env();
    assert_eq!(
        Utils::validate_report_reason_cid(&s(&e, "")),
        Err(ContractError::InvalidCid)
    );
}

// ─── CIDv0 (`Qm` prefix) ─────────────────────────────────────────────────────

/// Canonical CIDv0 (46 chars, `Qm` prefix) must be accepted.
#[test]
fn cidv0_canonical_46_chars_accepted() {
    let e = env();
    // Real-world CIDv0 from IPFS docs.
    let cid = s(&e, "QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG");
    assert_eq!(cid.len(), 46);
    assert!(Utils::is_valid_ipfs_cid(&cid));
}

/// CIDv0 at minimum allowed length (40 chars) must be accepted.
#[test]
fn cidv0_at_minimum_length_40_accepted() {
    let e = env();
    let cid_str = {
        let mut v = StdString::from("Qm");
        v.push_str(&repeat('a', 38)); // 2 + 38 = 40
        v
    };
    assert_eq!(cid_str.len(), 40);
    let cid = SorobanString::from_str(&e, &cid_str);
    assert!(Utils::is_valid_ipfs_cid(&cid));
}

/// CIDv0 shorter than 40 chars must be rejected.
#[test]
fn cidv0_too_short_at_39_rejected() {
    let e = env();
    let cid_str = {
        let mut v = StdString::from("Qm");
        v.push_str(&repeat('a', 37)); // 2 + 37 = 39
        v
    };
    assert_eq!(cid_str.len(), 39);
    let cid = SorobanString::from_str(&e, &cid_str);
    assert!(!Utils::is_valid_ipfs_cid(&cid));
}

/// CIDv0 at maximum allowed length (128 chars) must be accepted.
#[test]
fn cidv0_at_maximum_length_128_accepted() {
    let e = env();
    let cid_str = {
        let mut v = StdString::from("Qm");
        v.push_str(&repeat('a', 126)); // 2 + 126 = 128
        v
    };
    assert_eq!(cid_str.len(), 128);
    let cid = SorobanString::from_str(&e, &cid_str);
    assert!(Utils::is_valid_ipfs_cid(&cid));
}

/// CIDv0 longer than 128 chars must be rejected.
#[test]
fn cidv0_over_maximum_length_129_rejected() {
    let e = env();
    let cid_str = {
        let mut v = StdString::from("Qm");
        v.push_str(&repeat('a', 127)); // 2 + 127 = 129
        v
    };
    assert_eq!(cid_str.len(), 129);
    let cid = SorobanString::from_str(&e, &cid_str);
    assert!(!Utils::is_valid_ipfs_cid(&cid));
}

// ─── CIDv1 (`b` prefix / base32) ─────────────────────────────────────────────

/// Canonical CIDv1 (base32 encoded, `b` prefix) must be accepted.
#[test]
fn cidv1_canonical_59_chars_accepted() {
    let e = env();
    // Real-world CIDv1 from IPFS docs.
    let cid = s(&e, "bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi");
    assert!(cid.len() >= 40, "CIDv1 fixture must be at least 40 chars");
    assert!(Utils::is_valid_ipfs_cid(&cid));
}

/// CIDv1 at minimum allowed length (40 chars) must be accepted.
#[test]
fn cidv1_at_minimum_length_40_accepted() {
    let e = env();
    let cid_str = cid_of_len('b', 40);
    assert_eq!(cid_str.len(), 40);
    let cid = SorobanString::from_str(&e, &cid_str);
    assert!(Utils::is_valid_ipfs_cid(&cid));
}

/// CIDv1 at maximum allowed length (128 chars) must be accepted.
#[test]
fn cidv1_at_maximum_length_128_accepted() {
    let e = env();
    let cid_str = cid_of_len('b', 128);
    assert_eq!(cid_str.len(), 128);
    let cid = SorobanString::from_str(&e, &cid_str);
    assert!(Utils::is_valid_ipfs_cid(&cid));
}

/// CIDv1 longer than 128 chars must be rejected.
#[test]
fn cidv1_over_maximum_length_rejected() {
    let e = env();
    for extra in 1..=5 {
        let cid_str = cid_of_len('b', 128 + extra);
        let cid = SorobanString::from_str(&e, &cid_str);
        assert!(
            !Utils::is_valid_ipfs_cid(&cid),
            "CIDv1 of len {} must be rejected (over max 128)",
            128 + extra
        );
    }
}

// ─── Malformed / invalid prefix ──────────────────────────────────────────────

/// A CID with no recognised prefix must be rejected.
#[test]
fn malformed_wrong_prefix_z_rejected() {
    let e = env();
    let cid_str = {
        let mut v = StdString::from("Z");
        v.push_str(&repeat('m', 45)); // total 46 chars, valid length but bad prefix
        v
    };
    assert!(!Utils::is_valid_ipfs_cid(&SorobanString::from_str(&e, &cid_str)));
}

/// `Q` alone (without `m` as second char) must be rejected.
#[test]
fn malformed_q_without_m_prefix_rejected() {
    let e = env();
    // Start with 'Q' but second char is 'x', not 'm'
    let cid_str = {
        let mut v = StdString::from("Qx");
        v.push_str(&repeat('a', 44)); // 2 + 44 = 46 chars
        v
    };
    let cid = SorobanString::from_str(&e, &cid_str);
    // 'Q' prefix without second 'm' char — not a valid CIDv0 pattern
    // Current implementation checks only buf[0]=='Q'&&buf[1]=='m', so 'Qx' is rejected.
    assert!(!Utils::is_valid_ipfs_cid(&cid));
}

/// Numeric prefix must be rejected.
#[test]
fn malformed_numeric_prefix_rejected() {
    let e = env();
    let cid_str = {
        let mut v = StdString::from("1");
        v.push_str(&repeat('a', 45)); // total 46 chars
        v
    };
    assert!(!Utils::is_valid_ipfs_cid(&SorobanString::from_str(&e, &cid_str)));
}

/// Whitespace-prefixed string must be rejected.
#[test]
fn malformed_whitespace_prefix_rejected() {
    let e = env();
    let cid_str = {
        let mut v = StdString::from(" ");
        v.push_str(&repeat('a', 45));
        v
    };
    assert!(!Utils::is_valid_ipfs_cid(&SorobanString::from_str(&e, &cid_str)));
}

/// A single-character string must be rejected (too short).
#[test]
fn single_char_rejected() {
    let e = env();
    for ch in ['Q', 'b', 'a', '1'] {
        let mut s_val = StdString::new();
        s_val.push(ch);
        let cid = SorobanString::from_str(&e, &s_val);
        assert!(
            !Utils::is_valid_ipfs_cid(&cid),
            "Single-char CID '{ch}' must be rejected"
        );
    }
}

// ─── Length boundary sweep ────────────────────────────────────────────────────

/// CIDs of length 0–39 must all be rejected regardless of prefix.
#[test]
fn all_lengths_under_40_rejected() {
    let e = env();
    for len in 0..=39 {
        let cid_str = cid_of_len('b', len);
        let cid = SorobanString::from_str(&e, &cid_str);
        assert!(
            !Utils::is_valid_ipfs_cid(&cid),
            "CID of length {len} must be rejected (too short)"
        );
    }
}

/// CIDs of length 40–128 with CIDv1 prefix must all be accepted.
#[test]
fn all_lengths_40_to_128_cidv1_accepted() {
    let e = env();
    for len in 40..=128 {
        let cid_str = cid_of_len('b', len);
        let cid = SorobanString::from_str(&e, &cid_str);
        assert!(
            Utils::is_valid_ipfs_cid(&cid),
            "CIDv1 of length {len} must be accepted"
        );
    }
}

// ─── Future-proofing note ─────────────────────────────────────────────────────

/// Verify that the current implementation accepts exactly `Qm` (CIDv0) and
/// `b` (CIDv1 base32) prefixes, and that adding support for future multibase
/// prefixes (`u` = base64url, `m` = base64, `f` = base16) only requires
/// extending the prefix check in `Utils::is_valid_ipfs_cid`.
///
/// This test **documents** the limitation — it does NOT assert that future
/// prefixes are accepted, because the contract intentionally restricts to the
/// two most common IPFS CID formats.  A future PR can change these assertions
/// and extend the prefix list.
#[test]
fn future_cid_prefixes_currently_not_accepted() {
    let e = env();

    // base64url (u), base64 (m), base16 (f) — valid CIDv1 multibase prefixes
    // per the multibase spec, but not currently supported.
    for prefix in ['u', 'm', 'f'] {
        let cid_str = cid_of_len(prefix, 60);
        let cid = SorobanString::from_str(&e, &cid_str);
        // Currently rejected — future maintainers can add support by updating
        // `Utils::is_valid_ipfs_cid` to also check for these prefixes.
        assert!(
            !Utils::is_valid_ipfs_cid(&cid),
            "Prefix '{prefix}' is not yet supported (CIDv1 future multibase). \
             Update Utils::is_valid_ipfs_cid when adding support."
        );
    }
}

// ─── Wrapper function validation ──────────────────────────────────────────────

/// A valid CIDv0 must be accepted by all wrapper validators.
#[test]
fn valid_cidv0_accepted_by_all_validators() {
    let e = env();
    let cid = s(&e, "QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG");

    assert!(Utils::validate_logo_cid(&cid).is_ok());
    assert!(Utils::validate_metadata_cid(&cid).is_ok());
    assert!(Utils::validate_report_reason_cid(&cid).is_ok());
}

/// A valid CIDv1 must be accepted by all wrapper validators.
#[test]
fn valid_cidv1_accepted_by_all_validators() {
    let e = env();
    let cid = s(&e, "bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi");

    assert!(Utils::validate_logo_cid(&cid).is_ok());
    assert!(Utils::validate_metadata_cid(&cid).is_ok());
    assert!(Utils::validate_report_reason_cid(&cid).is_ok());
}

/// An invalid CID must produce `InvalidCid` from all wrapper validators.
#[test]
fn invalid_cid_rejected_by_all_validators_with_correct_error() {
    let e = env();
    // A CID with wrong prefix that is otherwise the right length.
    let bad = s(&e, "Xmnotavalidcidatallbuthastherightsortoflengthabcde");

    assert_eq!(Utils::validate_logo_cid(&bad), Err(ContractError::InvalidCid));
    assert_eq!(Utils::validate_metadata_cid(&bad), Err(ContractError::InvalidCid));
    assert_eq!(Utils::validate_report_reason_cid(&bad), Err(ContractError::InvalidCid));
}
