//! Analytics export tests (#934): metric consistency with on-chain state,
//! date windows, filters, paging, and JSON / CSV rendering.

use crate::constants::MAX_ANALYTICS_SCAN_SIZE;
use crate::errors::ContractError;
use crate::tests::fixtures::{create_test_project, setup_contract};
use crate::types::{AnalyticsFilter, ExportFormat};
use crate::DongleContractClient;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    Address, Env, String,
};

const EVIDENCE_CID: &str = "QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG";

fn filter(start_project_id: u64, limit: u32) -> AnalyticsFilter {
    AnalyticsFilter {
        category: None,
        verified_only: false,
        start_project_id,
        limit,
    }
}

/// Pay the verification fee for `project_id` and request verification.
fn request_paid_verification(
    env: &Env,
    client: &DongleContractClient<'_>,
    admin: &Address,
    owner: &Address,
    project_id: u64,
    fee: u128,
) {
    let token = env
        .register_stellar_asset_contract_v2(Address::generate(env))
        .address();
    client.set_fee(admin, &Some(token.clone()), &fee, &0, admin);
    soroban_sdk::token::StellarAssetClient::new(env, &token).mint(owner, &(fee as i128 * 10));
    client.pay_fee(owner, &project_id, &Some(token));
    client.request_verification(&project_id, owner, &String::from_str(env, EVIDENCE_CID));
}

#[test]
fn test_metrics_match_on_chain_counts() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let p1 = create_test_project(&client, &owner, "MetricsA");
    let p2 = create_test_project(&client, &owner, "MetricsB");
    for _ in 0..3 {
        client.add_review(&p1, &Address::generate(&env), &4, &None);
    }
    client.add_review(&p2, &Address::generate(&env), &5, &None);

    let export = client.export_analytics_data(&0, &u64::MAX, &ExportFormat::Json);
    let m = export.metrics;

    assert_eq!(m.projects_scanned as u64, client.get_project_count());
    assert_eq!(m.projects_created, 2);
    assert_eq!(
        m.reviews_created,
        client.get_project_stats(&p1).review_count + client.get_project_stats(&p2).review_count
    );
    assert_eq!(m.verification_requests, 0);
    assert_eq!(m.verification_rate_bps, 0);
    assert_eq!(m.total_fee_revenue, 0);
    assert_eq!(export.next_project_id, None);
}

#[test]
fn test_verification_rate_and_fee_revenue() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let approved = create_test_project(&client, &owner, "RateApproved");
    let rejected = create_test_project(&client, &owner, "RateRejected");

    request_paid_verification(&env, &client, &admin, &owner, approved, 100);
    client.approve_verification(&approved, &admin);
    request_paid_verification(&env, &client, &admin, &owner, rejected, 100);
    client.reject_verification(&rejected, &admin);

    let m = client
        .export_analytics_data(&0, &u64::MAX, &ExportFormat::Json)
        .metrics;

    assert_eq!(m.verification_requests, 2);
    assert_eq!(m.verifications_approved, 1);
    assert_eq!(m.verifications_rejected, 1);
    assert_eq!(m.verification_rate_bps, 5_000);
    assert_eq!(m.verification_fee_revenue, 200);
    assert_eq!(
        m.total_fee_revenue,
        m.verification_fee_revenue + m.registration_fee_revenue
    );
}

#[test]
fn test_date_range_excludes_activity_outside_window() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);

    env.ledger().with_mut(|l| l.timestamp = 1_000);
    let early = create_test_project(&client, &admin, "Early");
    client.add_review(&early, &Address::generate(&env), &3, &None);

    env.ledger().with_mut(|l| l.timestamp = 5_000);
    let late = create_test_project(&client, &admin, "Late");
    client.add_review(&late, &Address::generate(&env), &4, &None);
    client.add_review(&early, &Address::generate(&env), &5, &None);

    let window = client
        .export_analytics_data(&4_000, &6_000, &ExportFormat::Json)
        .metrics;
    assert_eq!(window.projects_scanned, 2);
    assert_eq!(window.projects_created, 1);
    assert_eq!(window.reviews_created, 2);

    let before = client
        .export_analytics_data(&0, &999, &ExportFormat::Json)
        .metrics;
    assert_eq!(before.projects_created, 0);
    assert_eq!(before.reviews_created, 0);
}

