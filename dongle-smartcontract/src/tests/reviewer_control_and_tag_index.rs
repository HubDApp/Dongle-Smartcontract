//! Reviewer-control checks (issue #478) and the inverted tag index (issue #483).

use crate::errors::ContractError;
use crate::tests::fixtures::{create_test_project, setup_contract};
use crate::types::{ProjectRegistrationParams, ProjectUpdateParams};
use crate::DongleContractClient;
use soroban_sdk::{testutils::Address as _, Address, Env, String, Vec};

fn setup(env: &Env) -> (DongleContractClient<'_>, Address) {
    setup_contract(env)
}

/// Register a project carrying `tags`.
fn create_tagged_project(
    client: &DongleContractClient<'_>,
    owner: &Address,
    name: &str,
    slug: &str,
    tags: &[&str],
) -> u64 {
    let env = &client.env;
    let mut tag_vec = Vec::new(env);
    for tag in tags {
        tag_vec.push_back(String::from_str(env, tag));
    }

    let params = ProjectRegistrationParams {
        owner: owner.clone(),
        name: String::from_str(env, name),
        slug: String::from_str(env, slug),
        description: String::from_str(env, "Test project description"),
        category: String::from_str(env, "DeFi"),
        website: None,
        license: None,
        logo_cid: None,
        metadata_cid: None,
        tags: Some(tag_vec),
        social_links: None,
        launch_timestamp: None,
        bounty_url: None,
    };
    client.mock_all_auths().register_project(&params)
}

fn tags_of(env: &Env, tags: &[&str]) -> Vec<String> {
    let mut v = Vec::new(env);
    for t in tags {
        v.push_back(String::from_str(env, t));
    }
    v
}

// ---------------------------------------------------------------------------
// Issue #478 — nobody who controls a project may review it
// ---------------------------------------------------------------------------

#[test]
fn owner_cannot_review_own_project() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "OwnerReview");
    let _ = admin;

    let result = client.try_add_review(&project_id, &owner, &5, &None);
    assert_eq!(result, Err(Ok(ContractError::OwnerCannotReview)));
}

#[test]
fn maintainer_cannot_review_the_project_they_maintain() {
    // The hole this closes: a maintainer is appointed by the owner and can edit
    // the project, so letting them review is the same rating inflation as
    // letting the owner do it.
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup(&env);
    let owner = Address::generate(&env);
    let maintainer = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "MaintainerReview");

    client
        .mock_all_auths()
        .add_maintainer(&project_id, &owner, &maintainer);

    let result = client.try_add_review(&project_id, &maintainer, &5, &None);
    assert_eq!(result, Err(Ok(ContractError::OwnerCannotReview)));
}

#[test]
fn an_unrelated_reviewer_is_still_allowed() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup(&env);
    let owner = Address::generate(&env);
    let reviewer = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "NormalReview");

    client.add_review(&project_id, &reviewer, &4, &None);

    let review = client.get_review(&project_id, &reviewer).unwrap();
    assert_eq!(review.rating, 4);
}

#[test]
fn a_reviewer_promoted_to_maintainer_cannot_keep_editing_their_review() {
    // Control can be acquired *after* an honest review. Without the check on the
    // update path, that reviewer keeps a rating they can now edit from inside.
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup(&env);
    let owner = Address::generate(&env);
    let reviewer = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "PromotedReviewer");

    client.add_review(&project_id, &reviewer, &2, &None);

    client
        .mock_all_auths()
        .add_maintainer(&project_id, &owner, &reviewer);

    let result = client.try_update_review(&project_id, &reviewer, &5, &None);
    assert_eq!(result, Err(Ok(ContractError::OwnerCannotReview)));

    // The original rating stands.
    assert_eq!(client.get_review(&project_id, &reviewer).unwrap().rating, 2);
}

#[test]
fn removing_a_maintainer_restores_their_ability_to_review() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup(&env);
    let owner = Address::generate(&env);
    let maintainer = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "RemovedMaintainer");

    client
        .mock_all_auths()
        .add_maintainer(&project_id, &owner, &maintainer);
    assert_eq!(
        client.try_add_review(&project_id, &maintainer, &5, &None),
        Err(Ok(ContractError::OwnerCannotReview))
    );

    client
        .mock_all_auths()
        .remove_maintainer(&project_id, &owner, &maintainer);

    client.add_review(&project_id, &maintainer, &5, &None);
    assert_eq!(
        client.get_review(&project_id, &maintainer).unwrap().rating,
        5
    );
}

