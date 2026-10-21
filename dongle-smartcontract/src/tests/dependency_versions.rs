//! Dependency version requirement tests (issue #923).
//!
//! Covers spec parsing for `1.2.3`, `^1.2`, `~1.2.3` and `>=1.0`, rejection of
//! malformed specs in `add_project_dependency`, and `get_compatible_versions`
//! filtering a dependency project's changelog versions.

#![cfg(test)]

use crate::errors::ContractError;
use crate::tests::fixtures::{create_test_project, setup_contract};
use crate::types::{DependencyRef, ProjectDependency, VersionOperator, VersionSpec};
use crate::version_spec::VersionSpecParser;
use crate::DongleContractClient;
use soroban_sdk::{testutils::Address as _, Address, Env, String};

extern crate alloc;
use alloc::format;

fn spec(env: &Env, raw: &str) -> VersionSpec {
    VersionSpecParser::parse(&String::from_str(env, raw)).unwrap()
}

fn matches(env: &Env, raw: &str, version: (u32, u32, u32)) -> bool {
    VersionSpecParser::matches(&spec(env, raw), version)
}

/// A spec carrying only `raw`; the contract fills in the parsed fields.
fn raw_spec(env: &Env, raw: &str) -> VersionSpec {
    VersionSpec {
        raw: String::from_str(env, raw),
        operator: VersionOperator::Exact,
        major: 0,
        minor: None,
        patch: None,
    }
}

fn project_dep(env: &Env, dep_project_id: u64, version: Option<VersionSpec>) -> ProjectDependency {
    ProjectDependency {
        reference: DependencyRef {
            project_id: Some(dep_project_id),
            external_cid: None,
            external_url: None,
            external_contract: None,
            version_requirement: version,
        },
        label: None,
        metadata_cid: None,
        added_at: env.ledger().timestamp(),
        updated_at: env.ledger().timestamp(),
    }
}

fn add_release(client: &DongleContractClient<'_>, project_id: u64, owner: &Address, n: u32, version: &str) {
    let env = &client.env;
    // Vary the last character with base58-safe letters so each entry has a
    // distinct, valid CID.
    let suffix = b"abcdefghijk"[n as usize] as char;
    let cid = String::from_str(
        env,
        &format!("QmYwAPJhy5nTAQCj9g1s2bkss7jBlEd22bN2R4s5gR5PT{suffix}"),
    );
    client.add_changelog_entry(
        &project_id,
        owner,
        &cid,
        &None,
        &Some(String::from_str(env, version)),
        &None,
    );
}

// ─── Parsing ─────────────────────────────────────────────────────────────────

#[test]
fn test_parse_exact_spec() {
    let env = Env::default();
    let s = spec(&env, "1.2.3");
    assert_eq!(s.operator, VersionOperator::Exact);
    assert_eq!((s.major, s.minor, s.patch), (1, Some(2), Some(3)));
}

#[test]
fn test_parse_caret_tilde_and_gte_specs() {
    let env = Env::default();

    let caret = spec(&env, "^1.2");
    assert_eq!(caret.operator, VersionOperator::Caret);
    assert_eq!((caret.major, caret.minor, caret.patch), (1, Some(2), None));

    let tilde = spec(&env, "~1.2.3");
    assert_eq!(tilde.operator, VersionOperator::Tilde);
    assert_eq!((tilde.major, tilde.minor, tilde.patch), (1, Some(2), Some(3)));

    let gte = spec(&env, ">=1.0");
    assert_eq!(gte.operator, VersionOperator::GreaterOrEqual);
    assert_eq!((gte.major, gte.minor, gte.patch), (1, Some(0), None));
}

#[test]
fn test_parse_rejects_malformed_specs() {
    let env = Env::default();
    for raw in [
        "", "1.2", "1", "^", "~", ">=", "^1.2.3.4", "1..2", "01.2.3", "^a.b", "<1.0", ">1.0",
        "=1.2.3", "^ 1.2", "1.2.3-rc.1", "v1.2.3", "^99999999999.0",
    ] {
        assert_eq!(
            VersionSpecParser::parse(&String::from_str(&env, raw)),
            Err(ContractError::InvalidVersionSpec),
            "spec {raw:?} should be rejected"
        );
    }
}

// ─── Matching ────────────────────────────────────────────────────────────────

#[test]
fn test_exact_matches_only_that_version() {
    let env = Env::default();
    assert!(matches(&env, "1.2.3", (1, 2, 3)));
    assert!(!matches(&env, "1.2.3", (1, 2, 4)));
    assert!(!matches(&env, "1.2.3", (1, 2, 2)));
}

#[test]
fn test_caret_matches_within_major() {
    let env = Env::default();
    assert!(matches(&env, "^1.2", (1, 2, 0)));
    assert!(matches(&env, "^1.2", (1, 9, 9)));
    assert!(!matches(&env, "^1.2", (1, 1, 9)));
    assert!(!matches(&env, "^1.2", (2, 0, 0)));
    assert!(matches(&env, "^1", (1, 0, 0)));
    assert!(!matches(&env, "^1", (2, 0, 0)));
}

#[test]
fn test_caret_with_zero_major_is_minor_scoped() {
    let env = Env::default();
    assert!(matches(&env, "^0.2.3", (0, 2, 3)));
    assert!(matches(&env, "^0.2.3", (0, 2, 9)));
    assert!(!matches(&env, "^0.2.3", (0, 3, 0)));
    assert!(matches(&env, "^0.0.3", (0, 0, 3)));
    assert!(!matches(&env, "^0.0.3", (0, 0, 4)));
    assert!(matches(&env, "^0.0", (0, 0, 7)));
    assert!(!matches(&env, "^0.0", (0, 1, 0)));
}

