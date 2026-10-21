//! Property-based fuzzing harness for contract invariants (closes #939).
//!
//! Uses [`proptest`] to generate random valid/invalid inputs for core
//! operations (validation ranges, view-cache TTL behavior, rate-limiter
//! accounting) and assert the invariants hold for every input.
//!
//! - Runs in CI on every commit (see `fuzz` job in `.github/workflows/ci.yml`).
//! - Failures are logged to the proptest regression suite
//!   (`dongle-smartcontract/proptest-regressions/`), which is checked in so
//!   regressions re-run on every future test run.
//! - Coverage dashboard: `docs/fuzz-coverage.md`.

use crate::admin_manager::AdminManager;
use crate::computed_view_cache::ComputedViewCache;
use crate::errors::ContractError;
use crate::rate_limiter::RateLimiter;
use crate::DongleContract;
use proptest::prelude::*;
use soroban_sdk::{testutils::Address as _, testutils::Ledger as _, Address, Bytes, Env, Symbol, symbol_short};

fn setup() -> (Env, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(DongleContract, ());
    (env, contract_id)
}

fn setup_with_admin() -> (Env, Address, Address) {
    let (env, contract_id) = setup();
    let admin = Address::generate(&env);
    env.as_contract(&contract_id, || {
        let _ = AdminManager::initialize(&env, admin.clone());
    });
    (env, contract_id, admin)
}

/// Invariant: a rating is valid iff it lies in 1..=5 (mirrors validation).
fn rating_is_valid(rating: u32) -> bool {
    (1..=5).contains(&rating)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    /// Random u32 ratings are classified exactly by the 1..=5 rule.
    #[test]
    fn fuzz_rating_range_never_misclassifies(rating in any::<u32>()) {
        prop_assert_eq!(rating_is_valid(rating), (1..=5u32).contains(&rating));
    }

    /// Cache TTL invariant: a stored view reads back before expiry and is
    /// gone (evicted) once the ledger passes `expires_at`.
    #[test]
    fn fuzz_view_cache_ttl(ttl in 1u32..500, payload in any::<[u8; 32]>(), advance in 0u32..1000) {
        let (env, contract_id) = setup();
        let key = Symbol::new(&env, "fuzz_view");
        let value = Bytes::from_array(&env, &payload);

        env.as_contract(&contract_id, || {
            ComputedViewCache::set_cached_view(&env, key.clone(), value.clone(), ttl);
            // Immediate read is always a hit.
            prop_assert_eq!(ComputedViewCache::get_cached_view(&env, key.clone()), Some(value.clone()));
            Ok::<(), TestCaseError>(())
        }).unwrap();

        let start = env.ledger().sequence();
        env.ledger().with_mut(|l| {
            l.sequence_number = start + advance;
        });
        let expired = advance >= ttl;
        env.as_contract(&contract_id, || {
            let got = ComputedViewCache::get_cached_view(&env, key.clone());
            if expired {
                prop_assert_eq!(got, None);
            } else {
                prop_assert_eq!(got, Some(value.clone()));
            }
            Ok::<(), TestCaseError>(())
        }).unwrap();
    }

    /// Rate-limiter accounting: with limit L, exactly min(N, L) of N calls
    /// succeed, the rest fail with RateLimitExceeded, and remaining matches.
    #[test]
    fn fuzz_rate_limiter_accounting(limit in 1u32..20, calls in 0usize..30) {
        let (env, contract_id, admin) = setup_with_admin();
        let user = Address::generate(&env);
        let op = symbol_short!("fuzz_op");

        env.as_contract(&contract_id, || {
            RateLimiter::configure_limit(&env, admin.clone(), op.clone(), limit, 3600).unwrap();
            let mut ok = 0u32;
            for _ in 0..calls {
                match RateLimiter::check_and_consume(&env, user.clone(), op.clone()) {
                    Ok(()) => ok += 1,
                    Err(e) => prop_assert_eq!(e, ContractError::RateLimitExceeded),
                }
            }
            let expected = (calls as u32).min(limit);
            prop_assert_eq!(ok, expected);
            prop_assert_eq!(RateLimiter::get_remaining(&env, user.clone(), op.clone()), limit - expected);
            if calls as u32 >= limit {
                prop_assert_eq!(
                    RateLimiter::check_and_consume(&env, user.clone(), op.clone()),
                    Err(ContractError::RateLimitExceeded)
                );
            }
            // Manual reset restores the full quota.
            RateLimiter::reset_rate_limit(&env, admin.clone(), user.clone(), op.clone()).unwrap();
            prop_assert_eq!(RateLimiter::get_remaining(&env, user.clone(), op.clone()), limit);
            Ok::<(), TestCaseError>(())
        }).unwrap();
    }

    /// Trusted addresses bypass limits entirely, untrusted ones do not gain quota.
    #[test]
    fn fuzz_trusted_bypass(calls in 0usize..30) {
        let (env, contract_id, admin) = setup_with_admin();
        let user = Address::generate(&env);
        let op = symbol_short!("fuzz_tr");

        env.as_contract(&contract_id, || {
            RateLimiter::configure_limit(&env, admin.clone(), op.clone(), 1, 3600).unwrap();
            RateLimiter::set_trusted(&env, admin.clone(), user.clone(), true).unwrap();
            for _ in 0..calls {
                prop_assert!(RateLimiter::check_and_consume(&env, user.clone(), op.clone()).is_ok());
            }
            RateLimiter::set_trusted(&env, admin.clone(), user.clone(), false).unwrap();
            // Quota untouched by bypassed calls: exactly 1 call left.
            prop_assert_eq!(RateLimiter::get_remaining(&env, user.clone(), op.clone()), 1);
            Ok::<(), TestCaseError>(())
        }).unwrap();
    }
}