#[test]
fn a_maintainer_of_one_project_may_review_another() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup(&env);
    let owner = Address::generate(&env);
    let maintainer = Address::generate(&env);
    let mine = create_test_project(&client, &owner, "MineProject");
    let other = create_test_project(&client, &owner, "OtherProject");

    client
        .mock_all_auths()
        .add_maintainer(&mine, &owner, &maintainer);

    // Blocked on the project they maintain, allowed on the one they do not.
    assert_eq!(
        client.try_add_review(&mine, &maintainer, &5, &None),
        Err(Ok(ContractError::OwnerCannotReview))
    );
    client.add_review(&other, &maintainer, &5, &None);
    assert_eq!(client.get_review(&other, &maintainer).unwrap().rating, 5);
}

// ---------------------------------------------------------------------------
// Issue #483 — inverted tag index
// ---------------------------------------------------------------------------

#[test]
fn tag_lookup_returns_only_projects_carrying_the_tag() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup(&env);
    let owner = Address::generate(&env);

    let defi = create_tagged_project(&client, &owner, "DefiOne", "defi-one", &["defi", "amm"]);
    let _nft = create_tagged_project(&client, &owner, "NftOne", "nft-one", &["nft"]);
    let defi2 = create_tagged_project(&client, &owner, "DefiTwo", "defi-two", &["defi"]);

    let found = client.get_projects_by_tag_batch(&tags_of(&env, &["defi"]), &50);

    assert_eq!(found.len(), 2);
    let ids: Vec<u64> = {
        let mut v = Vec::new(&env);
        for p in found.iter() {
            v.push_back(p.id);
        }
        v
    };
    assert!(ids.contains(&defi));
    assert!(ids.contains(&defi2));
}

#[test]
fn batch_lookup_covers_several_tags_in_one_call() {
    // The round-trip reduction the issue asks for.
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup(&env);
    let owner = Address::generate(&env);

    create_tagged_project(&client, &owner, "AlphaP", "alpha-p", &["defi"]);
    create_tagged_project(&client, &owner, "BetaP", "beta-p", &["nft"]);
    create_tagged_project(&client, &owner, "GammaP", "gamma-p", &["dao"]);

    let found = client.get_projects_by_tag_batch(&tags_of(&env, &["defi", "nft"]), &50);
    assert_eq!(found.len(), 2);
}

#[test]
fn a_project_matching_two_requested_tags_is_returned_once() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup(&env);
    let owner = Address::generate(&env);

    create_tagged_project(&client, &owner, "BothP", "both-p", &["defi", "nft"]);

    let found = client.get_projects_by_tag_batch(&tags_of(&env, &["defi", "nft"]), &50);
    assert_eq!(found.len(), 1);
}

#[test]
fn an_unknown_tag_returns_nothing() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup(&env);
    let owner = Address::generate(&env);
    create_tagged_project(&client, &owner, "SomeP", "some-p", &["defi"]);

    let found = client.get_projects_by_tag_batch(&tags_of(&env, &["nosuchtag"]), &50);
    assert_eq!(found.len(), 0);
}

#[test]
fn retagging_a_project_moves_it_between_index_entries() {
    // Without unindexing, the old tag keeps pointing at the project forever.
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup(&env);
    let owner = Address::generate(&env);
    let project_id = create_tagged_project(&client, &owner, "MoveP", "move-p", &["defi"]);

    let mut new_tags = Vec::new(&env);
    new_tags.push_back(String::from_str(&env, "nft"));

    client
        .mock_all_auths()
        .update_project(&ProjectUpdateParams {
            project_id,
            caller: owner.clone(),
            name: None,
            slug: None,
            description: None,
            category: None,
            website: None,
            license: None,
            logo_cid: None,
            metadata_cid: None,
            tags: Some(Some(new_tags)),
            social_links: None,
            launch_timestamp: None,
            bounty_url: None,
        });

    assert_eq!(
        client
            .get_projects_by_tag_batch(&tags_of(&env, &["defi"]), &50)
            .len(),
        0,
        "the project must no longer be indexed under its old tag"
    );
    assert_eq!(
        client
            .get_projects_by_tag_batch(&tags_of(&env, &["nft"]), &50)
            .len(),
        1
    );
}

