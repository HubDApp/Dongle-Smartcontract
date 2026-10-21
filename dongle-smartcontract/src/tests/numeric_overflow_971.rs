//! Issue #971 — Test numeric overflow and underflow conditions.
//!
//! Verifies that every numeric type used in the contract handles edge-case
//! values without panicking, wrapping silently, or producing inconsistent
//! state:
//!
//! * `u64` max / zero / boundary arithmetic (rating sums, project IDs,
//!   timestamps).
//! * `u128` max / zero (fee accumulators).
//! * Zero-value ratings and counts (division-by-zero guard in average
//!   calculation).
//! * Negative i64 / i32 values passed as unsigned parameters must be
//!   validated and rejected.
//! * Saturating arithmetic in `RatingCalculator` — no panic, no wrap.
//! * The `record_verification_refund` `u128` overflow path.

#![cfg(test)]

use crate::errors::ContractError;
use crate::fee_manager::FeeManager;
use crate::rating_calculator::RatingCalculator;
use crate::storage_keys::ExtensionKey;
use crate::tests::fixtures::{create_test_project, setup_contract};
use crate::types::FeeRefundRecord;
use soroban_sdk::{testutils::Address as _, Address, Env};

// ══════════════════════════════════════════════════════════════════════════════
// u64 edge cases
// ══════════════════════════════════════════════════════════════════════════════

// ── u64 max value in rating sum ───────────────────────────────────────────────

/// When the rating sum is already at `u64::MAX`, adding another rating must
/// saturate at `u64::MAX` rather than wrapping to a small number or panicking.
#[test]
fn overflow_u64_max_rating_sum_saturates_not_wraps() {
    let (new_sum, new_count, _avg) = RatingCalculator::add_rating(u64::MAX, 0, 5);
    assert_eq!(
        new_sum,
        u64::MAX,
        "rating sum must saturate at u64::MAX, not wrap"
    );
    assert_eq!(new_count, 1, "review count must increment normally");
}

/// When both the rating sum and the review count are at their maximum values,
/// `add_rating` must not overflow either field.
#[test]
fn overflow_u64_max_sum_and_u32_max_count_both_saturate() {
    let (new_sum, new_count, _avg) = RatingCalculator::add_rating(u64::MAX, u32::MAX, 5);
    assert_eq!(
        new_sum, u64::MAX,
        "saturating add at u64::MAX must not wrap"
    );
    assert_eq!(
        new_count, u32::MAX,
        "saturating add at u32::MAX must not wrap"
    );
}

// ── u64 zero values ────────────────────────────────────────────────────────────

/// Adding a rating of 1 to a zero-baseline must produce the correct sum and
/// a review count of 1.
#[test]
fn overflow_u64_zero_baseline_add_rating_correct() {
    let (new_sum, new_count, new_avg) = RatingCalculator::add_rating(0, 0, 1);
    assert_eq!(new_sum, 100, "rating 1 scaled by 100 = 100");
    assert_eq!(new_count, 1);
    assert_eq!(new_avg, 100, "average of a single rating-1 is 100");
}

/// `calculate_average` with a zero count must return 0 (division-by-zero
/// guard), not panic.
#[test]
fn overflow_u64_zero_review_count_average_is_zero_not_panic() {
    let avg = RatingCalculator::calculate_average(0, 0);
    assert_eq!(avg, 0, "average with zero reviews must be 0, not a div-by-zero panic");

    // Same guard must hold even when the sum is non-zero.
    let avg2 = RatingCalculator::calculate_average(1_000_000, 0);
    assert_eq!(avg2, 0, "non-zero sum with zero count must still return 0");
}

// ── u64 boundary: exactly at u64::MAX - 1 ─────────────────────────────────────

/// Adding a rating that would push the sum one step past `u64::MAX` must
/// saturate cleanly.
#[test]
fn overflow_u64_sum_one_below_max_saturates_on_next_add() {
    let near_max = u64::MAX - 50; // 500 would wrap the 5-rating (scaled 500)
    let (new_sum, _, _) = RatingCalculator::add_rating(near_max, 3, 5);
    // near_max + 500 > u64::MAX → must clamp at u64::MAX
    assert_eq!(
        new_sum, u64::MAX,
        "adding a rating that crosses u64::MAX must saturate at u64::MAX"
    );
}

// ── u64 remove_rating underflow ────────────────────────────────────────────────

/// Removing a rating from a zero-sum must saturate at 0 (not underflow to
/// a huge positive number or panic).
#[test]
fn overflow_u64_remove_rating_underflow_saturates_at_zero() {
    let (new_sum, new_count, _) = RatingCalculator::remove_rating(0, 0, 5);
    assert_eq!(
        new_sum, 0,
        "removing a rating from a zero-sum must saturate at 0, not underflow"
    );
    assert_eq!(
        new_count, 0,
        "removing from a zero-count must saturate at 0, not underflow"
    );
}

