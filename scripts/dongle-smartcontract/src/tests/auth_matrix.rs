//! Role × Function authorization matrix — Issue #215
//!
//! For every mutating function this file proves:
//!   - the correct role(s) succeed, and
//!   - every other role is rejected without mutating state.
//!
//! Roles: admin | owner | maintainer | reviewer | stranger (unrelated user)

#![cfg(test)]

use crate::errors::ContractError;
use crate::tests::fixtures::create_test_project;
use crate::types::{DependencyRef, DisputeResolutionAction, ProjectDependency, ProjectUpdateParams};
use crate::DongleContract;
use crate::DongleContractClient;
use soroban_sdk::{testutils::Address as _, Address, Env, String};

// ── helpers ───────────────────────────────────────────────────────────────────

fn init(env: &Env) -> (DongleContractClient<'_>, Address) {
    let id = env.register(DongleContract, ());
    let client = DongleContractClient::new(env, &id);
    let admin = Address::generate(env);
    client.mock_all_auths().initialize(&admin);
    (client, admin)
}

fn update_params(env: &Env, project_id: u64, caller: &Address) -> ProjectUpdateParams {
    ProjectUpdateParams {
        project_id,
        caller: caller.clone(),
        name: None,
        slug: None,
        description: Some(String::from_str(env, "changed")),
        category: None,
        website: None,
        logo_cid: None,
        metadata_cid: None,
        tags: None,
        social_links: None,
        launch_timestamp: None,
        bounty_url: None,
    }
}

fn make_dep(env: &Env) -> ProjectDependency {
    ProjectDependency {
        reference: DependencyRef {
            project_id: None,
            external_cid: None,
            external_url: Some(String::from_str(env, "https://example.com")),
            external_contract: None,
        },
        label: None,
        metadata_cid: None,
        added_at: 0,
        updated_at: 0,
    }
}

// ── admin management ──────────────────────────────────────────────────────────

// add_admin: admin ✓ | stranger ✗
#[test]
fn matrix_add_admin_admin_succeeds() {
    let env = Env::default();
    let (client, admin) = init(&env);
    let new_admin = Address::generate(&env);
    client.mock_all_auths().add_admin(&admin, &new_admin);
    assert!(client.is_admin(&new_admin));
}

#[test]
fn matrix_add_admin_stranger_fails_no_mutation() {
    let env = Env::default();
    let (client, _admin) = init(&env);
    let stranger = Address::generate(&env);
    let target = Address::generate(&env);
    let err = client.mock_all_auths().try_add_admin(&stranger, &target);
    assert_eq!(err, Err(Ok(ContractError::AdminOnly)));
    assert!(!client.is_admin(&target));
}

// remove_admin: admin ✓ | stranger ✗
#[test]
fn matrix_remove_admin_admin_succeeds() {
    let env = Env::default();
    let (client, admin) = init(&env);
    let second = Address::generate(&env);
    client.mock_all_auths().add_admin(&admin, &second);
    client.mock_all_auths().remove_admin(&admin, &second);
    assert!(!client.is_admin(&second));
}

#[test]
fn matrix_remove_admin_stranger_fails_no_mutation() {
    let env = Env::default();
    let (client, admin) = init(&env);
    let stranger = Address::generate(&env);
    let err = client.mock_all_auths().try_remove_admin(&stranger, &admin);
    assert_eq!(err, Err(Ok(ContractError::AdminOnly)));
    assert!(client.is_admin(&admin));
}

// ── update_project: owner ✓ | maintainer ✓ | admin ✗ | reviewer ✗ | stranger ✗ ──

#[test]
fn matrix_update_project_owner_succeeds() {
    let env = Env::default();
    let (client, _admin) = init(&env);
    let owner = Address::generate(&env);
    let pid = create_test_project(&client, &owner, "Proj");
    let p = client.mock_all_auths().update_project(&update_params(&env, pid, &owner));
    assert_eq!(p.description, String::from_str(&env, "changed"));
}

#[test]
fn matrix_update_project_maintainer_succeeds() {
    let env = Env::default();
    let (client, _admin) = init(&env);
    let owner = Address::generate(&env);
    let maintainer = Address::generate(&env);
    let pid = create_test_project(&client, &owner, "Proj");
    client.mock_all_auths().add_maintainer(&pid, &owner, &maintainer);
    let p = client.mock_all_auths().update_project(&update_params(&env, pid, &maintainer));
    assert_eq!(p.description, String::from_str(&env, "changed"));
}

