//! Report registry tests (issue #497).
//!
//! Covers `report_project`, duplicate prevention via `has_user_reported`,
//! `get_project_reports`, `get_project_report_count`, and the admin-only
//! `clear_project_reports` — including that clearing resets the per-reporter
//! dedup keys so the same user may report again.

#![cfg(test)]

use crate::tests::fixtures::{create_test_project, setup_contract};
use soroban_sdk::{testutils::Address as _, Address, Env, String};

extern crate alloc;
use alloc::format;

/// A syntactically valid IPFS CID, varied by its final character so a test can
/// distinguish two reports. The registry rejects anything that is not a CID.
fn reason_cid(env: &Env, last: &str) -> String {
    String::from_str(
        env,
        &format!("QmYwAPJhy5nTAQCj9g1s2bkss7jBlEd22bN2R4s5gR5PT{last}"),
    )
}

// ─── report_project ──────────────────────────────────────────────────────────

#[test]
fn test_report_project_records_the_report() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);

    let owner = Address::generate(&env);
    let reporter = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "ReportableProject");
    let cid = reason_cid(&env, "a");

    client.report_project(&project_id, &reporter, &cid);

    assert_eq!(client.get_project_report_count(&project_id), 1);
    let reports = client.get_project_reports(&project_id);
    assert_eq!(reports.len(), 1);

    let report = reports.get(0).unwrap();
    assert_eq!(report.project_id, project_id);
    assert_eq!(report.reporter, reporter);
    assert_eq!(report.reason_cid, cid);
}

#[test]
fn test_report_stores_the_ledger_timestamp() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);

    let owner = Address::generate(&env);
    let reporter = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "TimestampedReport");

    client.report_project(&project_id, &reporter, &reason_cid(&env, "a"));

    let report = client.get_project_reports(&project_id).get(0).unwrap();
    assert_eq!(report.timestamp, env.ledger().timestamp());
}

#[test]
fn test_report_nonexistent_project_is_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);

    let reporter = Address::generate(&env);
    assert!(client
        .try_report_project(&9_999u64, &reporter, &reason_cid(&env, "a"))
        .is_err());
}

#[test]
fn test_report_with_invalid_cid_is_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);

    let owner = Address::generate(&env);
    let reporter = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "BadCidProject");

    let empty = String::from_str(&env, "");
    assert!(client
        .try_report_project(&project_id, &reporter, &empty)
        .is_err());
    assert_eq!(client.get_project_report_count(&project_id), 0);

    let garbage = String::from_str(&env, "not-a-cid");
    assert!(client
        .try_report_project(&project_id, &reporter, &garbage)
        .is_err());
    assert_eq!(client.get_project_report_count(&project_id), 0);
}

// ─── Duplicate prevention ────────────────────────────────────────────────────

#[test]
fn test_has_user_reported_tracks_each_reporter() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);

    let owner = Address::generate(&env);
    let reporter = Address::generate(&env);
    let bystander = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "DedupProject");

    assert!(!client.has_user_reported(&project_id, &reporter));

    client.report_project(&project_id, &reporter, &reason_cid(&env, "a"));

    assert!(client.has_user_reported(&project_id, &reporter));
    assert!(!client.has_user_reported(&project_id, &bystander));
}

#[test]
fn test_duplicate_report_from_same_user_is_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);

    let owner = Address::generate(&env);
    let reporter = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "DoubleReportProject");

    client.report_project(&project_id, &reporter, &reason_cid(&env, "a"));

    // Even with a different reason, one report per user per project.
    assert!(client
        .try_report_project(&project_id, &reporter, &reason_cid(&env, "b"))
        .is_err());
    assert_eq!(client.get_project_report_count(&project_id), 1);
}

#[test]
fn test_multiple_reporters_accumulate() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);

    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "PopularReportProject");

    for (i, suffix) in ["a", "b", "c"].iter().enumerate() {
        let reporter = Address::generate(&env);
        client.report_project(&project_id, &reporter, &reason_cid(&env, suffix));
        assert_eq!(client.get_project_report_count(&project_id), (i + 1) as u32);
    }

    assert_eq!(client.get_project_reports(&project_id).len(), 3);
}