#[test]
fn the_watermark_tracks_registrations() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup(&env);
    let owner = Address::generate(&env);

    assert_eq!(client.get_tag_index_watermark(), 0);
    create_tagged_project(&client, &owner, "W1", "w-1", &["defi"]);
    assert_eq!(client.get_tag_index_watermark(), 1);
    create_tagged_project(&client, &owner, "W2", "w-2", &["defi"]);
    assert_eq!(client.get_tag_index_watermark(), 2);
}

#[test]
fn results_agree_with_the_scanning_implementation() {
    // The index must not change *what* is returned, only how fast.
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup(&env);
    let owner = Address::generate(&env);

    create_tagged_project(&client, &owner, "CmpA", "cmp-a", &["defi", "amm"]);
    create_tagged_project(&client, &owner, "CmpB", "cmp-b", &["nft"]);
    create_tagged_project(&client, &owner, "CmpC", "cmp-c", &["defi"]);

    let scanned = client.list_projects_by_tag(&String::from_str(&env, "defi"), &0, &50);
    let indexed = client.get_projects_by_tag_batch(&tags_of(&env, &["defi"]), &50);

    assert_eq!(scanned.len(), indexed.len());
}

#[test]
fn the_limit_is_respected() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup(&env);
    let owner = Address::generate(&env);

    create_tagged_project(&client, &owner, "LimA", "lim-a", &["defi"]);
    create_tagged_project(&client, &owner, "LimB", "lim-b", &["defi"]);
    create_tagged_project(&client, &owner, "LimC", "lim-c", &["defi"]);

    assert_eq!(
        client
            .get_projects_by_tag_batch(&tags_of(&env, &["defi"]), &2)
            .len(),
        2
    );
}

#[test]
fn reindex_is_admin_only_and_advances_the_watermark() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let owner = Address::generate(&env);
    create_tagged_project(&client, &owner, "RX1", "rx-1", &["defi"]);

    let stranger = Address::generate(&env);
    assert!(client.try_reindex_tags(&stranger, &10).is_err());

    let watermark = client.mock_all_auths().reindex_tags(&admin, &10);
    assert!(watermark >= 1);
}

// ---------------------------------------------------------------------------
// Issue #460 — the dead `src/validation.rs` module is gone
//
// That module referenced `params.bounty_cid`, a field `ProjectRegistrationParams`
// has never had. It was never declared in `lib.rs`, so it never compiled and
// never ran; the real validation lives in `register_project` via `Utils`.
// These pin that live behaviour, so deleting the dead file provably removed no
// coverage.
// ---------------------------------------------------------------------------

fn register_with_bounty_url(
    client: &DongleContractClient<'_>,
    owner: &Address,
    name: &str,
    slug: &str,
    bounty_url: Option<&str>,
) -> Result<u64, soroban_sdk::Val> {
    let env = &client.env;
    let params = ProjectRegistrationParams {
        owner: owner.clone(),
        name: String::from_str(env, name),
        slug: String::from_str(env, slug),
        description: String::from_str(env, "Test project description"),
        category: String::from_str(env, "DeFi"),
        website: None,
        license: None,
        logo_cid: None,
        metadata_cid: None,
        tags: None,
        social_links: None,
        launch_timestamp: None,
        bounty_url: bounty_url.map(|u| String::from_str(env, u)),
    };
    match client.mock_all_auths().try_register_project(&params) {
        Ok(Ok(id)) => Ok(id),
        Ok(Err(_)) => Err(soroban_sdk::Val::from_void().into()),
        Err(_) => Err(soroban_sdk::Val::from_void().into()),
    }
}

#[test]
fn a_valid_bounty_url_is_accepted() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup(&env);
    let owner = Address::generate(&env);

    let id = register_with_bounty_url(
        &client,
        &owner,
        "BountyOk",
        "bounty-ok",
        Some("https://example.com/security"),
    );
    assert!(id.is_ok());
}

#[test]
fn a_malformed_bounty_url_is_still_rejected_without_the_dead_module() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup(&env);
    let owner = Address::generate(&env);

    let result = register_with_bounty_url(
        &client,
        &owner,
        "BountyBad",
        "bounty-bad",
        Some("not-a-url"),
    );
    assert!(
        result.is_err(),
        "bounty_url validation must still run after removing src/validation.rs"
    );
}