#[test]
fn matrix_update_project_admin_fails_no_mutation() {
    let env = Env::default();
    let (client, admin) = init(&env);
    let owner = Address::generate(&env);
    let pid = create_test_project(&client, &owner, "Proj");
    let before = client.get_project(&pid).unwrap();
    let err = client.mock_all_auths().try_update_project(&update_params(&env, pid, &admin));
    assert_eq!(err, Err(Ok(ContractError::Unauthorized)));
    assert_eq!(client.get_project(&pid).unwrap().description, before.description);
}

#[test]
fn matrix_update_project_reviewer_fails_no_mutation() {
    let env = Env::default();
    let (client, _admin) = init(&env);
    let owner = Address::generate(&env);
    let reviewer = Address::generate(&env);
    let pid = create_test_project(&client, &owner, "Proj");
    client.mock_all_auths().add_review(&pid, &reviewer, &4, &None);
    let before = client.get_project(&pid).unwrap();
    let err = client.mock_all_auths().try_update_project(&update_params(&env, pid, &reviewer));
    assert_eq!(err, Err(Ok(ContractError::Unauthorized)));
    assert_eq!(client.get_project(&pid).unwrap().description, before.description);
}

#[test]
fn matrix_update_project_stranger_fails_no_mutation() {
    let env = Env::default();
    let (client, _admin) = init(&env);
    let owner = Address::generate(&env);
    let stranger = Address::generate(&env);
    let pid = create_test_project(&client, &owner, "Proj");
    let before = client.get_project(&pid).unwrap();
    let err = client.mock_all_auths().try_update_project(&update_params(&env, pid, &stranger));
    assert_eq!(err, Err(Ok(ContractError::Unauthorized)));
    assert_eq!(client.get_project(&pid).unwrap().description, before.description);
}

// ── archive_project / reactivate_project: owner ✓ | stranger ✗ ──────────────

#[test]
fn matrix_archive_owner_succeeds_stranger_fails() {
    let env = Env::default();
    let (client, _admin) = init(&env);
    let owner = Address::generate(&env);
    let stranger = Address::generate(&env);
    let pid = create_test_project(&client, &owner, "Arch");

    // stranger cannot archive
    let err = client.mock_all_auths().try_archive_project(&pid, &stranger);
    assert_eq!(err, Err(Ok(ContractError::Unauthorized)));
    assert!(!client.get_project(&pid).unwrap().archived);

    // owner can archive
    client.mock_all_auths().archive_project(&pid, &owner);
    assert!(client.get_project(&pid).unwrap().archived);
}

#[test]
fn matrix_reactivate_owner_succeeds_stranger_fails() {
    let env = Env::default();
    let (client, _admin) = init(&env);
    let owner = Address::generate(&env);
    let stranger = Address::generate(&env);
    let pid = create_test_project(&client, &owner, "React");
    client.mock_all_auths().archive_project(&pid, &owner);

    // stranger cannot reactivate
    let err = client.mock_all_auths().try_reactivate_project(&pid, &stranger);
    assert_eq!(err, Err(Ok(ContractError::Unauthorized)));
    assert!(client.get_project(&pid).unwrap().archived);

    // owner can reactivate
    client.mock_all_auths().reactivate_project(&pid, &owner);
    assert!(!client.get_project(&pid).unwrap().archived);
}

// ── add_maintainer / remove_maintainer: owner ✓ | maintainer ✗ | stranger ✗ ─

#[test]
fn matrix_add_maintainer_owner_succeeds_others_fail() {
    let env = Env::default();
    let (client, _admin) = init(&env);
    let owner = Address::generate(&env);
    let maintainer = Address::generate(&env);
    let stranger = Address::generate(&env);
    let new_m = Address::generate(&env);
    let pid = create_test_project(&client, &owner, "Maint");
    client.mock_all_auths().add_maintainer(&pid, &owner, &maintainer);

    // maintainer cannot add another maintainer
    let err = client.mock_all_auths().try_add_maintainer(&pid, &maintainer, &new_m);
    assert_eq!(err, Err(Ok(ContractError::Unauthorized)));

    // stranger cannot add a maintainer
    let err = client.mock_all_auths().try_add_maintainer(&pid, &stranger, &new_m);
    assert_eq!(err, Err(Ok(ContractError::Unauthorized)));

    // owner can add
    client.mock_all_auths().add_maintainer(&pid, &owner, &new_m);
    assert_eq!(client.get_maintainers(&pid).len(), 2);
}

