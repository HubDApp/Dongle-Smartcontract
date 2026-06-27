//! Tests for verified-project metadata invalidation.
//!
//! ## Metadata Policy Summary
//!
//! Once a project is `Verified`, the following identity-critical fields
//! are **frozen** — any attempt to change them is rejected with
//! `ContractError::VerifiedFieldFrozen`:
//!
//! | Field         | Reason frozen                                         |
//! |---------------|-------------------------------------------------------|
//! | `name`        | Public identity anchor; users trust the verified name |
//! | `slug`        | Stable URL identifier; changes break / enable spoofing|
//! | `category`    | Verification may be category-specific                 |
//! | `logo_cid`    | Visual identity audited during verification           |
//! | `metadata_cid`| Evidence CID used during the verification review      |
//!
//! Major fields (`name`, `website`, `metadata_cid`) reset verification.
//! Minor fields (`description`, `tags`, `social_links`, `launch_timestamp`)
//! remain freely mutable after verification.
//!
//! To change a frozen field, an admin must first revoke verification;
//! the project then returns to `Unverified` status and may be re-verified.

use crate::constants::MAJOR_METADATA_FIELDS;
use crate::errors::ContractError;
use crate::events::VerificationStatusResetEvent;
use crate::tests::fixtures::{create_test_project, setup_contract};
use crate::types::{ProjectUpdateParams, VerificationStatus};
use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, Events},
    Address, Env, IntoVal, String as SorobanString, TryIntoVal, Val,
};

// ─── helpers ─────────────────────────────────────────────────────────────────

fn mk_env() -> Env {
    Env::default()
}

/// Approve verification for a project — shortcut through the full flow.
fn approve_verification(
    client: &crate::DongleContractClient<'_>,
    admin: &Address,
    project_id: u64,
    env: &Env,
) {
    let evidence = SorobanString::from_str(env, "QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG");
    let owner = client.get_project(&project_id).unwrap().owner;
    client.request_verification(&project_id, &owner, &evidence);
    client.approve_verification(&project_id, admin);
}

fn decode_event<T: soroban_sdk::TryFromVal<Env, Val>>(env: &Env, data: &Val) -> Option<T> {
    TryIntoVal::<_, T>::try_into_val(data, env).ok()
}

fn has_verification_reset_event(env: &Env, project_id: u64, field: &str) -> bool {
    let expected_topics =
        (symbol_short!("VERIFY"), symbol_short!("RESET"), project_id).into_val(env);
    let expected_field = SorobanString::from_str(env, field);

    env.events().all().iter().any(|(_, topics, data)| {
        topics == expected_topics
            && decode_event::<VerificationStatusResetEvent>(env, &data)
                .map(|event| {
                    event.project_id == project_id
                        && event.previous_status == VerificationStatus::Verified
                        && event.new_status == VerificationStatus::Unverified
                        && event.fields.contains(&expected_field)
                })
                .unwrap_or(false)
    })
}

// ═══════════════════════════════════════════════════════════════════════════
// Unit-level: Utils::check_frozen_fields
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn unit_freeze_unverified_no_restriction() {
    // Not verified → all changes allowed
    let r = crate::utils::Utils::check_frozen_fields(false, true, true, true, true, true);
    assert!(r.is_ok());
}

#[test]
fn unit_freeze_verified_name_blocked() {
    let r = crate::utils::Utils::check_frozen_fields(true, true, false, false, false, false);
    assert!(r.is_ok());
}

#[test]
fn unit_freeze_verified_slug_blocked() {
    let r = crate::utils::Utils::check_frozen_fields(true, false, true, false, false, false);
    assert_eq!(r, Err(ContractError::VerifiedFieldFrozen));
}

#[test]
fn unit_freeze_verified_category_blocked() {
    let r = crate::utils::Utils::check_frozen_fields(true, false, false, true, false, false);
    assert_eq!(r, Err(ContractError::VerifiedFieldFrozen));
}

#[test]
fn unit_freeze_verified_logo_cid_blocked() {
    let r = crate::utils::Utils::check_frozen_fields(true, false, false, false, true, false);
    assert_eq!(r, Err(ContractError::VerifiedFieldFrozen));
}

#[test]
fn unit_freeze_verified_metadata_cid_blocked() {
    let r = crate::utils::Utils::check_frozen_fields(true, false, false, false, false, true);
    assert!(r.is_ok());
}

#[test]
fn unit_freeze_verified_no_changes_ok() {
    // Verified but none of the frozen fields are being changed
    let r = crate::utils::Utils::check_frozen_fields(true, false, false, false, false, false);
    assert!(r.is_ok());
}

#[test]
fn major_metadata_fields_are_defined() {
    assert_eq!(MAJOR_METADATA_FIELDS, ["name", "website", "metadata_cid"]);
}

