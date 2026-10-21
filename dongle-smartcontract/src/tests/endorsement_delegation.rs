//! Tests for delegated endorsement authority (#919).

#![cfg(test)]

use crate::tests::fixtures::{create_test_project, setup_contract};
use crate::ContractError;
use soroban_sdk::{testutils::Address as _, vec, Address, Env, String, Vec};

fn s(env: &Env, v: &str) -> String {
    String::from_str(env, v)
}

#[test]
fn delegation_requires_endorsement_authority() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let kol = Address::generate(&env);
    let delegate = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "Delegated Project");

    assert_eq!(
        client.try_delegate_endorsement_authority(&project_id, &kol, &delegate, &Vec::new(&env)),
        Err(Ok(ContractError::NoEndorsementAuthority))
    );

    client.endorse_project(&project_id, &kol);
    assert_eq!(
        client.try_delegate_endorsement_authority(&project_id, &kol, &kol, &Vec::new(&env)),
        Err(Ok(ContractError::SelfDelegation))
    );

    client.delegate_endorsement_authority(&project_id, &kol, &delegate, &Vec::new(&env));
    assert!(client.has_endorsement_authority(&project_id, &delegate));
    assert_eq!(
        client.get_endorsement_delegates(&project_id, &kol),
        vec![&env, delegate.clone()]
    );
    let record = client.get_user_project_endorsement(&project_id, &kol).unwrap();
    assert_eq!(record.delegated_to, Some(vec![&env, delegate.clone()]));
}

#[test]
fn delegate_endorses_on_behalf_within_scope() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let kol = Address::generate(&env);
    let delegate = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "Scoped Project");

    client.endorse_project_with_conditions(
        &project_id,
        &kol,
        &vec![&env, s(&env, "DeFi"), s(&env, "Security")],
        &None,
    );

    // Scope wider than the delegator's own areas is rejected.
    assert_eq!(
        client.try_delegate_endorsement_authority(
            &project_id,
            &kol,
            &delegate,
            &vec![&env, s(&env, "Gaming")],
        ),
        Err(Ok(ContractError::DelegationScopeExceeded))
    );

    client.delegate_endorsement_authority(
        &project_id,
        &kol,
        &delegate,
        &vec![&env, s(&env, "DeFi")],
    );

    assert_eq!(
        client.try_endorse_project_on_behalf(
            &project_id,
            &delegate,
            &vec![&env, s(&env, "Security")],
            &None,
        ),
        Err(Ok(ContractError::DelegationScopeExceeded))
    );

    let id = client.endorse_project_on_behalf(&project_id, &delegate, &Vec::new(&env), &None);
    let record = client.get_endorsement(&id).unwrap();
    assert_eq!(record.on_behalf_of, Some(kol.clone()));
    assert_eq!(record.areas, vec![&env, s(&env, "DeFi")]);
    assert!(client.has_endorsed(&project_id, &delegate));
}

#[test]
fn delegation_chain_is_tracked_and_cycles_prevented() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let a = Address::generate(&env);
    let b = Address::generate(&env);
    let c = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "Chain Project");

    client.endorse_project(&project_id, &a);
    client.delegate_endorsement_authority(&project_id, &a, &b, &Vec::new(&env));
    client.delegate_endorsement_authority(&project_id, &b, &c, &Vec::new(&env));

    assert_eq!(
        client.get_endorsement_delegation_chain(&project_id, &c),
        vec![&env, c.clone(), b.clone(), a.clone()]
    );
    assert!(client.has_endorsement_authority(&project_id, &c));

    // c → a would close the loop a → b → c → a.
    assert_eq!(
        client.try_delegate_endorsement_authority(&project_id, &c, &a, &Vec::new(&env)),
        Err(Ok(ContractError::DelegationCycleDetected))
    );
    // b already holds authority from a.
    assert_eq!(
        client.try_delegate_endorsement_authority(&project_id, &c, &b, &Vec::new(&env)),
        Err(Ok(ContractError::AlreadyDelegated))
    );
}

#[test]
fn revocation_suspends_downstream_authority() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let a = Address::generate(&env);
    let b = Address::generate(&env);
    let c = Address::generate(&env);
    let stranger = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "Revoke Project");

    client.endorse_project(&project_id, &a);
    client.delegate_endorsement_authority(&project_id, &a, &b, &Vec::new(&env));
    client.delegate_endorsement_authority(&project_id, &b, &c, &Vec::new(&env));

    assert_eq!(
        client.try_revoke_endorsement_delegation(&project_id, &stranger, &b),
        Err(Ok(ContractError::DelegationNotFound))
    );

    client.revoke_endorsement_delegation(&project_id, &a, &b);

    assert!(client.get_endorsement_delegation(&project_id, &b).is_none());
    assert!(!client.has_endorsement_authority(&project_id, &b));
    assert!(!client.has_endorsement_authority(&project_id, &c));
    assert_eq!(
        client.try_endorse_project_on_behalf(&project_id, &c, &Vec::new(&env), &None),
        Err(Ok(ContractError::NoEndorsementAuthority))
    );
    let record = client.get_user_project_endorsement(&project_id, &a).unwrap();
    assert_eq!(record.delegated_to, None);
}

#[test]
fn root_unendorsing_suspends_delegates() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let a = Address::generate(&env);
    let b = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "Root Leaves");

    client.endorse_project(&project_id, &a);
    client.delegate_endorsement_authority(&project_id, &a, &b, &Vec::new(&env));
    client.unendorse_project(&project_id, &a);

    assert!(!client.has_endorsement_authority(&project_id, &b));
}
