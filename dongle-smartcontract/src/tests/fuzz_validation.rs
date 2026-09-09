//! Issue #545 — property-based fuzz tests for the CID and URL validators.
//!
//! The issue names `validate_bounty_url` / `validate_bounty_cid`; in the tree
//! today those map onto:
//!
//! * URL:  `Utils::validate_website` (used for `bounty_url`, `website`,
//!   `repository_url`).
//! * CID:  `Utils::is_valid_ipfs_cid` and its wrappers `validate_logo_cid`,
//!   `validate_metadata_cid`, `validate_report_reason_cid`,
//!   `ReviewValidation::validate_review_cid` and
//!   `VerificationValidation::validate_evidence_cid`.
//!
//! `string_validation.rs` already covers structured, regex-constrained cases.
//! This module complements it with *unconstrained* input: arbitrary strings
//! (empty, oversized, whitespace, control characters, multi-byte UTF-8) checked
//! against a byte-level oracle. For every input it asserts that:
//!
//! 1. the validator never panics — proptest fails the case on any panic, e.g. a
//!    slice-length mismatch in `String::copy_into_slice` or an out-of-bounds
//!    index into the fixed-size scratch buffer; and
//! 2. the accept/reject decision matches the documented byte-prefix and
//!    length rules.
//!
//! cargo-fuzz / afl.rs need a nightly toolchain and a separate fuzz target
//! crate; proptest is already a dev-dependency and gives equivalent coverage
//! for these small pure validators, with shrinking and a checked-in regression
//! corpus (`proptest-regressions/`).

extern crate alloc;
use alloc::string::String as StdString;

use crate::constants::{MAX_CID_LEN, MAX_WEBSITE_LEN, MIN_CID_LEN};
use crate::errors::ContractError;
use crate::review_registry::ReviewValidation;
use crate::utils::Utils;
use crate::verification_registry::VerificationValidation;
use proptest::prelude::*;
use soroban_sdk::{Env, String as SorobanString};

fn sstr(env: &Env, s: &str) -> SorobanString {
    SorobanString::from_str(env, s)
}

/// Run every CID validator; returning the accept flags is incidental — the
/// point is that calling all of them must not panic.
fn cid_accepts(env: &Env, cid: &SorobanString) -> [bool; 6] {
    [
        Utils::is_valid_ipfs_cid(cid),
        Utils::validate_logo_cid(cid).is_ok(),
        Utils::validate_metadata_cid(cid).is_ok(),
        Utils::validate_report_reason_cid(cid).is_ok(),
        ReviewValidation::validate_review_cid(cid).is_ok(),
        VerificationValidation::validate_evidence_cid(cid).is_ok(),
    ]
}

/// Byte-level oracle for `is_valid_ipfs_cid`.
fn cid_should_be_valid(raw: &str) -> bool {
    let bytes = raw.as_bytes();
    let len = bytes.len();
    let in_range = (40..=MAX_CID_LEN).contains(&len);
    let good_prefix = bytes.first().copied() == Some(b'b')
        || (bytes.first().copied() == Some(b'Q') && bytes.get(1).copied() == Some(b'm'));
    in_range && good_prefix
}

// ── plain unit checks the fuzz block relies on ──────────────────────────────

#[test]
fn every_cid_validator_rejects_empty() {
    let env = Env::default();
    assert_eq!(cid_accepts(&env, &sstr(&env, "")), [false; 6]);
}

#[test]
fn url_validator_rejects_empty() {
    let env = Env::default();
    assert_eq!(
        Utils::validate_website(&sstr(&env, "")),
        Err(ContractError::InvalidInput)
    );
}