// ═══════════════════════════════════════════════════════════════════════════
// Integration: update_project on a verified project
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn verified_project_update_name_resets_verification() {
    let env = mk_env();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let project_id = create_test_project(&client, &admin, "MyProject");
    approve_verification(&client, &admin, project_id, &env);

    let project = client.get_project(&project_id).unwrap();
    assert_eq!(project.verification_status, VerificationStatus::Verified);

    let params = ProjectUpdateParams {
        project_id,
        caller: admin.clone(),
        name: Some(SorobanString::from_str(&env, "NewName")),
        slug: None,
        description: None,
        category: None,
        website: None,
        logo_cid: None,
        metadata_cid: None,
        tags: None,
        social_links: None,
        launch_timestamp: None,
        bounty_url: None,
    };
    let project = client.update_project(&params);
    assert_eq!(project.name, SorobanString::from_str(&env, "NewName"));
    assert_eq!(project.verification_status, VerificationStatus::Unverified);
    assert!(has_verification_reset_event(&env, project_id, "name"));
}

#[test]
fn verified_project_update_slug_blocked() {
    let env = mk_env();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let project_id = create_test_project(&client, &admin, "SlugProject");
    approve_verification(&client, &admin, project_id, &env);

    let params = ProjectUpdateParams {
        project_id,
        caller: admin.clone(),
        name: None,
        slug: Some(SorobanString::from_str(&env, "brand-new-slug")),
        description: None,
        category: None,
        website: None,
        logo_cid: None,
        metadata_cid: None,
        tags: None,
        social_links: None,
        launch_timestamp: None,
        bounty_url: None,
    };
    let result = client.try_update_project(&params);
    assert_eq!(result, Err(Ok(ContractError::VerifiedFieldFrozen.into())));
}

#[test]
fn verified_project_update_category_blocked() {
    let env = mk_env();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let project_id = create_test_project(&client, &admin, "CatProject");
    approve_verification(&client, &admin, project_id, &env);

    let params = ProjectUpdateParams {
        project_id,
        caller: admin.clone(),
        name: None,
        slug: None,
        description: None,
        category: Some(SorobanString::from_str(&env, "NFT")),
        website: None,
        logo_cid: None,
        metadata_cid: None,
        tags: None,
        social_links: None,
        launch_timestamp: None,
        bounty_url: None,
    };
    let result = client.try_update_project(&params);
    assert_eq!(result, Err(Ok(ContractError::VerifiedFieldFrozen.into())));
}

#[test]
fn verified_project_update_logo_cid_blocked() {
    let env = mk_env();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let project_id = create_test_project(&client, &admin, "LogoProject");
    approve_verification(&client, &admin, project_id, &env);

    let params = ProjectUpdateParams {
        project_id,
        caller: admin.clone(),
        name: None,
        slug: None,
        description: None,
        category: None,
        website: None,
        logo_cid: Some(Some(SorobanString::from_str(
            &env,
            "QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG",
        ))),
        metadata_cid: None,
        tags: None,
        social_links: None,
        launch_timestamp: None,
        bounty_url: None,
    };
    let result = client.try_update_project(&params);
    assert_eq!(result, Err(Ok(ContractError::VerifiedFieldFrozen.into())));
}

#[test]
fn verified_project_update_metadata_cid_resets_verification() {
    let env = mk_env();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let project_id = create_test_project(&client, &admin, "MetaProject");
    approve_verification(&client, &admin, project_id, &env);

    let params = ProjectUpdateParams {
        project_id,
        caller: admin.clone(),
        name: None,
        slug: None,
        description: None,
        category: None,
        website: None,
        logo_cid: None,
        metadata_cid: Some(Some(SorobanString::from_str(
            &env,
            "bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi",
        ))),
        tags: None,
        social_links: None,
        launch_timestamp: None,
        bounty_url: None,
    };
    let project = client.update_project(&params);
    assert_eq!(project.verification_status, VerificationStatus::Unverified);
    assert!(has_verification_reset_event(
        &env,
        project_id,
        "metadata_cid"
    ));
}

// ─── Mutable fields remain editable after verification ────────────────────

#[test]
fn verified_project_update_description_allowed() {
    let env = mk_env();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let project_id = create_test_project(&client, &admin, "DescProject");
    approve_verification(&client, &admin, project_id, &env);

    let params = ProjectUpdateParams {
        project_id,
        caller: admin.clone(),
        name: None,
        slug: None,
        description: Some(SorobanString::from_str(&env, "Updated description text")),
        category: None,
        website: None,
        logo_cid: None,
        metadata_cid: None,
        tags: None,
        social_links: None,
        launch_timestamp: None,
        bounty_url: None,
    };
    let result = client.try_update_project(&params);
    assert!(
        result.is_ok(),
        "description update should be allowed on verified project"
    );
    let project = client.get_project(&project_id).unwrap();
    assert_eq!(
        project.description,
        SorobanString::from_str(&env, "Updated description text")
    );
    assert_eq!(project.verification_status, VerificationStatus::Verified);
}

