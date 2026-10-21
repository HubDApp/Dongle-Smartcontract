//! Automatic inactivity archival, notice, search, and restore tests (#753).

use crate::auto_archive_types::{AutoArchiveConfig, AutoArchiveNoticeStatus};
use crate::errors::ContractError;
use crate::tests::fixtures::setup_contract;
use crate::types::{ProjectLifecycleStatus, ProjectRegistrationParams};
use soroban_sdk::{testutils::Address as _, Address, Env, String};

const DAY: u64 = 24 * 60 * 60;

fn register_project(
    client: &crate::DongleContractClient<'_>,
    env: &Env,
    owner: &Address,
    name: &str,
) -> u64 {
    client
        .mock_all_auths()
        .register_project(&ProjectRegistrationParams {
            owner: owner.clone(),
            name: String::from_str(env, name),
            slug: String::from_str(env, &name.to_lowercase()),
            description: String::from_str(env, "Auto archive test project"),
            category: String::from_str(env, "Infrastructure"),
            website: None,
            license: None,
            logo_cid: None,
            metadata_cid: None,
            tags: None,
            social_links: None,
            launch_timestamp: None,
            bounty_url: None,
            repository_url: None,
        })
}

fn set_time(env: &Env, timestamp: u64) {
    env.ledger().with_mut(|ledger| ledger.timestamp = timestamp);
}

#[test]
fn default_and_admin_configured_thresholds() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);
    let stranger = Address::generate(&env);
    let default = client.get_auto_archive_config();
    assert!(default.enabled);
    assert_eq!(default.inactivity_threshold_secs, 2 * 365 * DAY);

    let config = AutoArchiveConfig {
        enabled: true,
        inactivity_threshold_secs: 365 * DAY,
    };
    assert_eq!(
        client
            .mock_all_auths()
            .try_set_auto_archive_config(&stranger, &config),
        Err(Ok(ContractError::Unauthorized))
    );
    assert_eq!(
        client.mock_all_auths().try_set_auto_archive_config(
            &admin,
            &AutoArchiveConfig {
                enabled: true,
                inactivity_threshold_secs: 10,
            },
        ),
        Err(Ok(ContractError::AutoArchiveThresholdInvalid))
    );
    client
        .mock_all_auths()
        .set_auto_archive_config(&admin, &config);
    assert_eq!(client.get_auto_archive_config(), config);
}

#[test]
fn notice_thirty_day_wait_archive_search_and_owner_restore() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let keeper = Address::generate(&env);
    let stranger = Address::generate(&env);
    let project_id = register_project(&client, &env, &owner, "LegacyProject");
    let registered_at = client.get_project(project_id).unwrap().updated_at;
    let threshold = 100u64 * DAY;
    client.mock_all_auths().set_auto_archive_config(
        &admin,
        &AutoArchiveConfig {
            enabled: true,
            inactivity_threshold_secs: threshold,
        },
    );

    set_time(&env, registered_at + 70 * DAY - 1);
    assert_eq!(
        client
            .mock_all_auths()
            .try_notify_project_auto_archive(&keeper, &project_id),
        Err(Ok(ContractError::AutoArchiveNotDue))
    );

    set_time(&env, registered_at + 70 * DAY);
    assert!(client
        .mock_all_auths()
        .notify_project_auto_archive(&keeper, &project_id));
    let notice = client.get_auto_archive_notice(project_id).unwrap();
    assert_eq!(notice.owner, owner);
    assert_eq!(notice.archive_eligible_at, registered_at + threshold);
    assert_eq!(notice.status, AutoArchiveNoticeStatus::Pending);
    client
        .mock_all_auths()
        .record_auto_archive_delivery(&admin, &project_id, &true);
    assert_eq!(
        client.get_auto_archive_notice(project_id).unwrap().status,
        AutoArchiveNoticeStatus::Delivered
    );

    set_time(&env, registered_at + threshold - 1);
    let early = client
        .mock_all_auths()
        .archive_inactive_projects(&keeper, &1, &10)
        .unwrap();
    assert_eq!(early.archived_count, 0);

    set_time(&env, registered_at + threshold);
    let archived = client
        .mock_all_auths()
        .archive_inactive_projects(&keeper, &1, &10)
        .unwrap();
    assert_eq!(archived.archived_count, 1);
    assert_eq!(archived.archived_project_ids.get(0), Some(project_id));
    assert!(!archived.has_more);
    assert!(client.get_project(project_id).unwrap().archived);
    assert_eq!(client.list_projects(&1, &10).len(), 0);

    let search = client
        .search_archived_projects(&String::from_str(&env, "LEGACY"), &0, &10)
        .unwrap();
    assert_eq!(search.projects.len(), 1);
    assert_eq!(search.projects.get(0).unwrap().id, project_id);
    assert!(!search.has_more);
    assert!(client
        .get_project_by_name(&String::from_str(&env, "LegacyProject"))
        .is_some());

    let record = client.get_auto_archive_record(project_id).unwrap();
    assert_eq!(record.owner, owner);
    assert_eq!(record.archived_by, keeper);
    assert!(record.restored_at.is_none());
    assert_eq!(
        client
            .mock_all_auths()
            .try_restore_auto_archived_project(&project_id, &stranger),
        Err(Ok(ContractError::Unauthorized))
    );
    let restored = client
        .mock_all_auths()
        .restore_auto_archived_project(&project_id, &owner)
        .unwrap();
    assert!(!restored.archived);
    assert!(client.get_auto_archive_notice(project_id).is_none());
    assert!(client
        .get_auto_archive_record(project_id)
        .unwrap()
        .restored_at
        .is_some());
}

#[test]
fn project_update_invalidates_existing_notice() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let keeper = Address::generate(&env);
    let project_id = register_project(&client, &env, &owner, "ActiveAgain");
    let registered_at = client.get_project(project_id).unwrap().updated_at;
    let threshold = 100u64 * DAY;
    client.mock_all_auths().set_auto_archive_config(
        &admin,
        &AutoArchiveConfig {
            enabled: true,
            inactivity_threshold_secs: threshold,
        },
    );

    set_time(&env, registered_at + 70 * DAY);
    client
        .mock_all_auths()
        .notify_project_auto_archive(&keeper, &project_id);
    set_time(&env, registered_at + 80 * DAY);
    client.mock_all_auths().set_project_lifecycle_status(
        &project_id,
        &owner,
        &ProjectLifecycleStatus::Beta,
    );
    let project = client.get_project(project_id).unwrap();
    let stale_notice = client.get_auto_archive_notice(project_id).unwrap();
    assert_ne!(stale_notice.last_activity_at, project.updated_at);

    set_time(&env, registered_at + 180 * DAY);
    let result = client
        .mock_all_auths()
        .archive_inactive_projects(&keeper, &1, &10)
        .unwrap();
    assert_eq!(result.archived_count, 0);
    assert!(!client.get_project(project_id).unwrap().archived);
}
