//! Tests for the address endorsement graph and multi-hop chains (#917).

#![cfg(test)]

use crate::tests::fixtures::setup_contract;
use crate::ContractError;
use soroban_sdk::{testutils::Address as _, vec, Address, Env};

#[test]
fn self_endorsement_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let a = Address::generate(&env);

    assert_eq!(
        client.try_endorse_address(&a, &a),
        Err(Ok(ContractError::SelfEndorsement))
    );
}

#[test]
fn direct_endorsement_has_full_weight() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let a = Address::generate(&env);
    let b = Address::generate(&env);

    client.endorse_address(&a, &b);
    assert!(client.has_endorsed_address(&a, &b));
    assert_eq!(
        client.try_endorse_address(&a, &b),
        Err(Ok(ContractError::AddressAlreadyEndorsed))
    );

    let chain = client.get_endorsement_path(&a, &b).unwrap();
    assert_eq!(chain.path, vec![&env, a.clone(), b.clone()]);
    assert_eq!(chain.hops, 1);
    assert_eq!(chain.weight, 10_000);
}

#[test]
fn transitive_path_decays_with_length_and_reputation() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let a = Address::generate(&env);
    let b = Address::generate(&env);
    let c = Address::generate(&env);

    client.endorse_address(&a, &b);
    client.endorse_address(&b, &c);

    let chain = client.get_endorsement_path(&a, &c).unwrap();
    assert_eq!(chain.path, vec![&env, a.clone(), b.clone(), c.clone()]);
    assert_eq!(chain.hops, 2);
    // hop decay 50% × intermediary b (1 endorser) 60% = 30%.
    assert_eq!(chain.weight, 3_000);

    // No path in the reverse direction.
    assert!(client.get_endorsement_path(&c, &a).is_none());
}

#[test]
fn vouchers_include_transitive_endorsers() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let a = Address::generate(&env);
    let b = Address::generate(&env);
    let c = Address::generate(&env);

    client.endorse_address(&a, &b);
    client.endorse_address(&b, &c);

    let vouchers = client.get_vouchers(&c, &0);
    assert_eq!(vouchers.len(), 2);
    assert_eq!(vouchers.get(0).unwrap().voucher, b);
    assert_eq!(vouchers.get(0).unwrap().hops, 1);
    assert_eq!(vouchers.get(1).unwrap().voucher, a);
    assert_eq!(vouchers.get(1).unwrap().hops, 2);

    // Depth 1 only returns direct endorsers.
    assert_eq!(client.get_vouchers(&c, &1).len(), 1);
}

#[test]
fn cycle_detection_finds_loop() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let a = Address::generate(&env);
    let b = Address::generate(&env);
    let c = Address::generate(&env);

    client.endorse_address(&a, &b);
    client.endorse_address(&b, &c);
    assert!(client.detect_endorsement_cycle(&a).is_none());

    client.endorse_address(&c, &a);
    let cycle = client.detect_endorsement_cycle(&a).unwrap();
    assert_eq!(cycle, vec![&env, a.clone(), b.clone(), c.clone(), a.clone()]);

    // Traversals terminate on cyclic graphs.
    assert_eq!(client.get_vouchers(&a, &0).len(), 2);
}

#[test]
fn revoking_breaks_the_chain() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let a = Address::generate(&env);
    let b = Address::generate(&env);
    let c = Address::generate(&env);

    client.endorse_address(&a, &b);
    client.endorse_address(&b, &c);
    client.revoke_address_endorsement(&b, &c);

    assert!(client.get_endorsement_path(&a, &c).is_none());
    assert_eq!(client.get_endorsement_reputation(&c), 0);
    assert_eq!(
        client.try_revoke_address_endorsement(&b, &c),
        Err(Ok(ContractError::AddressNotEndorsed))
    );
}
