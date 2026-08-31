//! Tests for admin concurrent-operation atomicity (issue #670).
//!
//! # Background
//!
//! Soroban executes each transaction atomically and in strict isolation.
//! Two "concurrent" admin operations (e.g. add and remove submitted by
//! different callers in the same ledger close) are *sequenced*, not
//! interleaved.  This means:
//!
//! * The `Admin(addr)` mapping and the `AdminList` vec are always consistent
//!   with each other after any committed transaction.
//! * There can be no "dead admin" — an address that is present in one data
//!   structure but absent from the other.
//! * Conflicting operations receive typed errors rather than silently
//!   corrupting state.
//!
//! The tests below simulate sequential orderings that *represent* concurrent
//! submit scenarios (since the Soroban test harness, like the ledger itself,
//! executes one call at a time) and assert the **invariant** that must hold
//! after every mutation:
//!
//! ```text
//! is_admin(addr)  <=>  admin_list.contains(addr)
//! ```

use crate::DongleContract;
use crate::DongleContractClient;
use crate::errors::ContractError;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{Address, Env};

extern crate alloc;

// ─── helpers ─────────────────────────────────────────────────────────────────

/// Set up a contract with a single initial admin and return the client + admin.
fn setup(env: &Env) -> (DongleContractClient<'_>, Address) {
    let contract_id = env.register(DongleContract, ());
    let client = DongleContractClient::new(env, &contract_id);
    let admin = Address::generate(env);
    client.mock_all_auths().initialize(&admin);
    (client, admin)
}

/// Core invariant: for every address, `is_admin` must agree with `get_admin_list`.
///
/// A failure here means we have a "dead admin" — the two data structures are
/// out of sync, which is the bug described in issue #670.
fn assert_no_dead_admins(client: &DongleContractClient<'_>, addr: &Address) {
    let in_mapping = client.is_admin(addr);
    let admin_list = client.get_admin_list();
    let in_list = admin_list.iter().any(|a| a == *addr);

    assert_eq!(
        in_mapping, in_list,
        "Dead-admin invariant violated for {:?}: \
         is_admin={} but admin_list.contains={}",
        addr, in_mapping, in_list
    );
}

/// Assert that `get_admin_count()` equals `get_admin_list().len()`.
fn assert_count_matches_list(client: &DongleContractClient<'_>) {
    let count = client.get_admin_count();
    let list_len = client.get_admin_list().len();
    assert_eq!(
        count, list_len,
        "Admin count ({}) does not match admin list length ({})",
        count, list_len
    );
}

// ─── individual consistency checks ───────────────────────────────────────────

/// After `add_admin`, both the mapping and the list must reflect the new admin.
#[test]
fn test_add_admin_consistency_mapping_and_list_in_sync() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let new_admin = Address::generate(&env);

    client.mock_all_auths().add_admin(&admin, &new_admin);

    // Invariant: mapping and list agree
    assert_no_dead_admins(&client, &new_admin);
    assert_no_dead_admins(&client, &admin);
    assert_count_matches_list(&client);

    // Explicit assertions
    assert!(client.is_admin(&new_admin));
    let list = client.get_admin_list();
    assert!(list.contains(&new_admin));
    assert_eq!(client.get_admin_count(), 2);
}

/// After `remove_admin`, both the mapping and the list must no longer contain the address.
#[test]
fn test_remove_admin_consistency_mapping_and_list_in_sync() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let second_admin = Address::generate(&env);

    client.mock_all_auths().add_admin(&admin, &second_admin);
    client.mock_all_auths().remove_admin(&admin, &second_admin);

    // Invariant: mapping and list agree
    assert_no_dead_admins(&client, &second_admin);
    assert_no_dead_admins(&client, &admin);
    assert_count_matches_list(&client);

    // Explicit assertions
    assert!(!client.is_admin(&second_admin));
    let list = client.get_admin_list();
    assert!(!list.contains(&second_admin));
    assert_eq!(client.get_admin_count(), 1);
}

// ─── no-dead-admin tests ──────────────────────────────────────────────────────

