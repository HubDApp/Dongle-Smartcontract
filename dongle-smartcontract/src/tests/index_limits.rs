//! Storage index size limit tests: owner projects and review indexes.

use crate::constants::{MAX_PROJECTS_PER_USER, MAX_REVIEWS_PER_PROJECT, MAX_REVIEWS_PER_USER};
use crate::errors::ContractError;
use crate::tests::fixtures::{create_test_project, setup_contract};
use crate::types::ProjectRegistrationParams;
use soroban_sdk::{testutils::Address as _, Address, Env, String};

fn register_project_for_owner(
    env: &Env,
    client: &crate::DongleContractClient<'_>,
    owner: &Address,
    name: &str,
) -> u64 {
    extern crate alloc;
    use alloc::format;
    let slug = name.to_lowercase().replace(' ', "-");
    let params = ProjectRegistrationParams {
        owner: owner.clone(),
        name: String::from_str(env, name),
        slug: String::from_str(env, &slug),
        description: String::from_str(env, "Test project description"),
        category: String::from_str(env, "DeFi"),
        website: None,
        license: None,
        logo_cid: None,
        metadata_cid: None,
        tags: None,
        social_links: None,
        launch_timestamp: None,
        bounty_url: None,
        bounty_cid: None,
    };
    client.mock_all_auths().register_project(&params)
}

#[test]
fn test_max_projects_per_user_enforced() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    for i in 0..MAX_PROJECTS_PER_USER {
        extern crate alloc;
        use alloc::format;
        let name = format!("Project-", i);
        // ... rest of function (assume unchanged)
    }
}