#[test]
fn matrix_remove_maintainer_owner_succeeds_stranger_fails() {
    let env = Env::default();
    let (client, _admin) = init(&env);
    let owner = Address::generate(&env);
    let maintainer = Address::generate(&env);
    let stranger = Address::generate(&env);
    let pid = create_test_project(&client, &owner, "RemMaint");
    client.mock_all_auths().add_maintainer(&pid, &owner, &maintainer);

    // stranger cannot remove
    let err = client.mock_all_auths().try_remove_maintainer(&pid, &stranger, &maintainer);
    assert_eq!(err, Err(Ok(ContractError::Unauthorized)));
    assert_eq!(client.get_maintainers(&pid).len(), 1);

    // owner can remove
    client.mock_all_auths().remove_maintainer(&pid, &owner, &maintainer);
    assert_eq!(client.get_maintainers(&pid).len(), 0);
}

// ── link_project / unlink_project: owner ✓ | stranger ✗ ─────────────────────

#[test]
fn matrix_link_project_owner_succeeds_stranger_fails() {
    let env = Env::default();
    let (client, _admin) = init(&env);
    let owner = Address::generate(&env);
    let stranger = Address::generate(&env);
    let pid1 = create_test_project(&client, &owner, "Link1");
    let pid2 = create_test_project(&client, &owner, "Link2");

    let err = client.mock_all_auths().try_link_project(&pid1, &stranger, &pid2);
    assert_eq!(err, Err(Ok(ContractError::Unauthorized)));
    assert_eq!(client.get_linked_projects(&pid1).len(), 0);

    client.mock_all_auths().link_project(&pid1, &owner, &pid2);
    assert_eq!(client.get_linked_projects(&pid1).len(), 1);
}

#[test]
fn matrix_unlink_project_owner_succeeds_stranger_fails() {
    let env = Env::default();
    let (client, _admin) = init(&env);
    let owner = Address::generate(&env);
    let stranger = Address::generate(&env);
    let pid1 = create_test_project(&client, &owner, "ULink1");
    let pid2 = create_test_project(&client, &owner, "ULink2");
    client.mock_all_auths().link_project(&pid1, &owner, &pid2);

    let err = client.mock_all_auths().try_unlink_project(&pid1, &stranger, &pid2);
    assert_eq!(err, Err(Ok(ContractError::Unauthorized)));
    assert_eq!(client.get_linked_projects(&pid1).len(), 1);

    client.mock_all_auths().unlink_project(&pid1, &owner, &pid2);
    assert_eq!(client.get_linked_projects(&pid1).len(), 0);
}

// ── initiate_transfer: owner ✓ | maintainer ✗ | stranger ✗ ──────────────────

#[test]
fn matrix_initiate_transfer_owner_succeeds_others_fail() {
    let env = Env::default();
    let (client, _admin) = init(&env);
    let owner = Address::generate(&env);
    let maintainer = Address::generate(&env);
    let stranger = Address::generate(&env);
    let new_owner = Address::generate(&env);
    let pid = create_test_project(&client, &owner, "Transfer");
    client.mock_all_auths().add_maintainer(&pid, &owner, &maintainer);

    let err = client.mock_all_auths().try_initiate_transfer(&pid, &maintainer, &new_owner);
    assert_eq!(err, Err(Ok(ContractError::Unauthorized)));

    let err = client.mock_all_auths().try_initiate_transfer(&pid, &stranger, &new_owner);
    assert_eq!(err, Err(Ok(ContractError::Unauthorized)));
    assert_eq!(client.get_project(&pid).unwrap().owner, owner);

    client.mock_all_auths().initiate_transfer(&pid, &owner, &new_owner);
    // owner unchanged until accept; no error means it was accepted by contract logic
}

// ── review registry ───────────────────────────────────────────────────────────
// update_review: reviewer (own) ✓ | stranger ✗
// delete_review: reviewer (own) ✓ | stranger ✗
// respond_to_review: owner ✓ | stranger ✗
// hide_review / restore_review: admin ✓ | stranger ✗
// admin_delete_review: admin ✓ | stranger ✗

