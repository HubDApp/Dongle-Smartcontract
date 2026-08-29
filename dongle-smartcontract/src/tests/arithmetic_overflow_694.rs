//! Tests for issue #694 — Integer Overflow/Underflow in Arithmetic Operations.
//!
//! Audited every raw (non-`checked_`/`saturating_`) `+`, `-`, `*` in
//! non-test source under `dongle-smartcontract/src` for a value that could
//! realistically be pushed toward its type's bound by caller-controlled
//! input, rather than a loop counter already bounded by the collection it
//! walks. Findings:
//!
//! - `RatingCalculator::add_rating` used raw `current_sum + scaled_rating`
//!   and `current_count + 1`, while its siblings `update_rating` and
//!   `remove_rating` in the same `impl` block both use
//!   `saturating_sub`/`saturating_add` for the equivalent aggregate math —
//!   exactly the "usage may be inconsistent" pattern the issue describes.
//!   Fixed to `saturating_add`, matching the convention its siblings
//!   already established (a rating aggregate degrading gracefully by
//!   clamping is the existing, intentional choice here — nothing about it
//!   should abort a review submission with a hard error).
//! - `FeeManager::record_verification_refund` already guards its refund
//!   accumulator with `checked_add(..).ok_or(ContractError::ArithmeticOverflow)`,
//!   but no test exercised that branch — `grep`-ing the whole tests/
//!   directory for `ArithmeticOverflow` found zero matches before this file.
//!   Added a direct test for it below.
//! - Every other raw arithmetic site found in non-test source (loop
//!   counters in project_registry.rs/admin_action_log.rs/pagination.rs
//!   guarded by their own `while`/`for` bounds; the digit-formatting buffer
//!   walk in dependency_registry.rs sized exactly for u64's max digit
//!   count; the underflow-looking `total - keep` in
//!   verification_registry/storage.rs already clamped via
//!   `keep = min(keep_count, total)` one line above) was already provably
//!   safe by construction — see each file's own comments for the specific
//!   reasoning where it wasn't obvious inline.

#![cfg(test)]

use crate::errors::ContractError;
use crate::fee_manager::FeeManager;
use crate::rating_calculator::RatingCalculator;
use crate::storage_keys::ExtensionKey;
use crate::tests::fixtures::{create_test_project, setup_contract};
use crate::types::FeeRefundRecord;
use soroban_sdk::{testutils::Address as _, Address, Env};

// ─── RatingCalculator::add_rating — saturating, not panicking/wrapping ────────

#[test]
fn add_rating_sum_saturates_instead_of_wrapping_at_the_type_boundary() {
    // current_sum sits one below u64::MAX; adding any positive rating would
    // wrap silently under plain `+` in a release build (or panic in a debug
    // build) — it must saturate at u64::MAX instead.
    let (new_sum, new_count, _avg) = RatingCalculator::add_rating(u64::MAX - 10, 5, 5);
    assert_eq!(new_sum, u64::MAX, "rating sum must saturate, not wrap");
    assert_eq!(new_count, 6);
}

#[test]
fn add_rating_count_saturates_instead_of_wrapping_at_the_type_boundary() {
    let (_sum, new_count, _avg) = RatingCalculator::add_rating(0, u32::MAX, 5);
    assert_eq!(new_count, u32::MAX, "review count must saturate, not wrap to 0");
}

#[test]
fn add_rating_matches_update_and_remove_ratings_saturating_convention() {
    // Sanity check that all three aggregate-mutating functions in this impl
    // block now agree: none of them can wrap past their type's bound.
    let (sum_after_add, count_after_add, _) = RatingCalculator::add_rating(u64::MAX, u32::MAX, 5);
    assert_eq!(sum_after_add, u64::MAX);
    assert_eq!(count_after_add, u32::MAX);

    let (sum_after_remove, count_after_remove, _) =
        RatingCalculator::remove_rating(0, 0, 5);
    assert_eq!(sum_after_remove, 0, "remove_rating already saturates at 0");
    assert_eq!(count_after_remove, 0);

    let (sum_after_update, _, _) = RatingCalculator::update_rating(0, 1, 5, 5);
    assert_eq!(sum_after_update, 0, "update_rating already saturates at 0");
}

// ─── FeeManager::record_verification_refund — checked_add + ArithmeticOverflow ─
//
// Seeded directly into storage via `env.as_contract` rather than driven
// through the full reject-verification flow: reaching u128::MAX through
// real fee payments would need a token amount at that same scale, but paid
// fees move through Stellar tokens (i128), a different, smaller bound than
// the u128 refund ledger this function itself accumulates into.

#[test]
fn record_verification_refund_returns_arithmetic_overflow_when_accumulator_would_wrap() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "OverflowRefundProject");
    let contract_id = client.address.clone();
    let payer = Address::generate(&env);

    // Seed an existing unclaimed refund sitting one below u128::MAX, so the
    // next accumulation is guaranteed to overflow.
    env.as_contract(&contract_id, || {
        let existing = FeeRefundRecord {
            project_id,
            request_id: 1,
            payer: payer.clone(),
            amount: u128::MAX - 1,
            token: None,
            created_at: env.ledger().timestamp(),
            claimed_at: None,
        };
        env.storage()
            .persistent()
            .set(&ExtensionKey::FeeRefund(project_id), &existing);
    });

    let result = env.as_contract(&contract_id, || {
        FeeManager::record_verification_refund(&env, project_id, 2, payer.clone(), 2)
    });

    assert_eq!(
        result,
        Err(ContractError::ArithmeticOverflow),
        "accumulating a refund past u128::MAX must return ArithmeticOverflow, not wrap"
    );

    // The pre-existing refund record must be untouched by the failed attempt.
    let unchanged: FeeRefundRecord = env.as_contract(&contract_id, || {
        env.storage()
            .persistent()
            .get(&ExtensionKey::FeeRefund(project_id))
            .unwrap()
    });
    assert_eq!(unchanged.amount, u128::MAX - 1);
}

#[test]
fn record_verification_refund_accumulates_normally_below_the_boundary() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "NormalRefundProject");
    let contract_id = client.address.clone();
    let payer = Address::generate(&env);

    env.as_contract(&contract_id, || {
        let existing = FeeRefundRecord {
            project_id,
            request_id: 1,
            payer: payer.clone(),
            amount: 100,
            token: None,
            created_at: env.ledger().timestamp(),
            claimed_at: None,
        };
        env.storage()
            .persistent()
            .set(&ExtensionKey::FeeRefund(project_id), &existing);
    });

    let result = env.as_contract(&contract_id, || {
        FeeManager::record_verification_refund(&env, project_id, 2, payer.clone(), 50)
    });

    let refund = result.expect("well under the boundary, must succeed").unwrap();
    assert_eq!(refund.amount, 150, "refund amounts must accumulate normally below the boundary");
}
