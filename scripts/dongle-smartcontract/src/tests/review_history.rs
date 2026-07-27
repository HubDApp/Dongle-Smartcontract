//! Review revision history and weighted rating tests (#239, #244).

use crate::tests::fixtures::{create_test_project, setup_contract};
use crate::types::ReviewRevisionEvent;
use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, Events, Ledger},
    Address, Env, IntoVal, String, TryIntoVal,
};

const CID_V1: &str = "QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG";
const CID_V2: &str = "QmYwAPJzv5CZsnAzt8auVZRnG8X1sC3yRyvCb4s46HoPa1";
const CID_V3: &str = "QmYwAPJzv5CZsnAzt8auVZRnG8X1sC3yRyvCb4s46HoPa2";
const CID_W1: &str = "QmYwAPJzv5CZsnAzt8auVZRnG8X1sC3yRyvCb4s46HoPa3";
const CID_W2: &str = "QmYwAPJzv5CZsnAzt8auVZRnG8X1sC3yRyvCb4s46HoPa4";
const CID_W3: &str = "QmYwAPJzv5CZsnAzt8auVZRnG8X1sC3yRyvCb4s46HoPa5";
const CID_W4: &str = "QmYwAPJzv5CZsnAzt8auVZRnG8X1sC3yRyvCb4s46HoPa6";
const CID_W5: &str = "QmYwAPJzv5CZsnAzt8auVZRnG8X1sC3yRyvCb4s46HoPa7";

#[test]
fn test_review_history_multiple_edits_and_ordering() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let project_id = create_test_project(&client, &admin, "History-Project");
    let reviewer = Address::generate(&env);

    let cid1 = String::from_str(&env, CID_V1);
    client.submit_review(&project_id, &reviewer, &3, &cid1);

    env.ledger()
        .set_timestamp(env.ledger().timestamp().saturating_add(3601));
    let cid2 = String::from_str(&env, CID_V2);
    client.update_review(&project_id, &reviewer, &4, &Some(cid2.clone()));

    env.ledger()
        .set_timestamp(env.ledger().timestamp().saturating_add(3601));
    let cid3 = String::from_str(&env, CID_V3);
    client.update_review(&project_id, &reviewer, &5, &Some(cid3.clone()));

    assert_eq!(client.get_review_revision_count(&project_id, &reviewer), 2);

    let history = client.get_review_history(&project_id, &reviewer, &0, &10);
    assert_eq!(history.len(), 2);

    let first = history.get(0).unwrap();
    assert_eq!(first.revision_index, 0);
    assert_eq!(first.rating, 3);
    assert_eq!(first.content_cid, Some(String::from_str(&env, CID_V1)));

    let second = history.get(1).unwrap();
    assert_eq!(second.revision_index, 1);
    assert_eq!(second.rating, 4);
    assert_eq!(second.content_cid, Some(cid2));

    let latest = client.get_review(&project_id, &reviewer).unwrap();
    assert_eq!(latest.rating, 5);
    assert_eq!(latest.content_cid, Some(cid3));
}

#[test]
fn test_review_revision_event_emitted() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let project_id = create_test_project(&client, &admin, "Revision-Events");
    let reviewer = Address::generate(&env);

    let cid1 = String::from_str(&env, CID_V1);
    client.submit_review(&project_id, &reviewer, &2, &cid1);

    env.ledger()
        .set_timestamp(env.ledger().timestamp().saturating_add(3601));
    let cid2 = String::from_str(&env, CID_V2);
    client.update_review(&project_id, &reviewer, &4, &Some(cid2.clone()));

    let expected_topics = (
        symbol_short!("REVIEW"),
        symbol_short!("REVISED"),
        project_id,
        reviewer.clone(),
    )
        .into_val(&env);

    let has_revision_event = env.events().all().iter().any(|(_, topics, data)| {
        topics == expected_topics
            && TryIntoVal::<_, ReviewRevisionEvent>::try_into_val(&data, &env)
                .map(|event| {
                    event.revision_index == 0
                        && event.previous_rating == 2
                        && event.new_rating == 4
                        && event.previous_content_cid == Some(String::from_str(&env, CID_V1))
                        && event.new_content_cid == Some(cid2.clone())
                })
                .unwrap_or(false)
    });
    assert!(
        has_revision_event,
        "ReviewRevisionEvent must be emitted on edit"
    );
}

#[test]
fn test_weighted_rating_boundary_and_average_compatibility() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let project_id = create_test_project(&client, &admin, "Weighted-Rating");

    // Zero reviews → prior mean (3.50 = 350)
    assert_eq!(client.get_weighted_rating(&project_id), 350);
    let stats_empty = client.get_project_stats(&project_id);
    assert_eq!(stats_empty.average_rating, 0);

    let reviewer = Address::generate(&env);
    client.submit_review(&project_id, &reviewer, &5, &String::from_str(&env, CID_W1));

    let stats_one = client.get_project_stats(&project_id);
    assert_eq!(stats_one.average_rating, 500);
    assert_eq!(client.get_weighted_rating(&project_id), 375); // (5*350 + 500) / 6

    for cid in [CID_W2, CID_W3, CID_W4, CID_W5] {
        let r = Address::generate(&env);
        client.submit_review(&project_id, &r, &4, &String::from_str(&env, cid));
    }

    let stats_many = client.get_project_stats(&project_id);
    assert_eq!(stats_many.review_count, 5);
    assert_eq!(stats_many.average_rating, 420); // (5 + 4*4)/5 = 4.20
    let weighted = client.get_weighted_rating(&project_id);
    assert!(weighted >= 350 && weighted <= 500);
}

#[test]
fn test_weighted_rating_formula_validation() {
    use crate::rating_calculator::RatingCalculator;

    assert_eq!(RatingCalculator::calculate_weighted(0, 0), 350);
    assert_eq!(RatingCalculator::calculate_weighted(500, 1), 375);
    assert_eq!(RatingCalculator::calculate_weighted(2000, 4), 416);
    assert_eq!(RatingCalculator::calculate_average(2000, 4), 500);
    assert_eq!(RatingCalculator::calculate_weighted(2000, 4), 416);
}