#[test]
fn matrix_update_review_reviewer_succeeds_stranger_fails() {
    let env = Env::default();
    let (client, _admin) = init(&env);
    let owner = Address::generate(&env);
    let reviewer = Address::generate(&env);
    let stranger = Address::generate(&env);
    let pid = create_test_project(&client, &owner, "RevUpd");
    client.mock_all_auths().add_review(&pid, &reviewer, &3, &None);

    // stranger cannot update reviewer's review
    let err = client.mock_all_auths().try_update_review(&pid, &stranger, &5, &None);
    assert_eq!(err, Err(Ok(ContractError::ReviewNotFound)));
    assert_eq!(client.get_review(&pid, &reviewer).unwrap().rating, 3);

    // reviewer can update own review
    client.mock_all_auths().update_review(&pid, &reviewer, &5, &None);
    assert_eq!(client.get_review(&pid, &reviewer).unwrap().rating, 5);
}

#[test]
fn matrix_delete_review_reviewer_succeeds_stranger_fails() {
    let env = Env::default();
    let (client, _admin) = init(&env);
    let owner = Address::generate(&env);
    let reviewer = Address::generate(&env);
    let stranger = Address::generate(&env);
    let pid = create_test_project(&client, &owner, "RevDel");
    client.mock_all_auths().add_review(&pid, &reviewer, &3, &None);

    let err = client.mock_all_auths().try_delete_review(&pid, &stranger);
    assert_eq!(err, Err(Ok(ContractError::ReviewNotFound)));
    assert!(client.get_review(&pid, &reviewer).is_some());

    client.mock_all_auths().delete_review(&pid, &reviewer);
    assert!(client.get_review(&pid, &reviewer).is_none());
}

#[test]
fn matrix_respond_to_review_owner_succeeds_stranger_fails() {
    let env = Env::default();
    let (client, _admin) = init(&env);
    let owner = Address::generate(&env);
    let reviewer = Address::generate(&env);
    let stranger = Address::generate(&env);
    let pid = create_test_project(&client, &owner, "RevResp");
    client.mock_all_auths().add_review(&pid, &reviewer, &4, &None);

    let err = client.mock_all_auths().try_respond_to_review(
        &pid, &stranger, &reviewer, &String::from_str(&env, "thanks"),
    );
    assert_eq!(err, Err(Ok(ContractError::Unauthorized)));
    assert!(client.get_review_response(&pid, &reviewer).is_none());

    client.mock_all_auths().respond_to_review(
        &pid, &owner, &reviewer, &String::from_str(&env, "thanks"),
    );
    assert!(client.get_review_response(&pid, &reviewer).is_some());
}

#[test]
fn matrix_hide_restore_review_admin_succeeds_stranger_fails() {
    let env = Env::default();
    let (client, admin) = init(&env);
    let owner = Address::generate(&env);
    let reviewer = Address::generate(&env);
    let stranger = Address::generate(&env);
    let pid = create_test_project(&client, &owner, "RevHide");
    client.mock_all_auths().add_review(&pid, &reviewer, &4, &None);

    // stranger cannot hide
    let err = client.mock_all_auths().try_hide_review(&pid, &reviewer, &stranger);
    assert_eq!(err, Err(Ok(ContractError::AdminOnly)));
    assert!(!client.get_review(&pid, &reviewer).unwrap().hidden);

    // admin can hide
    client.mock_all_auths().hide_review(&pid, &reviewer, &admin);
    assert!(client.get_review(&pid, &reviewer).unwrap().hidden);

    // stranger cannot restore
    let err = client.mock_all_auths().try_restore_review(&pid, &reviewer, &stranger);
    assert_eq!(err, Err(Ok(ContractError::AdminOnly)));
    assert!(client.get_review(&pid, &reviewer).unwrap().hidden);

    // admin can restore
    client.mock_all_auths().restore_review(&pid, &reviewer, &admin);
    assert!(!client.get_review(&pid, &reviewer).unwrap().hidden);
}