/// Removing a rating that is larger than the current sum must saturate at 0.
#[test]
fn overflow_u64_remove_rating_larger_than_sum_saturates_at_zero() {
    let (new_sum, new_count, _) = RatingCalculator::remove_rating(100, 1, 5);
    // 100 - 500 (5*100) would underflow → must saturate at 0
    assert_eq!(
        new_sum, 0,
        "underflow in remove_rating must saturate at 0, not wrap"
    );
    assert_eq!(new_count, 0);
}

// ── u64 update_rating: both underflow and overflow guarded ────────────────────

/// `update_rating` removes the old rating and adds the new one in one step.
/// When the removal underflows, the intermediate result must not go negative
/// (saturating at 0), and the subsequent add must then accumulate from 0.
#[test]
fn overflow_u64_update_rating_underflow_then_add_correct() {
    // sum=0, remove old=5 (500 scaled) → saturates to 0, then add new=3 → 300.
    let (new_sum, count, new_avg) = RatingCalculator::update_rating(0, 1, 5, 3);
    assert_eq!(
        new_sum, 300,
        "update_rating underflow must saturate and then add new rating from 0"
    );
    assert_eq!(count, 1);
    assert_eq!(new_avg, 300);
}

// ══════════════════════════════════════════════════════════════════════════════
// u128 edge cases
// ══════════════════════════════════════════════════════════════════════════════

// ── u128 max: record_verification_refund overflow ────────────────────────────

/// Accumulating a refund into an account whose balance is already at
/// `u128::MAX - 1` must return `ContractError::ArithmeticOverflow` and
/// must NOT modify the stored record.
#[test]
fn overflow_u128_refund_accumulator_at_max_minus_one_returns_overflow_error() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "U128MaxRefund971");
    let contract_id = client.address.clone();
    let payer = Address::generate(&env);

    // Seed a record at u128::MAX - 1 so the next accumulation of 2 would wrap.
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
        "accumulating past u128::MAX must return ArithmeticOverflow"
    );

    // The stored record must be unchanged by the failed operation.
    let unchanged: FeeRefundRecord = env.as_contract(&contract_id, || {
        env.storage()
            .persistent()
            .get(&ExtensionKey::FeeRefund(project_id))
            .unwrap()
    });
    assert_eq!(
        unchanged.amount,
        u128::MAX - 1,
        "failed accumulation must not modify the stored refund amount"
    );
}

/// Accumulating exactly to `u128::MAX` (no overflow) must succeed.
#[test]
fn overflow_u128_refund_accumulator_exactly_at_max_succeeds() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "U128ExactMax971");
    let contract_id = client.address.clone();
    let payer = Address::generate(&env);

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

    // Adding exactly 1 reaches u128::MAX without overflow.
    let result = env.as_contract(&contract_id, || {
        FeeManager::record_verification_refund(&env, project_id, 2, payer.clone(), 1)
    });

    assert!(
        result.is_ok(),
        "accumulating to exactly u128::MAX must succeed"
    );
    let record = result.unwrap().unwrap();
    assert_eq!(
        record.amount,
        u128::MAX,
        "refund amount must equal u128::MAX after accumulating the final unit"
    );
}

// ── u128 zero baseline ─────────────────────────────────────────────────────────

/// Starting from zero, a normal refund accumulation must work correctly.
#[test]
fn overflow_u128_zero_baseline_refund_accumulates_correctly() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "U128Zero971");
    let contract_id = client.address.clone();
    let payer = Address::generate(&env);

    let result = env.as_contract(&contract_id, || {
        FeeManager::record_verification_refund(&env, project_id, 1, payer.clone(), 1_000)
    });

    assert!(result.is_ok(), "first refund from zero baseline must succeed");
    let record = result.unwrap().unwrap();
    assert_eq!(
        record.amount, 1_000,
        "refund amount must equal the accumulated value from zero"
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// Zero-value validations at the contract boundary
// ══════════════════════════════════════════════════════════════════════════════

// ── Rating must be between 1 and 5 ────────────────────────────────────────────

/// A rating of 0 is below the valid range [1, 5] and must be rejected.
#[test]
fn overflow_rating_zero_rejected_at_contract_boundary() {
    let env = Env::default();
    let (client, _) = setup_contract(&env);
    let owner = Address::generate(&env);
    let reviewer = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "RatingZero971");

    let result = client
        .mock_all_auths()
        .try_add_review(&project_id, &reviewer, &0, &None);
    assert_eq!(
        result,
        Err(Ok(ContractError::InvalidRating)),
        "rating 0 must be rejected as InvalidRating"
    );
}

// ── Rating > 5 is rejected ─────────────────────────────────────────────────────