#[test]
fn test_inverted_range_is_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);

    assert_eq!(
        client.try_export_analytics_data(&10, &5, &ExportFormat::Csv),
        Err(Ok(ContractError::InvalidInput.into()))
    );
}

#[test]
fn test_category_and_verified_filters() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let verified = create_test_project(&client, &owner, "FilterVerified");
    create_test_project(&client, &owner, "FilterPlain");
    request_paid_verification(&env, &client, &admin, &owner, verified, 100);
    client.approve_verification(&verified, &admin);

    let mut only_verified = filter(1, 0);
    only_verified.verified_only = true;
    let m = client
        .export_analytics_data_filtered(&0, &u64::MAX, &ExportFormat::Json, &only_verified)
        .metrics;
    assert_eq!(m.projects_scanned, 2);
    assert_eq!(m.projects_matched, 1);
    assert_eq!(m.projects_created, 1);

    // Fixture projects are all "DeFi".
    let mut other_category = filter(1, 0);
    other_category.category = Some(String::from_str(&env, "NFT"));
    let m = client
        .export_analytics_data_filtered(&0, &u64::MAX, &ExportFormat::Json, &other_category)
        .metrics;
    assert_eq!(m.projects_matched, 0);
    assert_eq!(m.projects_created, 0);
}

#[test]
fn test_paging_sums_to_full_scan() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    for name in ["PageOne", "PageTwo", "PageThree"] {
        let pid = create_test_project(&client, &admin, name);
        client.add_review(&pid, &Address::generate(&env), &4, &None);
    }

    let first = client.export_analytics_data_filtered(&0, &u64::MAX, &ExportFormat::Csv, &filter(1, 2));
    assert_eq!(first.metrics.projects_scanned, 2);
    assert_eq!(first.next_project_id, Some(3));

    let second = client.export_analytics_data_filtered(
        &0,
        &u64::MAX,
        &ExportFormat::Csv,
        &filter(first.next_project_id.unwrap(), 2),
    );
    assert_eq!(second.next_project_id, None);

    let full = client.export_analytics_data_filtered(
        &0,
        &u64::MAX,
        &ExportFormat::Csv,
        &filter(1, MAX_ANALYTICS_SCAN_SIZE + 10),
    );
    assert_eq!(
        first.metrics.reviews_created + second.metrics.reviews_created,
        full.metrics.reviews_created
    );
    assert_eq!(full.metrics.projects_scanned, 3);
}

#[test]
fn test_payload_formats() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    create_test_project(&client, &admin, "Payload");

    let json = client.export_analytics_data(&0, &100, &ExportFormat::Json);
    assert_eq!(json.format, ExportFormat::Json);
    let expected_json = String::from_str(
        &env,
        "{\"start\":0,\"end\":100,\"projects_scanned\":1,\"projects_matched\":1,\
         \"projects_created\":1,\"reviews_created\":0,\"verification_requests\":0,\
         \"verifications_approved\":0,\"verifications_rejected\":0,\
         \"verification_rate_bps\":0,\"verification_fee_revenue\":\"0\",\
         \"registration_fee_revenue\":\"0\",\"total_fee_revenue\":\"0\"}",
    );
    assert_eq!(json.payload, expected_json);

    let csv = client.export_analytics_data(&0, &100, &ExportFormat::Csv);
    let expected_csv = String::from_str(
        &env,
        "start,end,projects_scanned,projects_matched,projects_created,reviews_created,\
         verification_requests,verifications_approved,verifications_rejected,\
         verification_rate_bps,verification_fee_revenue,registration_fee_revenue,\
         total_fee_revenue\n0,100,1,1,1,0,0,0,0,0,0,0,0\n",
    );
    assert_eq!(csv.payload, expected_csv);
    assert_eq!(csv.metrics, json.metrics);
}