#[test]
fn matrix_admin_delete_review_admin_succeeds_stranger_fails() {
    let env = Env::default();
    let (client, admin) = init(&env);
    let owner = Address::generate(&env);
    let reviewer = Address::generate(&env);
    let stranger = Address::generate(&env);
    let pid = create_test_project(&client, &owner, "RevAdmDel");
    client.mock_all_auths().add_review(&pid, &reviewer, &4, &None);

    let err = client.mock_all_auths().try_admin_delete_review(&pid, &reviewer, &stranger);
    assert_eq!(err, Err(Ok(ContractError::AdminOnly)));
    assert!(client.get_review(&pid, &reviewer).is_some());

    client.mock_all_auths().admin_delete_review(&pid, &reviewer, &admin);
    assert!(client.get_review(&pid, &reviewer).is_none());
}

// ── verification registry ─────────────────────────────────────────────────────
// request_verification: owner ✓ | stranger ✗
// approve/reject/revoke_verification: admin ✓ | stranger ✗

#[test]
fn matrix_request_verification_owner_succeeds_stranger_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = init(&env);
    let owner = Address::generate(&env);
    let stranger = Address::generate(&env);
    let pid = create_test_project(&client, &owner, "VerReq");

    let err = client.try_request_verification(&pid, &stranger, &String::from_str(&env, "QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG"));
    assert_eq!(err, Err(Ok(ContractError::Unauthorized)));

    client.request_verification(&pid, &owner, &String::from_str(&env, "QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG"));
    assert_eq!(client.get_verification(&pid).status, crate::types::VerificationStatus::Pending);
}

#[test]
fn matrix_approve_verification_admin_succeeds_stranger_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = init(&env);
    let owner = Address::generate(&env);
    let stranger = Address::generate(&env);
    let pid = create_test_project(&client, &owner, "VerApprove");
    client.request_verification(&pid, &owner, &String::from_str(&env, "QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG"));

    let err = client.try_approve_verification(&pid, &stranger);
    assert_eq!(err, Err(Ok(ContractError::AdminOnly)));

    client.approve_verification(&pid, &admin);
    use crate::types::VerificationStatus;
    assert_eq!(
        client.get_verification(&pid).status,
        VerificationStatus::Verified
    );
}

#[test]
fn matrix_reject_verification_admin_succeeds_stranger_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = init(&env);
    let owner = Address::generate(&env);
    let stranger = Address::generate(&env);
    let pid = create_test_project(&client, &owner, "VerReject");
    client.request_verification(&pid, &owner, &String::from_str(&env, "QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG"));

    let err = client.try_reject_verification(&pid, &stranger);
    assert_eq!(err, Err(Ok(ContractError::AdminOnly)));

    client.reject_verification(&pid, &admin);
    use crate::types::VerificationStatus;
    assert_eq!(
        client.get_verification(&pid).status,
        VerificationStatus::Rejected
    );
}

#[test]
fn matrix_revoke_verification_admin_succeeds_stranger_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = init(&env);
    let owner = Address::generate(&env);
    let stranger = Address::generate(&env);
    let pid = create_test_project(&client, &owner, "VerRevoke");
    client.request_verification(&pid, &owner, &String::from_str(&env, "QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG"));
    client.approve_verification(&pid, &admin);

    let reason = String::from_str(&env, "fraud");
    let err = client.try_revoke_verification(&pid, &stranger, &reason);
    assert_eq!(err, Err(Ok(ContractError::AdminOnly)));

    client.revoke_verification(&pid, &admin, &reason);
    use crate::types::VerificationStatus;
    assert_eq!(
        client.get_verification(&pid).status,
        VerificationStatus::Unverified
    );
}

// ── set_fee: admin ✓ | stranger ✗ ────────────────────────────────────────────

#[test]
fn matrix_set_fee_admin_succeeds_stranger_fails() {
    let env = Env::default();
    let (client, admin) = init(&env);
    let stranger = Address::generate(&env);
    let treasury = Address::generate(&env);

    let err = client.mock_all_auths().try_set_fee(&stranger, &None, &100u128, &0u128, &treasury);
    assert_eq!(err, Err(Ok(ContractError::AdminOnly)));

    client.mock_all_auths().set_fee(&admin, &None, &100u128, &0u128, &treasury);
    assert_eq!(client.get_fee_config().verification_fee, 100u128);
}

// ── set_featured: admin ✓ | stranger ✗ ───────────────────────────────────────