proptest! {
    // ── URL validator ──────────────────────────────────────────────────────

    /// `validate_website` returns (never panics) for any input.
    #[test]
    fn url_validator_never_panics(raw in "(?s).{0,400}") {
        let env = Env::default();
        let _ = Utils::validate_website(&sstr(&env, &raw));
    }

    /// Empty, over-limit, or scheme-less input is always rejected with
    /// `InvalidInput`.
    #[test]
    fn url_validator_rejects_malformed(raw in "(?s).{0,400}") {
        let env = Env::default();
        let byte_len = raw.len();
        let has_scheme = raw.starts_with("http://") || raw.starts_with("https://");

        if byte_len == 0 || byte_len > MAX_WEBSITE_LEN || !has_scheme {
            prop_assert_eq!(
                Utils::validate_website(&sstr(&env, &raw)),
                Err(ContractError::InvalidInput)
            );
        }
    }

    /// Any `http(s)://` URL whose byte length is within the budget is accepted.
    #[test]
    fn url_validator_accepts_wellformed(
        scheme in "https?://",
        rest in "[a-zA-Z0-9./_?=&%-]{0,200}",
    ) {
        let env = Env::default();
        let mut raw = StdString::from(scheme.as_str());
        raw.push_str(&rest);
        prop_assume!(raw.len() <= MAX_WEBSITE_LEN);
        prop_assert!(Utils::validate_website(&sstr(&env, &raw)).is_ok());
    }

    // ── CID validators ─────────────────────────────────────────────────────

    /// No CID validator panics on any input.
    #[test]
    fn cid_validators_never_panic(raw in "(?s).{0,400}") {
        let env = Env::default();
        let _ = cid_accepts(&env, &sstr(&env, &raw));
    }

    /// `is_valid_ipfs_cid` agrees exactly with the byte-level oracle, and the
    /// non-length-restricted wrappers agree with `is_valid_ipfs_cid` on the
    /// same inputs.
    #[test]
    fn cid_core_matches_oracle(raw in "(?s).{0,400}") {
        let env = Env::default();
        let cid = sstr(&env, &raw);
        let expected = cid_should_be_valid(&raw);

        prop_assert_eq!(Utils::is_valid_ipfs_cid(&cid), expected, "input {:?}", raw);
        // logo / metadata / report-reason wrappers are `is_valid_ipfs_cid`
        // plus a redundant empty check, so they must track it exactly.
        prop_assert_eq!(Utils::validate_logo_cid(&cid).is_ok(), expected);
        prop_assert_eq!(Utils::validate_metadata_cid(&cid).is_ok(), expected);
        prop_assert_eq!(Utils::validate_report_reason_cid(&cid).is_ok(), expected);
    }

    /// Anything the oracle rejects is rejected by every CID validator, including
    /// the stricter review- and evidence-CID paths.
    #[test]
    fn cid_all_validators_reject_invalid(raw in "(?s).{0,400}") {
        let env = Env::default();
        prop_assume!(!cid_should_be_valid(&raw));
        let accepts = cid_accepts(&env, &sstr(&env, &raw));
        prop_assert_eq!(accepts, [false; 6], "input {:?} wrongly accepted", raw);
    }

    /// A canonical CIDv0 (`Qm` + 44 base58 chars = 46 bytes) is accepted by
    /// every validator, including the evidence path that also enforces
    /// `MIN_CID_LEN`.
    #[test]
    fn cid_validators_accept_canonical_v0(body in "[1-9A-HJ-NP-Za-km-z]{44}") {
        let env = Env::default();
        let mut raw = StdString::from("Qm");
        raw.push_str(&body);
        prop_assert_eq!(raw.len(), MIN_CID_LEN);
        prop_assert_eq!(cid_accepts(&env, &sstr(&env, &raw)), [true; 6]);
    }

    /// A `b`-prefixed CIDv1 within the length window passes the core check and
    /// the plain wrappers.
    #[test]
    fn cid_validators_accept_v1(body in "[a-z2-7]{45,120}") {
        let env = Env::default();
        let mut raw = StdString::from("b");
        raw.push_str(&body);
        prop_assume!(raw.len() <= MAX_CID_LEN);
        let cid = sstr(&env, &raw);
        prop_assert!(Utils::is_valid_ipfs_cid(&cid));
        prop_assert!(Utils::validate_logo_cid(&cid).is_ok());
    }

    /// Inputs whose byte length is just past `MAX_CID_LEN` are rejected without
    /// tripping the internal fixed-size buffer copy.
    #[test]
    fn cid_validators_reject_oversized(prefix in "Qm|b", extra in 1usize..96) {
        let env = Env::default();
        let mut raw = StdString::from(prefix.as_str());
        raw.push_str(&"a".repeat(MAX_CID_LEN + extra));
        let cid = sstr(&env, &raw);
        prop_assert!(!Utils::is_valid_ipfs_cid(&cid));
        prop_assert_eq!(cid_accepts(&env, &cid), [false; 6]);
    }

    /// Multi-byte UTF-8 whose byte length lands in the CID window but whose
    /// leading bytes are continuation/lead bytes (never `Q`/`b`) is rejected,
    /// and the internal copy of exactly `len` bytes does not panic.
    #[test]
    fn cid_rejects_multibyte_in_length_window(reps in 14usize..=42usize) {
        let env = Env::default();
        // '€' is 3 bytes; reps in [14,42] => 42..=126 bytes, inside 40..=128.
        let raw: StdString = "\u{20AC}".repeat(reps);
        prop_assume!((40..=MAX_CID_LEN).contains(&raw.len()));
        prop_assert!(!Utils::is_valid_ipfs_cid(&sstr(&env, &raw)));
    }
}
