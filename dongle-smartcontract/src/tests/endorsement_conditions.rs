//! Tests for area-scoped and time-limited endorsements (#918).

#![cfg(test)]

use crate::tests::fixtures::{create_test_project, setup_contract};
use crate::ContractError;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    vec, Address, Env, String, Vec,
};

fn s(env: &Env, v: &str) -> String {
    String::from_str(env, v)
}

#[test]
fn plain_endorsement_creates_unconditional_record() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let user = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "Plain Endorsed");

    client.endorse_project(&project_id, &user);

    let record = client.get_user_project_endorsement(&project_id, &user).unwrap();
    assert_eq!(record.areas.len(), 0);
    assert_eq!(record.expires_at, None);
    assert!(client.has_active_endorsement(&project_id, &user));
}

#[test]
fn endorsements_filter_by_area() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let defi_expert = Address::generate(&env);
    let generalist = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "Area Project");

    client.endorse_project_with_conditions(
        &project_id,
        &defi_expert,
        &vec![&env, s(&env, "DeFi")],
        &None,
    );
    client.endorse_project_with_conditions(
        &project_id,
        &generalist,
        &vec![&env, s(&env, "DeFi"), s(&env, "Security")],
        &None,
    );

    let defi = client.get_endorsements_by_area(&project_id, &s(&env, "DeFi"), &0, &10);
    assert_eq!(defi.len(), 2);
    let security = client.get_endorsements_by_area(&project_id, &s(&env, "Security"), &0, &10);
    assert_eq!(security.len(), 1);
    assert_eq!(security.get(0).unwrap().endorser, generalist);

    let both = client.filter_endorsers_by_areas(
        &project_id,
        &vec![&env, s(&env, "DeFi"), s(&env, "Security")],
    );
    assert_eq!(both, vec![&env, generalist.clone()]);

    // Unendorsing drops the record from every area index.
    client.unendorse_project(&project_id, &generalist);
    assert_eq!(
        client
            .get_endorsements_by_area(&project_id, &s(&env, "Security"), &0, &10)
            .len(),
        0
    );
}

#[test]
fn invalid_areas_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let user = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "Invalid Area");

    assert_eq!(
        client.try_endorse_project_with_conditions(
            &project_id,
            &user,
            &vec![&env, s(&env, "DeFi"), s(&env, "DeFi")],
            &None,
        ),
        Err(Ok(ContractError::InvalidEndorsementArea))
    );
    assert_eq!(
        client.try_endorse_project_with_conditions(
            &project_id,
            &user,
            &vec![&env, s(&env, "")],
            &None,
        ),
        Err(Ok(ContractError::InvalidEndorsementArea))
    );

    let mut many: Vec<String> = Vec::new(&env);
    for name in ["a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k"] {
        many.push_back(s(&env, name));
    }
    assert_eq!(
        client.try_endorse_project_with_conditions(&project_id, &user, &many, &None),
        Err(Ok(ContractError::TooManyEndorsementAreas))
    );
}

#[test]
fn time_limited_endorsement_auto_expires() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().with_mut(|li| li.timestamp = 1_000);
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let user = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "Expiring");

    assert_eq!(
        client.try_endorse_project_with_conditions(
            &project_id,
            &user,
            &vec![&env, s(&env, "DeFi")],
            &Some(1_000),
        ),
        Err(Ok(ContractError::InvalidEndorsementExpiry))
    );

    client.endorse_project_with_conditions(
        &project_id,
        &user,
        &vec![&env, s(&env, "DeFi")],
        &Some(2_000),
    );
    assert!(client.has_active_endorsement(&project_id, &user));

    env.ledger().with_mut(|li| li.timestamp = 2_000);
    assert!(!client.has_active_endorsement(&project_id, &user));
    assert_eq!(
        client
            .get_endorsements_by_area(&project_id, &s(&env, "DeFi"), &0, &10)
            .len(),
        0
    );
}

#[test]
fn renew_extends_by_original_period() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().with_mut(|li| li.timestamp = 1_000);
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let user = Address::generate(&env);
    let other = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "Renewable");

    let id = client.endorse_project_with_conditions(
        &project_id,
        &user,
        &vec![&env, s(&env, "DeFi")],
        &Some(1_500),
    );

    env.ledger().with_mut(|li| li.timestamp = 3_000);
    assert!(!client.has_active_endorsement(&project_id, &user));

    assert_eq!(
        client.try_renew_endorsement(&other, &id),
        Err(Ok(ContractError::Unauthorized))
    );

    let new_expiry = client.renew_endorsement(&user, &id);
    assert_eq!(new_expiry, 3_500);
    assert!(client.has_active_endorsement(&project_id, &user));
    assert_eq!(client.get_endorsement(&id).unwrap().renewed_at, Some(3_000));
}

#[test]
fn permanent_endorsement_is_not_renewable() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let user = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "Permanent");

    let id = client.endorse_project_with_conditions(&project_id, &user, &Vec::new(&env), &None);

    assert_eq!(
        client.try_renew_endorsement(&user, &id),
        Err(Ok(ContractError::EndorsementNotRenewable))
    );
    assert_eq!(
        client.try_renew_endorsement(&user, &999),
        Err(Ok(ContractError::EndorsementNotFound))
    );
}