#[test]
fn matrix_set_featured_admin_succeeds_stranger_fails() {
    let env = Env::default();
    let (client, admin) = init(&env);
    let owner = Address::generate(&env);
    let stranger = Address::generate(&env);
    let pid = create_test_project(&client, &owner, "FeatProj");

    let err = client.mock_all_auths().try_set_featured(&stranger, &pid, &true);
    assert_eq!(err, Err(Ok(ContractError::AdminOnly)));
    assert_eq!(client.list_featured_projects(&0u32, &10u32).len(), 0);

    client.mock_all_auths().set_featured(&admin, &pid, &true);
    assert_eq!(client.list_featured_projects(&0u32, &10u32).len(), 1);
}

// ── collection management: admin ✓ | stranger ✗ ──────────────────────────────

#[test]
fn matrix_create_collection_admin_succeeds_stranger_fails() {
    let env = Env::default();
    let (client, admin) = init(&env);
    let stranger = Address::generate(&env);
    let n = String::from_str(&env, "Col");
    let d = String::from_str(&env, "Desc");

    let err = client.mock_all_auths().try_create_collection(&stranger, &n, &d);
    assert_eq!(err, Err(Ok(ContractError::AdminOnly)));

    client.mock_all_auths().create_collection(&admin, &n, &d);
    assert_eq!(client.get_collection_count(), 1u64);
}

#[test]
fn matrix_collection_mutations_admin_succeeds_stranger_fails() {
    let env = Env::default();
    let (client, admin) = init(&env);
    let owner = Address::generate(&env);
    let stranger = Address::generate(&env);
    let pid = create_test_project(&client, &owner, "ColProj");
    let n = String::from_str(&env, "Col");
    let d = String::from_str(&env, "Desc");
    let cid = client.mock_all_auths().create_collection(&admin, &n, &d);

    // add_project_to_collection
    let err = client.mock_all_auths().try_add_project_to_collection(&stranger, &cid, &pid);
    assert_eq!(err, Err(Ok(ContractError::AdminOnly)));

    client.mock_all_auths().add_project_to_collection(&admin, &cid, &pid);
    assert_eq!(client.get_collection_project_count(&cid), 1u32);

    // remove_project_from_collection
    let err = client.mock_all_auths().try_remove_project_from_collection(&stranger, &cid, &pid);
    assert_eq!(err, Err(Ok(ContractError::AdminOnly)));
    assert_eq!(client.get_collection_project_count(&cid), 1u32);

    client.mock_all_auths().remove_project_from_collection(&admin, &cid, &pid);
    assert_eq!(client.get_collection_project_count(&cid), 0u32);
}

// ── project dependencies: owner/maintainer ✓ | stranger ✗ ───────────────────

#[test]
fn matrix_add_dependency_owner_succeeds_stranger_fails() {
    let env = Env::default();
    let (client, _admin) = init(&env);
    let owner = Address::generate(&env);
    let stranger = Address::generate(&env);
    let pid = create_test_project(&client, &owner, "DepProj");
    let dep = make_dep(&env);

    let err = client.mock_all_auths().try_add_project_dependency(&pid, &stranger, &dep);
    assert_eq!(err, Err(Ok(ContractError::Unauthorized)));
    assert_eq!(client.get_project_dependencies(&pid).len(), 0);

    client.mock_all_auths().add_project_dependency(&pid, &owner, &dep);
    assert_eq!(client.get_project_dependencies(&pid).len(), 1);
}

// ── set_project_claimable: owner ✓ | stranger ✗ ──────────────────────────────

#[test]
fn matrix_set_claimable_owner_succeeds_stranger_fails() {
    let env = Env::default();
    let (client, _admin) = init(&env);
    let owner = Address::generate(&env);
    let stranger = Address::generate(&env);
    let pid = create_test_project(&client, &owner, "ClaimProj");

    let err = client.mock_all_auths().try_set_project_claimable(&pid, &stranger, &true);
    assert_eq!(err, Err(Ok(ContractError::Unauthorized)));
    assert!(!client.get_project(&pid).unwrap().claimable);

    client.mock_all_auths().set_project_claimable(&pid, &owner, &true);
    assert!(client.get_project(&pid).unwrap().claimable);
}

// ── approve/reject_claim_request: admin ✓ | stranger ✗ ──────────────────────