/// Add then remove: no dead admin must remain.
#[test]
fn test_no_dead_admin_after_add_then_remove() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let target = Address::generate(&env);

    client.mock_all_auths().add_admin(&admin, &target);
    assert_no_dead_admins(&client, &target);

    client.mock_all_auths().remove_admin(&admin, &target);
    assert_no_dead_admins(&client, &target);
    assert_count_matches_list(&client);
}

/// Multiple add/remove cycles must leave no dead admins at any stage.
#[test]
fn test_no_dead_admin_after_multiple_add_remove_cycles() {
    let env = Env::default();
    let (client, admin) = setup(&env);

    let targets: alloc::vec::Vec<Address> = (0..4).map(|_| Address::generate(&env)).collect();

    // Add all
    for t in &targets {
        client.mock_all_auths().add_admin(&admin, t);
        assert_no_dead_admins(&client, t);
        assert_count_matches_list(&client);
    }

    // Remove all except one (need at least 1 admin remaining including initial)
    // Remove targets[0] and targets[1]
    client.mock_all_auths().remove_admin(&admin, &targets[0]);
    assert_no_dead_admins(&client, &targets[0]);
    assert_count_matches_list(&client);

    client.mock_all_auths().remove_admin(&admin, &targets[1]);
    assert_no_dead_admins(&client, &targets[1]);
    assert_count_matches_list(&client);

    // Re-add targets[0] — must be consistent again
    client.mock_all_auths().add_admin(&admin, &targets[0]);
    assert_no_dead_admins(&client, &targets[0]);
    assert_count_matches_list(&client);

    // Verify surviving admins
    assert!(client.is_admin(&admin));
    assert!(!client.is_admin(&targets[1])); // still removed
    assert!(client.is_admin(&targets[0])); // re-added
    assert!(client.is_admin(&targets[2]));
    assert!(client.is_admin(&targets[3]));
}

// ─── last-write-wins / idempotency ───────────────────────────────────────────

/// Adding the same admin twice is idempotent: the second call is a no-op and
/// the list must not contain a duplicate entry.
///
/// This documents the "last-write-wins" / idempotent behaviour for `add_admin`.
#[test]
fn test_last_write_wins_add_admin_idempotent() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let new_admin = Address::generate(&env);

    client.mock_all_auths().add_admin(&admin, &new_admin);
    // Second call: no-op, must not duplicate
    client.mock_all_auths().add_admin(&admin, &new_admin);

    assert_no_dead_admins(&client, &new_admin);
    assert_count_matches_list(&client);

    let list = client.get_admin_list();
    let occurrences = list.iter().filter(|a| *a == new_admin).count();
    assert_eq!(
        occurrences, 1,
        "Duplicate entry in admin list after idempotent add_admin"
    );
    assert_eq!(client.get_admin_count(), 2);
}

// ─── concurrent-sequence simulations ─────────────────────────────────────────

/// Simulate "concurrent" add and remove of the same address.
///
/// In a real ledger two transactions touching the same address-admin slot would
/// be sequenced.  This test tries both orderings to show that whichever wins,
/// the state is always consistent.
#[test]
fn test_concurrent_add_remove_sequence_leaves_consistent_state() {
    // Ordering 1: add wins (add executes after remove attempted but target
    // was not yet admin, so remove returns AdminNotFound, then add succeeds).
    {
        let env = Env::default();
        let (client, admin) = setup(&env);
        let target = Address::generate(&env);

        // "Concurrent" remove arrives first but target is not admin yet → typed error
        let err = client.mock_all_auths().try_remove_admin(&admin, &target);
        assert_eq!(err, Err(Ok(ContractError::AdminNotFound)));

        // add executes second — should succeed
        client.mock_all_auths().add_admin(&admin, &target);

        assert_no_dead_admins(&client, &target);
        assert!(client.is_admin(&target));
        assert_count_matches_list(&client);
    }

    // Ordering 2: remove wins (add first, then remove).
    {
        let env = Env::default();
        let (client, admin) = setup(&env);
        let target = Address::generate(&env);

        client.mock_all_auths().add_admin(&admin, &target);
        client.mock_all_auths().remove_admin(&admin, &target);

        assert_no_dead_admins(&client, &target);
        assert!(!client.is_admin(&target));
        assert_count_matches_list(&client);
    }
}

