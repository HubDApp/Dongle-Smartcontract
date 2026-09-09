//! Comprehensive state-machine tests for [`ClaimStatus`] (issue #668).
//!
//! These tests verify:
//! - Terminal state identification (`is_terminal`)
//! - Valid transition detection (`can_transition_to`)
//! - Invalid transitions are rejected by the transition methods
//! - Double-transition and cross-transition error behaviour

use crate::errors::ContractError;
use crate::types::{ClaimKind, ClaimStatus};

// ── is_terminal ──────────────────────────────────────────────────────────────

#[test]
fn test_pending_is_not_terminal() {
    assert!(!ClaimStatus::Pending.is_terminal());
}

#[test]
fn test_approved_is_terminal() {
    assert!(ClaimStatus::Approved.is_terminal());
}

#[test]
fn test_rejected_is_terminal() {
    assert!(ClaimStatus::Rejected.is_terminal());
}

// ── can_transition_to — valid paths ──────────────────────────────────────────

#[test]
fn test_valid_transition_pending_to_approved() {
    assert!(ClaimStatus::Pending.can_transition_to(ClaimStatus::Approved));
}

#[test]
fn test_valid_transition_pending_to_rejected() {
    assert!(ClaimStatus::Pending.can_transition_to(ClaimStatus::Rejected));
}

// ── can_transition_to — invalid paths ────────────────────────────────────────

#[test]
fn test_invalid_transition_approved_to_pending() {
    assert!(!ClaimStatus::Approved.can_transition_to(ClaimStatus::Pending));
}

#[test]
fn test_invalid_transition_rejected_to_pending() {
    assert!(!ClaimStatus::Rejected.can_transition_to(ClaimStatus::Pending));
}

#[test]
fn test_invalid_transition_approved_to_rejected() {
    assert!(!ClaimStatus::Approved.can_transition_to(ClaimStatus::Rejected));
}

#[test]
fn test_invalid_transition_rejected_to_approved() {
    assert!(!ClaimStatus::Rejected.can_transition_to(ClaimStatus::Approved));
}

#[test]
fn test_invalid_transition_pending_to_pending() {
    assert!(!ClaimStatus::Pending.can_transition_to(ClaimStatus::Pending));
}

// ── transition methods — error behaviour ─────────────────────────────────────

#[test]
fn test_double_approve_returns_invalid_status_error() {
    let mut status = ClaimStatus::Approved;
    let err = status.transition_to_approved().unwrap_err();
    assert_eq!(err, ContractError::InvalidStatus);
    // Status must remain unchanged
    assert_eq!(status, ClaimStatus::Approved);
}

#[test]
fn test_double_reject_returns_invalid_status_error() {
    let mut status = ClaimStatus::Rejected;
    let err = status.transition_to_rejected().unwrap_err();
    assert_eq!(err, ContractError::InvalidStatus);
    assert_eq!(status, ClaimStatus::Rejected);
}

#[test]
fn test_approve_after_reject_returns_invalid_status_error() {
    let mut status = ClaimStatus::Rejected;
    let err = status.transition_to_approved().unwrap_err();
    assert_eq!(err, ContractError::InvalidStatus);
    // Status must remain Rejected (not mutated)
    assert_eq!(status, ClaimStatus::Rejected);
}

#[test]
fn test_reject_after_approve_returns_invalid_status_error() {
    let mut status = ClaimStatus::Approved;
    let err = status.transition_to_rejected().unwrap_err();
    assert_eq!(err, ContractError::InvalidStatus);
    // Status must remain Approved (not mutated)
    assert_eq!(status, ClaimStatus::Approved);
}

// ── bulk assertions covering every state ─────────────────────────────────────

#[test]
fn test_all_terminal_states_cannot_transition() {
    let terminal_states = [ClaimStatus::Approved, ClaimStatus::Rejected];
    let all_states = [
        ClaimStatus::Pending,
        ClaimStatus::Approved,
        ClaimStatus::Rejected,
    ];

    for &from in &terminal_states {
        for &to in &all_states {
            assert!(
                !from.can_transition_to(to),
                "Terminal state {:?} should not be able to transition to {:?}",
                from,
                to
            );
        }
    }
}

#[test]
fn test_only_pending_can_transition() {
    // Pending can transition to Approved and Rejected, but not to itself.
    assert!(ClaimStatus::Pending.can_transition_to(ClaimStatus::Approved));
    assert!(ClaimStatus::Pending.can_transition_to(ClaimStatus::Rejected));
    assert!(!ClaimStatus::Pending.can_transition_to(ClaimStatus::Pending));

    // Neither terminal state can transition to anything.
    for &from in &[ClaimStatus::Approved, ClaimStatus::Rejected] {
        for &to in &[
            ClaimStatus::Pending,
            ClaimStatus::Approved,
            ClaimStatus::Rejected,
        ] {
            assert!(
                !from.can_transition_to(to),
                "{:?} should not transition to {:?}",
                from,
                to
            );
        }
    }
}

// ── ClaimKind is importable alongside ClaimStatus (smoke-test) ───────────────

#[test]
fn test_claim_kind_variants_are_accessible() {
    // Just verifies the import compiles and variants exist.
    let _ownership = ClaimKind::Ownership;
    let _contract = ClaimKind::ContractAddress;
}
