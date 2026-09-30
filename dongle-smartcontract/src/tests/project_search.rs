use crate::types::{ProjectRegistrationParams, VerificationStatus};
use crate::DongleContractClient;
use soroban_sdk::{testutils::Address as _, Address, Env, String, Vec};

use super::fixtures::{create_test_project, setup_contract};

fn register_project(
    client: &DongleContractClient<'_>,
    owner: &Address,
    name: &str,
    description: &str,
    category: &str,
    tags: Option<Vec<String>>,
) -> u64 {
    let env = &client.env;
    let slug = name.to_lowercase().replace(' ', "-");
    client.mock_all_auths().register_project(&ProjectRegistrationParams {
        owner: owner.clone(),
        name: String::from_str(env, name),
        slug: String::from_str(env, &slug),
        description: String::from_str(env, description),
        category: String::from_str(env, category),
        website: None,
        license: None,
        logo_cid: None,
        metadata_cid: None,
        tags,
        social_links: None,
        launch_timestamp: None,
        bounty_url: None,
        repository_url: None,
    })
}

#[test]
fn search_matches_all_requested_fields_case_insensitively() {
    let env = Env::default();
    let (client, _) = setup_contract(&env);
    let owner = Address::generate(&env);
    let mut tags = Vec::new(&env);
    tags.push_back(String::from_str(&env, "Orbital"));

    let name_id = register_project(&client, &owner, "Orbital Network", "Tools", "Finance", None);
    let description_id = register_project(
        &client,
        &owner,
        "Description Match",
        "Build ORBITAL applications",
        "Finance",
        None,
    );
    let category_id = register_project(
        &client,
        &owner,
        "Category Match",
        "Tools",
        "Orbital",
        None,
    );
    let tag_id = register_project(&client, &owner, "Tag Match", "Tools", "Finance", Some(tags));

    let results = client.search_projects(&String::from_str(&env, "oRbItAl"), &0, &100);
    assert_eq!(results.len(), 4);
    assert_eq!(results.get(0).unwrap().id, name_id);
    assert!(results.iter().any(|project| project.id == description_id));
    assert!(results.iter().any(|project| project.id == category_id));
    assert!(results.iter().any(|project| project.id == tag_id));
}

#[test]
fn search_ranks_relevance_rating_and_verification_then_paginates() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let exact_id = create_test_project(&client, &owner, "alpha");
    let description_id = register_project(
        &client,
        &owner,
        "Description Result",
        "alpha project",
        "Finance",
        None,
    );
    let rating_id = register_project(
        &client,
        &owner,
        "Rated Result",
        "alpha project",
        "Finance",
        None,
    );
    let reviewer = Address::generate(&env);
    client
        .mock_all_auths()
        .add_review(&rating_id, &reviewer, &5, &None);

    let verified_owner = Address::generate(&env);
    let verified_id = register_project(
        &client,
        &verified_owner,
        "Verified Result",
        "alpha project",
        "Finance",
        None,
    );
    let token_admin = Address::generate(&env);
    let token = env
        .register_stellar_asset_contract_v2(token_admin)
        .address();
    client.set_fee(&admin, &Some(token.clone()), &1, &0, &admin);
    soroban_sdk::token::StellarAssetClient::new(&env, &token).mint(&verified_owner, &1);
    client.pay_fee(&verified_owner, &verified_id, &Some(token.clone()));
    client.request_verification(
        &verified_id,
        &verified_owner,
        &String::from_str(&env, "QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG"),
    );
    client.approve_verification(&verified_id, &admin);

    let results = client.search_projects(&String::from_str(&env, "alpha"), &0, &2);
    assert_eq!(results.len(), 2);
    assert_eq!(results.get(0).unwrap().id, exact_id);
    assert_eq!(results.get(1).unwrap().id, verified_id);

    let next_page = client.search_projects(&String::from_str(&env, "alpha"), &2, &100);
    assert_eq!(next_page.len(), 2);
    assert_eq!(next_page.get(0).unwrap().id, rating_id);
    assert_eq!(next_page.get(1).unwrap().id, description_id);

    assert_eq!(
        client.get_project(&verified_id).unwrap().verification_status,
        VerificationStatus::Verified
    );
}

#[test]
fn empty_search_query_returns_no_results() {
    let env = Env::default();
    let (client, _) = setup_contract(&env);
    let owner = Address::generate(&env);
    create_test_project(&client, &owner, "Any Project");

    assert!(client
        .search_projects(&String::from_str(&env, ""), &0, &100)
        .is_empty());
}