#[test]
fn verified_project_update_website_resets_verification() {
    let env = mk_env();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let project_id = create_test_project(&client, &admin, "WebProject");
    approve_verification(&client, &admin, project_id, &env);

    let params = ProjectUpdateParams {
        project_id,
        caller: admin.clone(),
        name: None,
        slug: None,
        description: None,
        category: None,
        website: Some(Some(SorobanString::from_str(
            &env,
            "https://newsite.example.com",
        ))),
        logo_cid: None,
        metadata_cid: None,
        tags: None,
        social_links: None,
        launch_timestamp: None,
        bounty_url: None,
    };
    let project = client.update_project(&params);
    assert_eq!(project.verification_status, VerificationStatus::Unverified);
    assert!(has_verification_reset_event(&env, project_id, "website"));
}

#[test]
fn verified_project_no_change_to_frozen_fields_allowed() {
    // Submitting the same values for frozen fields should NOT trigger the freeze,
    // because the values are not actually changing.
    let env = mk_env();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let project_id = create_test_project(&client, &admin, "SameFields");
    approve_verification(&client, &admin, project_id, &env);

    let project = client.get_project(&project_id).unwrap();

    let params = ProjectUpdateParams {
        project_id,
        caller: admin.clone(),
        // Same name → no diff detected → freeze doesn't trigger
        name: Some(project.name.clone()),
        slug: Some(project.slug.clone()),
        description: Some(SorobanString::from_str(&env, "Updated description")),
        category: Some(project.category.clone()),
        website: None,
        logo_cid: None,
        metadata_cid: None,
        tags: None,
        social_links: None,
        launch_timestamp: None,
        bounty_url: None,
    };
    let result = client.try_update_project(&params);
    assert!(
        result.is_ok(),
        "submitting unchanged frozen-field values should be allowed"
    );
}

// ─── After revocation, frozen fields are editable again ───────────────────

#[test]
fn after_revoke_frozen_fields_become_mutable() {
    let env = mk_env();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let project_id = create_test_project(&client, &admin, "RevokeTest");
    approve_verification(&client, &admin, project_id, &env);

    // Verify the project is Verified
    let p = client.get_project(&project_id).unwrap();
    assert_eq!(p.verification_status, VerificationStatus::Verified);

    // Revoke verification
    let reason = SorobanString::from_str(&env, "Testing revocation for field unfreeze");
    client.revoke_verification(&project_id, &admin, &reason);

    // After revocation the project should no longer be Verified
    let p = client.get_project(&project_id).unwrap();
    assert_ne!(p.verification_status, VerificationStatus::Verified);

    // Now changing the name should succeed
    let params = ProjectUpdateParams {
        project_id,
        caller: admin.clone(),
        name: Some(SorobanString::from_str(&env, "NewNameAfterRevoke")),
        slug: None,
        description: None,
        category: None,
        website: None,
        logo_cid: None,
        metadata_cid: None,
        tags: None,
        social_links: None,
        launch_timestamp: None,
        bounty_url: None,
    };
    let result = client.try_update_project(&params);
    assert!(
        result.is_ok(),
        "name change after revocation should succeed"
    );
}

// ─── Unverified project: all fields are mutable ───────────────────────────

#[test]
fn unverified_project_all_fields_mutable() {
    let env = mk_env();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let project_id = create_test_project(&client, &admin, "FreeProject");

    let project = client.get_project(&project_id).unwrap();
    assert_eq!(project.verification_status, VerificationStatus::Unverified);

    let params = ProjectUpdateParams {
        project_id,
        caller: admin.clone(),
        name: Some(SorobanString::from_str(&env, "NewFreeName")),
        slug: Some(SorobanString::from_str(&env, "new-free-slug")),
        description: Some(SorobanString::from_str(&env, "New description")),
        category: Some(SorobanString::from_str(&env, "NFT")),
        website: Some(Some(SorobanString::from_str(&env, "https://example.com"))),
        logo_cid: None,
        metadata_cid: None,
        tags: None,
        social_links: None,
        launch_timestamp: None,
        bounty_url: None,
    };
    let result = client.try_update_project(&params);
    assert!(
        result.is_ok(),
        "unverified project should allow all field updates"
    );
}

// ─── Pending-verification project: frozen fields still mutable ────────────

#[test]
fn pending_verification_project_fields_are_mutable() {
    let env = mk_env();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let project_id = create_test_project(&client, &admin, "PendingProject");

    // Submit verification request but don't approve yet
    let evidence = SorobanString::from_str(&env, "QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG");
    let owner = client.get_project(&project_id).unwrap().owner;
    client.request_verification(&project_id, &owner, &evidence);

    let project = client.get_project(&project_id).unwrap();
    assert_eq!(project.verification_status, VerificationStatus::Pending);

    // Changing name on a Pending project should still be allowed
    let params = ProjectUpdateParams {
        project_id,
        caller: admin.clone(),
        name: Some(SorobanString::from_str(&env, "PendingNewName")),
        slug: None,
        description: None,
        category: None,
        website: None,
        logo_cid: None,
        metadata_cid: None,
        tags: None,
        social_links: None,
        launch_timestamp: None,
        bounty_url: None,
    };
    let result = client.try_update_project(&params);
    assert!(
        result.is_ok(),
        "fields should be mutable while verification is only Pending"
    );
}