#[test]
fn test_reports_are_scoped_per_project() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);

    let owner = Address::generate(&env);
    let reporter = Address::generate(&env);
    let first = create_test_project(&client, &owner, "FirstScoped");
    let second = create_test_project(&client, &owner, "SecondScoped");

    client.report_project(&first, &reporter, &reason_cid(&env, "a"));

    assert_eq!(client.get_project_report_count(&first), 1);
    assert_eq!(client.get_project_report_count(&second), 0);
    // The same reporter may still report a *different* project.
    assert!(!client.has_user_reported(&second, &reporter));
    client.report_project(&second, &reporter, &reason_cid(&env, "b"));
    assert_eq!(client.get_project_report_count(&second), 1);
}

// ─── Empty state ─────────────────────────────────────────────────────────────

#[test]
fn test_queries_on_unreported_project_are_empty() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);

    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "PristineProject");

    assert_eq!(client.get_project_report_count(&project_id), 0);
    assert_eq!(client.get_project_reports(&project_id).len(), 0);
}

// ─── clear_project_reports ───────────────────────────────────────────────────

#[test]
fn test_admin_can_clear_reports() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);

    let owner = Address::generate(&env);
    let reporter = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "ClearableProject");

    client.report_project(&project_id, &reporter, &reason_cid(&env, "a"));
    assert_eq!(client.get_project_report_count(&project_id), 1);

    client.clear_project_reports(&project_id, &admin);

    assert_eq!(client.get_project_report_count(&project_id), 0);
    assert_eq!(client.get_project_reports(&project_id).len(), 0);
}

#[test]
fn test_clearing_lets_the_same_user_report_again() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);

    let owner = Address::generate(&env);
    let reporter = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "ReReportProject");

    client.report_project(&project_id, &reporter, &reason_cid(&env, "a"));
    assert!(client.has_user_reported(&project_id, &reporter));

    client.clear_project_reports(&project_id, &admin);

    // The per-reporter dedup key must be removed, not just the list.
    assert!(!client.has_user_reported(&project_id, &reporter));
    client.report_project(&project_id, &reporter, &reason_cid(&env, "b"));
    assert_eq!(client.get_project_report_count(&project_id), 1);
}

#[test]
fn test_clear_removes_dedup_keys_for_every_reporter() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);

    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "MultiClearProject");

    let reporters: alloc::vec::Vec<Address> = (0..3).map(|_| Address::generate(&env)).collect();
    for (i, reporter) in reporters.iter().enumerate() {
        let suffix = ["a", "b", "c"][i];
        client.report_project(&project_id, reporter, &reason_cid(&env, suffix));
    }
    assert_eq!(client.get_project_report_count(&project_id), 3);

    client.clear_project_reports(&project_id, &admin);

    for reporter in reporters.iter() {
        assert!(
            !client.has_user_reported(&project_id, reporter),
            "every reporter's dedup key must be cleared, not just the first"
        );
    }
}

#[test]
fn test_non_admin_cannot_clear_reports() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);

    let owner = Address::generate(&env);
    let reporter = Address::generate(&env);
    let outsider = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "GuardedClearProject");

    client.report_project(&project_id, &reporter, &reason_cid(&env, "a"));

    assert!(client
        .try_clear_project_reports(&project_id, &outsider)
        .is_err());
    // The project owner is not an admin either.
    assert!(client
        .try_clear_project_reports(&project_id, &owner)
        .is_err());
    assert_eq!(client.get_project_report_count(&project_id), 1);
}

#[test]
fn test_clear_with_no_reports_is_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);

    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "NothingToClearProject");

    assert!(client
        .try_clear_project_reports(&project_id, &admin)
        .is_err());
}

#[test]
fn test_clear_nonexistent_project_is_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);

    assert!(client.try_clear_project_reports(&9_999u64, &admin).is_err());
}

#[test]
fn test_clear_is_scoped_to_one_project() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);

    let owner = Address::generate(&env);
    let reporter = Address::generate(&env);
    let first = create_test_project(&client, &owner, "ClearScopeFirst");
    let second = create_test_project(&client, &owner, "ClearScopeSecond");

    client.report_project(&first, &reporter, &reason_cid(&env, "a"));
    client.report_project(&second, &reporter, &reason_cid(&env, "b"));

    client.clear_project_reports(&first, &admin);

    assert_eq!(client.get_project_report_count(&first), 0);
    assert_eq!(client.get_project_report_count(&second), 1);
    assert!(client.has_user_reported(&second, &reporter));
}