/// The admin list must exactly reflect every entry present in the mapping:
/// no extra addresses in the list, no missing addresses.
#[test]
fn test_admin_list_reflects_all_mapping_entries() {
    let env = Env::default();
    let (client, admin) = setup(&env);

    let extra: alloc::vec::Vec<Address> = (0..3).map(|_| Address::generate(&env)).collect();
    for e in &extra {
        client.mock_all_auths().add_admin(&admin, e);
    }

    let list = client.get_admin_list();

    // Every address in the list must be in the mapping
    for addr in list.iter() {
        assert!(
            client.is_admin(&addr),
            "Address in list but not in mapping: {:?}",
            addr
        );
    }

    // Every address we added must be in the list
    for e in &extra {
        assert!(
            list.contains(e),
            "Added address missing from list: {:?}",
            e
        );
    }

    assert_count_matches_list(&client);
}

/// add → remove → add cycle must leave the address in a consistent "admin" state.
#[test]
fn test_add_remove_add_cycle_consistent() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let target = Address::generate(&env);

    // First add
    client.mock_all_auths().add_admin(&admin, &target);
    assert_no_dead_admins(&client, &target);
    assert!(client.is_admin(&target));

    // Remove
    client.mock_all_auths().remove_admin(&admin, &target);
    assert_no_dead_admins(&client, &target);
    assert!(!client.is_admin(&target));

    // Re-add
    client.mock_all_auths().add_admin(&admin, &target);
    assert_no_dead_admins(&client, &target);
    assert!(client.is_admin(&target));

    let list = client.get_admin_list();
    let occurrences = list.iter().filter(|a| *a == target).count();
    assert_eq!(occurrences, 1, "Duplicate entry after add-remove-add cycle");
    assert_count_matches_list(&client);
}

// ─── edge cases ───────────────────────────────────────────────────────────────

/// Removing the same admin twice: second attempt returns AdminNotFound and
/// leaves state consistent.
#[test]
fn test_remove_same_admin_twice_second_call_returns_error() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let second = Address::generate(&env);

    client.mock_all_auths().add_admin(&admin, &second);
    client.mock_all_auths().remove_admin(&admin, &second);

    // Second remove — must error, not corrupt state
    let err = client.mock_all_auths().try_remove_admin(&admin, &second);
    assert_eq!(err, Err(Ok(ContractError::AdminNotFound)));

    assert_no_dead_admins(&client, &second);
    assert_count_matches_list(&client);
    assert_eq!(client.get_admin_count(), 1);
}

/// The initial admin is always consistent after initialization.
#[test]
fn test_initial_admin_consistent_after_initialize() {
    let env = Env::default();
    let (client, admin) = setup(&env);

    assert_no_dead_admins(&client, &admin);
    assert_count_matches_list(&client);
    assert_eq!(client.get_admin_count(), 1);
    let list = client.get_admin_list();
    assert_eq!(list.len(), 1);
    assert!(list.contains(&admin));
}

/// Adding many admins and then removing all but one keeps every intermediate
/// state consistent.
#[test]
fn test_large_admin_set_add_and_remove_all_consistent() {
    let env = Env::default();
    let (client, admin) = setup(&env);

    let admins: alloc::vec::Vec<Address> = (0..8).map(|_| Address::generate(&env)).collect();

    for a in &admins {
        client.mock_all_auths().add_admin(&admin, a);
        assert_no_dead_admins(&client, a);
        assert_count_matches_list(&client);
    }

    // Remove them all (initial admin is still there so we never drop to 0)
    for a in &admins {
        client.mock_all_auths().remove_admin(&admin, a);
        assert_no_dead_admins(&client, a);
        assert_count_matches_list(&client);
    }

    assert_eq!(client.get_admin_count(), 1);
    assert!(client.is_admin(&admin));
}