#[test]
fn test_tilde_matches_within_minor() {
    let env = Env::default();
    assert!(matches(&env, "~1.2.3", (1, 2, 3)));
    assert!(matches(&env, "~1.2.3", (1, 2, 10)));
    assert!(!matches(&env, "~1.2.3", (1, 2, 2)));
    assert!(!matches(&env, "~1.2.3", (1, 3, 0)));
    assert!(matches(&env, "~1", (1, 5, 0)));
    assert!(!matches(&env, "~1", (2, 0, 0)));
}

#[test]
fn test_gte_matches_anything_at_or_above() {
    let env = Env::default();
    assert!(matches(&env, ">=1.0", (1, 0, 0)));
    assert!(matches(&env, ">=1.0", (7, 3, 1)));
    assert!(!matches(&env, ">=1.0", (0, 9, 9)));
}

#[test]
fn test_parse_version_handles_prefix_and_suffix() {
    let env = Env::default();
    let v = |s: &str| VersionSpecParser::parse_version(&String::from_str(&env, s));
    assert_eq!(v("1.2.3"), Some((1, 2, 3)));
    assert_eq!(v("v1.2.3"), Some((1, 2, 3)));
    assert_eq!(v("1.2.3-rc.1"), Some((1, 2, 3)));
    assert_eq!(v("1.2.3+build.5"), Some((1, 2, 3)));
    assert_eq!(v("1.2"), None);
    assert_eq!(v("latest"), None);
}

// ─── Contract integration ────────────────────────────────────────────────────

#[test]
fn test_add_dependency_stores_canonical_version_spec() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let app = create_test_project(&client, &owner, "VersionedApp");
    let lib = create_test_project(&client, &owner, "VersionedLib");

    client.add_project_dependency(&app, &owner, &project_dep(&env, lib, Some(raw_spec(&env, "~1.2.3"))));

    let stored = client.get_project_dependencies(&app).get(0).unwrap();
    let spec = stored.reference.version_requirement.unwrap();
    assert_eq!(spec.operator, VersionOperator::Tilde);
    assert_eq!((spec.major, spec.minor, spec.patch), (1, Some(2), Some(3)));
}

#[test]
fn test_add_dependency_rejects_invalid_version_spec() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let app = create_test_project(&client, &owner, "BadSpecApp");
    let lib = create_test_project(&client, &owner, "BadSpecLib");

    let result = client.try_add_project_dependency(
        &app,
        &owner,
        &project_dep(&env, lib, Some(raw_spec(&env, "<2.0"))),
    );
    assert_eq!(result, Err(Ok(ContractError::InvalidVersionSpec)));
    assert_eq!(client.get_project_dependency_count(&app), 0);
}

#[test]
fn test_get_compatible_versions_filters_changelog_versions() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let app = create_test_project(&client, &owner, "CompatApp");
    let lib = create_test_project(&client, &owner, "CompatLib");

    for (n, v) in ["0.9.0", "1.0.0", "1.2.0", "1.2.7", "1.3.0", "2.0.0", "nightly"]
        .iter()
        .enumerate()
    {
        add_release(&client, lib, &owner, n as u32, v);
    }

    client.add_project_dependency(&app, &owner, &project_dep(&env, lib, Some(raw_spec(&env, "^1.2"))));

    let versions = client.get_compatible_versions(&app, &lib);
    assert_eq!(versions.len(), 3);
    assert_eq!(versions.get(0).unwrap(), String::from_str(&env, "1.2.0"));
    assert_eq!(versions.get(1).unwrap(), String::from_str(&env, "1.2.7"));
    assert_eq!(versions.get(2).unwrap(), String::from_str(&env, "1.3.0"));
}

#[test]
fn test_get_compatible_versions_without_requirement_returns_all_valid() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let app = create_test_project(&client, &owner, "AnyVersionApp");
    let lib = create_test_project(&client, &owner, "AnyVersionLib");

    add_release(&client, lib, &owner, 0, "1.0.0");
    add_release(&client, lib, &owner, 1, "not-a-version");
    add_release(&client, lib, &owner, 2, "2.1.0");

    client.add_project_dependency(&app, &owner, &project_dep(&env, lib, None));

    assert_eq!(client.get_compatible_versions(&app, &lib).len(), 2);
}

#[test]
fn test_get_compatible_versions_unknown_dependency() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let app = create_test_project(&client, &owner, "LonelyApp");
    let other = create_test_project(&client, &owner, "UnrelatedLib");

    assert_eq!(
        client.try_get_compatible_versions(&app, &other),
        Err(Ok(ContractError::DependencyNotFound))
    );
}

#[test]
fn test_update_dependency_changes_version_requirement() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let app = create_test_project(&client, &owner, "UpdateSpecApp");
    let lib = create_test_project(&client, &owner, "UpdateSpecLib");

    let original = project_dep(&env, lib, Some(raw_spec(&env, "1.0.0")));
    client.add_project_dependency(&app, &owner, &original);

    client.update_project_dependency(
        &app,
        &owner,
        &original.reference,
        &project_dep(&env, lib, Some(raw_spec(&env, ">=1.0"))),
    );

    let stored = client.get_project_dependencies(&app).get(0).unwrap();
    assert_eq!(
        stored.reference.version_requirement.unwrap().operator,
        VersionOperator::GreaterOrEqual
    );
}