#[test]
fn matrix_approve_claim_admin_succeeds_stranger_fails() {
    let env = Env::default();
    let (client, admin) = init(&env);
    let owner = Address::generate(&env);
    let claimant = Address::generate(&env);
    let stranger = Address::generate(&env);
    let pid = create_test_project(&client, &owner, "ClaimAppr");
    client.mock_all_auths().set_project_claimable(&pid, &owner, &true);
    let claim_id = client
        .mock_all_auths()
        .submit_claim_request(&pid, &claimant, &String::from_str(&env, "ipfs://proof"));

    let err = client.mock_all_auths().try_approve_claim_request(&claim_id, &stranger);
    assert_eq!(err, Err(Ok(ContractError::AdminOnly)));

    client.mock_all_auths().approve_claim_request(&claim_id, &admin);
    use crate::types::ClaimStatus;
    assert_eq!(
        client.get_claim_request(&claim_id).unwrap().status,
        ClaimStatus::Approved
    );
}

#[test]
fn matrix_reject_claim_admin_succeeds_stranger_fails() {
    let env = Env::default();
    let (client, admin) = init(&env);
    let owner = Address::generate(&env);
    let claimant = Address::generate(&env);
    let stranger = Address::generate(&env);
    let pid = create_test_project(&client, &owner, "ClaimRej");
    client.mock_all_auths().set_project_claimable(&pid, &owner, &true);
    let claim_id = client
        .mock_all_auths()
        .submit_claim_request(&pid, &claimant, &String::from_str(&env, "ipfs://proof"));

    let err = client.mock_all_auths().try_reject_claim_request(&claim_id, &stranger);
    assert_eq!(err, Err(Ok(ContractError::AdminOnly)));

    client.mock_all_auths().reject_claim_request(&claim_id, &admin);
    use crate::types::ClaimStatus;
    assert_eq!(
        client.get_claim_request(&claim_id).unwrap().status,
        ClaimStatus::Rejected
    );
}

// ── clear_project_reports: admin ✓ | stranger ✗ ──────────────────────────────

#[test]
fn matrix_clear_reports_admin_succeeds_stranger_fails() {
    let env = Env::default();
    let (client, admin) = init(&env);
    let owner = Address::generate(&env);
    let reporter = Address::generate(&env);
    let stranger = Address::generate(&env);
    let pid = create_test_project(&client, &owner, "ReportProj");
    client.mock_all_auths().report_project(&pid, &reporter, &String::from_str(&env, "QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG"));
    assert_eq!(client.get_project_report_count(&pid), 1u32);

    let err = client.mock_all_auths().try_clear_project_reports(&pid, &stranger);
    assert_eq!(err, Err(Ok(ContractError::AdminOnly)));
    assert_eq!(client.get_project_report_count(&pid), 1u32);

    client.mock_all_auths().clear_project_reports(&pid, &admin);
    assert_eq!(client.get_project_report_count(&pid), 0u32);
}

// ── resolve_duplicate_dispute: admin ✓ | stranger ✗ ─────────────────────────

#[test]
fn matrix_resolve_dispute_admin_succeeds_stranger_fails() {
    let env = Env::default();
    let (client, admin) = init(&env);
    let owner = Address::generate(&env);
    let stranger = Address::generate(&env);
    let pid1 = create_test_project(&client, &owner, "Disp1");
    let pid2 = create_test_project(&client, &owner, "Disp2");
    let dispute_id = client.mock_all_auths().open_duplicate_dispute(
        &pid1,
        &pid2,
        &stranger,
        &String::from_str(&env, "QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG"),
    );

    let err = client.mock_all_auths().try_resolve_duplicate_dispute(
        &dispute_id,
        &stranger,
        &DisputeResolutionAction::Reject,
    );
    assert_eq!(err, Err(Ok(ContractError::AdminOnly)));
    use crate::types::DisputeStatus;
    assert_eq!(
        client.get_duplicate_dispute(&dispute_id).unwrap().status,
        DisputeStatus::Pending
    );

    client.mock_all_auths().resolve_duplicate_dispute(
        &dispute_id,
        &admin,
        &DisputeResolutionAction::Reject,
    );
    assert_eq!(
        client.get_duplicate_dispute(&dispute_id).unwrap().status,
        DisputeStatus::Rejected
    );
}