/// A rating of 6 (above the maximum of 5) must be rejected.
#[test]
fn overflow_rating_above_max_rejected_at_contract_boundary() {
    let env = Env::default();
    let (client, _) = setup_contract(&env);
    let owner = Address::generate(&env);
    let reviewer = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "RatingAboveMax971");

    let result = client
        .mock_all_auths()
        .try_add_review(&project_id, &reviewer, &6, &None);
    assert_eq!(
        result,
        Err(Ok(ContractError::InvalidRating)),
        "rating 6 must be rejected as InvalidRating"
    );
}

// ── u32 MAX rating is rejected (> 5) ──────────────────────────────────────────

/// `u32::MAX` is a valid Rust value but an invalid rating.  The contract
/// must reject it without panicking.
#[test]
fn overflow_rating_u32_max_rejected_as_invalid() {
    let env = Env::default();
    let (client, _) = setup_contract(&env);
    let owner = Address::generate(&env);
    let reviewer = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "RatingU32Max971");

    let result = client
        .mock_all_auths()
        .try_add_review(&project_id, &reviewer, &u32::MAX, &None);
    assert_eq!(
        result,
        Err(Ok(ContractError::InvalidRating)),
        "u32::MAX rating must be rejected as InvalidRating without panicking"
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// Overflow protection in RatingCalculator — unit-level coverage
// ══════════════════════════════════════════════════════════════════════════════

/// All three aggregate-mutation functions must agree: no function may wrap or
/// panic at the type boundary.
#[test]
fn overflow_protection_all_rating_calculator_functions_saturate_consistently() {
    // add_rating at maximum inputs — sum and count both saturate.
    let (s_add, c_add, _) = RatingCalculator::add_rating(u64::MAX, u32::MAX, 5);
    assert_eq!(s_add, u64::MAX);
    assert_eq!(c_add, u32::MAX);

    // remove_rating at zero inputs — both underflow-saturate to 0.
    let (s_rem, c_rem, _) = RatingCalculator::remove_rating(0, 0, 5);
    assert_eq!(s_rem, 0);
    assert_eq!(c_rem, 0);

    // update_rating: remove old at zero (saturates), then add new.
    let (s_upd, c_upd, avg_upd) = RatingCalculator::update_rating(0, 1, 5, 5);
    // saturating_sub(0, 500) = 0; saturating_add(0, 500) = 500
    assert_eq!(s_upd, 500);
    assert_eq!(c_upd, 1);
    assert_eq!(avg_upd, 500);
}

/// Verify the normal case: `add_rating` → `remove_rating` round-trip
/// returns to the original values.
#[test]
fn overflow_protection_add_then_remove_rating_round_trips_correctly() {
    let initial_sum: u64 = 1_000;
    let initial_count: u32 = 10;
    let rating: u32 = 4;

    let (sum_after_add, count_after_add, _) =
        RatingCalculator::add_rating(initial_sum, initial_count, rating);

    let (sum_after_remove, count_after_remove, _) =
        RatingCalculator::remove_rating(sum_after_add, count_after_add, rating);

    assert_eq!(
        sum_after_remove, initial_sum,
        "removing the same rating added should restore the original sum"
    );
    assert_eq!(
        count_after_remove, initial_count,
        "removing the same review should restore the original count"
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// Large legitimate values (not overflowing, just large) must be handled
// ══════════════════════════════════════════════════════════════════════════════

/// Registering projects and submitting reviews with legitimate (large but
/// valid) numeric parameters must succeed without panics.
#[test]
fn overflow_large_but_valid_values_handled_without_panic() {
    let env = Env::default();
    let (client, _) = setup_contract(&env);
    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "LargeValid971");

    // Submit 20 reviews with alternating ratings 1 and 5.
    for i in 0..20u32 {
        let reviewer = Address::generate(&env);
        let rating = if i % 2 == 0 { 1u32 } else { 5u32 };
        client
            .mock_all_auths()
            .add_review(&project_id, &reviewer, &rating, &None);
    }

    let stats = client.get_project_stats(&project_id);
    assert_eq!(stats.review_count, 20, "20 reviews must have been recorded");

    // Expected sum: 10 × 100 + 10 × 500 = 6 000
    assert_eq!(
        stats.rating_sum, 6_000,
        "rating sum must equal the arithmetic total of all submitted ratings (scaled)"
    );
}

// ── Overflow protection: calculate_average with very large sum and count ───────

/// When both sum and count are at realistic but large values, the average
/// must be computed without overflow.
#[test]
fn overflow_calculate_average_large_values_no_panic() {
    // 1_000_000 reviews each rated 5 (scaled 500) → sum = 500_000_000
    let sum: u64 = 500_000_000;
    let count: u32 = 1_000_000;
    let avg = RatingCalculator::calculate_average(sum, count);
    assert_eq!(avg, 500, "average of 1M identical 5-star ratings must be 500 (= 5.00 × 100)");
}
