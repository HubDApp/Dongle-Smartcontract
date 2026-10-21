//! Migration export tests (#935): bundle contents and metadata, paging,
//! version rules, and bundle validation.

use crate::constants::{CONTRACT_VERSION, MIGRATION_FORMAT_V1};
use crate::errors::ContractError;
use crate::migration_export::MigrationExport;
use crate::tests::fixtures::{create_test_project, setup_contract};
use crate::types::MigrationIssue;
use soroban_sdk::{testutils::Address as _, Address, BytesN, Env, String};

fn s(env: &Env, value: &str) -> String {
    String::from_str(env, value)
}

#[test]
fn test_export_bundle_contents_and_metadata() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let p1 = create_test_project(&client, &admin, "MigrateA");
    create_test_project(&client, &admin, "MigrateB");
    client.add_review(&p1, &Address::generate(&env), &4, &None);

    let bundle = client.export_for_migration(&admin, &s(&env, "1.1.0"), &0, &0);
    let meta = &bundle.metadata;

    assert_eq!(meta.format_version, MIGRATION_FORMAT_V1);
    assert_eq!(meta.source_contract, client.address);
    assert_eq!(meta.source_contract_version, s(&env, CONTRACT_VERSION));
    assert_eq!(meta.target_version, s(&env, "1.1.0"));
    assert_eq!(meta.exported_by, admin);
    assert_eq!(meta.project_count, 2);
    assert_eq!((meta.start_project_id, meta.end_project_id), (1, 2));
    assert_eq!(meta.next_project_id, None);
    assert_eq!(meta.project_records, 2);
    assert_eq!(meta.review_records, 1);
    assert_eq!(bundle.projects.len(), 2);
    assert!(bundle.config.admins.contains(&admin));
}

#[test]
fn test_export_pages_with_next_project_id() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    for name in ["PgA", "PgB", "PgC"] {
        create_test_project(&client, &admin, name);
    }
    let target = s(&env, "1.0.1");

    let first = client.export_for_migration(&admin, &target, &1, &2);
    assert_eq!(first.metadata.next_project_id, Some(3));

    let second = client.export_for_migration(&admin, &target, &3, &2);
    assert_eq!(second.metadata.next_project_id, None);
    assert_eq!(second.projects.get(0).unwrap().id, 3);
}

#[test]
fn test_export_rejects_bad_versions_and_non_admins() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);

    for bad in ["", "1.0", "1.0.0.0", "v1.0.0", "1..0", "1.0.x"] {
        assert_eq!(
            client.try_export_for_migration(&admin, &s(&env, bad), &0, &0),
            Err(Ok(ContractError::InvalidInput.into())),
            "{bad}"
        );
    }
    for incompatible in ["0.9.0", "3.0.0"] {
        assert_eq!(
            client.try_export_for_migration(&admin, &s(&env, incompatible), &0, &0),
            Err(Ok(ContractError::IncompatibleMigrationTarget.into())),
            "{incompatible}"
        );
    }
    assert_eq!(
        client.try_export_for_migration(&Address::generate(&env), &s(&env, "1.1.0"), &0, &0),
        Err(Ok(ContractError::AdminOnly.into()))
    );
}

#[test]
fn test_validate_accepts_untouched_bundle() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let p1 = create_test_project(&client, &admin, "ValidOk");
    client.add_review(&p1, &Address::generate(&env), &5, &None);
    let target = s(&env, "2.0.0");

    let bundle = client.export_for_migration(&admin, &target, &0, &0);
    let report = client.validate_migration_bundle(&bundle, &target);

    assert!(report.compatible, "{:?}", report.issues);
    assert!(report.issues.is_empty());
    assert_eq!(report.format_version, MIGRATION_FORMAT_V1);
}

#[test]
fn test_validate_detects_tampering_and_mismatches() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    create_test_project(&client, &admin, "Tamper");
    let target = s(&env, "1.2.0");
    let bundle = client.export_for_migration(&admin, &target, &0, &0);

    let mut edited = bundle.clone();
    let mut project = edited.projects.get(0).unwrap();
    project.name = s(&env, "Edited");
    edited.projects.set(0, project);
    let report = client.validate_migration_bundle(&edited, &target);
    assert!(!report.compatible);
    assert!(report.issues.contains(&MigrationIssue::ContentHashMismatch));

    let mut dropped = bundle.clone();
    dropped.projects.pop_back();
    let report = client.validate_migration_bundle(&dropped, &target);
    assert!(report.issues.contains(&MigrationIssue::RecordCountMismatch));

    let mut future = bundle.clone();
    future.metadata.format_version = MIGRATION_FORMAT_V1 + 1;
    let report = client.validate_migration_bundle(&future, &target);
    assert!(report.issues.contains(&MigrationIssue::UnsupportedFormatVersion));

    let report = client.validate_migration_bundle(&bundle, &s(&env, "1.3.0"));
    assert!(report.issues.contains(&MigrationIssue::TargetVersionMismatch));

    let mut forged = bundle.clone();
    forged.metadata.content_hash = BytesN::from_array(&env, &[0u8; 32]);
    let report = client.validate_migration_bundle(&forged, &target);
    assert!(report.issues.contains(&MigrationIssue::ContentHashMismatch));
}

#[test]
fn test_validate_reports_version_rules() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let bundle = client.export_for_migration(&admin, &s(&env, "1.0.0"), &0, &0);

    let mut downgrade = bundle.clone();
    downgrade.metadata.target_version = s(&env, "0.5.0");
    let report = client.validate_migration_bundle(&downgrade, &s(&env, "0.5.0"));
    assert!(report.issues.contains(&MigrationIssue::DowngradeNotSupported));

    let mut jump = bundle.clone();
    jump.metadata.target_version = s(&env, "5.0.0");
    let report = client.validate_migration_bundle(&jump, &s(&env, "5.0.0"));
    assert!(report.issues.contains(&MigrationIssue::MajorVersionJumpTooLarge));

    let report = client.validate_migration_bundle(&bundle, &s(&env, "latest"));
    assert!(report.issues.contains(&MigrationIssue::InvalidVersionString));
}

#[test]
fn test_parse_version() {
    let env = Env::default();
    assert_eq!(MigrationExport::parse_version(&s(&env, "1.2.3")), Some((1, 2, 3)));
    assert_eq!(MigrationExport::parse_version(&s(&env, "10.0.42")), Some((10, 0, 42)));
    assert_eq!(MigrationExport::parse_version(&s(&env, "1.2.")), None);
    assert_eq!(MigrationExport::parse_version(&s(&env, ".1.2")), None);
    assert_eq!(MigrationExport::parse_version(&s(&env, "99999999999.0.0")), None);
    assert!(
        MigrationExport::parse_version(&s(&env, "1.10.0"))
            > MigrationExport::parse_version(&s(&env, "1.9.9"))
    );
}
