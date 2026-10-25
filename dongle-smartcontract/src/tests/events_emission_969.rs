//! Issue #969 — Verify all operations emit correct events.
//!
//! Each test drives a mutating operation and then inspects
//! `env.events().all()` to confirm:
//!
//! * The event was emitted at all (operation coverage).
//! * The event payload fields match the operation's inputs/outputs
//!   (data accuracy).
//! * The event topics carry the expected symbolic tags so an indexer
//!   can filter by action type (topic consistency).
//! * Events appear in the expected order when multiple are produced
//!   by a single call (ordering).

use crate::events::{
    AdminAddedEvent, AdminRemovedEvent, ProjectArchivedEvent, ProjectReactivatedEvent,
    ProjectRegisteredEvent, ProjectUpdatedEvent, ReviewDeletedByAdminEvent, ReviewHiddenEvent,
    ReviewRestoredEvent, VerificationApprovedEvent, VerificationRejectedEvent,
    VerificationRequestedEvent,
};
use crate::types::ProjectRegistrationParams;
use crate::{DongleContract, DongleContractClient};
use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, Events, Ledger, LedgerInfo},
    Address, Env, IntoVal, String, TryFromVal, TryIntoVal, Val, Vec,
};

// ── Helpers ───────────────────────────────────────────────────────────────────

const TEST_TS: u64 = 1_750_000_000;

fn setup(env: &Env) -> (DongleContractClient<'_>, Address) {
    env.ledger().set(LedgerInfo {
        timestamp: TEST_TS,
        protocol_version: 22,
        sequence_number: 1,
        network_id: Default::default(),
        base_reserve: 10,
        min_temp_entry_ttl: 16,
        min_persistent_entry_ttl: 4096,
        max_entry_ttl: 6_312_000,
    });

    let contract_id = env.register(DongleContract, ());
    let client = DongleContractClient::new(env, &contract_id);
    let admin = Address::generate(env);
    client.mock_all_auths().initialize(&admin);
    (client, admin)
}

fn register_project(client: &DongleContractClient<'_>, env: &Env, owner: &Address, name: &str) -> u64 {
    let slug = name.to_lowercase().replace(' ', "-");
    client
        .mock_all_auths()
        .register_project(&ProjectRegistrationParams {
            owner: owner.clone(),
            name: String::from_str(env, name),
            slug: String::from_str(env, &slug),
            description: String::from_str(env, "Test description"),
            category: String::from_str(env, "DeFi"),
            website: None,
            license: None,
            logo_cid: None,
            metadata_cid: None,
            tags: None,
            social_links: None,
            launch_timestamp: None,
            bounty_url: None,
            repository_url: None,
            language_code: None,
            verification_requirements: None,
        })
}

/// Try to deserialise an event payload as `T` and pass it through `predicate`.
fn has_event<T, Topics, F>(env: &Env, expected_topics: Topics, predicate: F) -> bool
where
    T: soroban_sdk::TryFromVal<Env, Val>,
    Topics: IntoVal<Env, Vec<Val>>,
    F: Fn(T) -> bool,
{
    let expected_topics_val = expected_topics.into_val(env);
    let expected_topics_vec: soroban_sdk::Vec<Val> =
        soroban_sdk::Vec::try_from_val(env, &expected_topics_val).unwrap();
    env.events().all().iter().any(|(_, topics, data)| {
        topics == expected_topics_vec
            && TryIntoVal::<_, T>::try_into_val(&data, env)
                .map(&predicate)
                .unwrap_or(false)
    })
}

/// Return the number of events whose topics match.
fn count_events_with_topics<Topics>(env: &Env, expected_topics: Topics) -> usize
where
    Topics: IntoVal<Env, Vec<Val>>,
{
    let expected_topics_val = expected_topics.into_val(env);
    let expected_topics_vec: soroban_sdk::Vec<Val> =
        soroban_sdk::Vec::try_from_val(env, &expected_topics_val).unwrap();
    env.events()
        .all()
        .iter()
        .filter(|(_, topics, _)| topics == &expected_topics_vec)
        .count()
}

// ── #1 — register_project emits ProjectRegisteredEvent ───────────────────────

#[test]
fn event_register_project_emits_correct_topics_and_payload() {
    let env = Env::default();
    let (client, _) = setup(&env);
    let owner = Address::generate(&env);

    let project_id = register_project(&client, &env, &owner, "Emit969Reg");

    assert!(
        has_event::<ProjectRegisteredEvent, _, _>(
            &env,
            (
                symbol_short!("PROJECT"),
                symbol_short!("CREATED"),
                project_id,
            ),
            |evt| {
                evt.project_id == project_id
                    && evt.owner == owner
                    && evt.name == String::from_str(&env, "Emit969Reg")
                    && evt.timestamp == TEST_TS
            },
        ),
        "ProjectRegisteredEvent must be emitted with correct topics and payload"
    );
}

// ── #2 — update_project emits ProjectUpdatedEvent ────────────────────────────

#[test]
fn event_update_project_emits_correct_topics_and_payload() {
    let env = Env::default();
    let (client, _) = setup(&env);
    let owner = Address::generate(&env);
    let project_id = register_project(&client, &env, &owner, "Emit969Upd");

    client.mock_all_auths().update_project(&crate::types::ProjectUpdateParams {
        project_id,
        caller: owner.clone(),
        name: Some(String::from_str(&env, "Emit969UpdNew")),
        slug: None,
        description: None,
        category: None,
        website: None,
        license: None,
        logo_cid: None,
        metadata_cid: None,
        tags: None,
        social_links: None,
        launch_timestamp: None,
        bounty_url: None,
        repository_url: None,
        language_code: None,
        verification_requirements: None,
    });

    assert!(
        has_event::<ProjectUpdatedEvent, _, _>(
            &env,
            (
                symbol_short!("PROJECT"),
                symbol_short!("UPDATED"),
                project_id,
            ),
            |evt| evt.project_id == project_id && evt.owner == owner && evt.timestamp == TEST_TS,
        ),
        "ProjectUpdatedEvent must be emitted with correct topics and payload after update_project"
    );
}

// ── #3 — archive_project emits ProjectArchivedEvent ──────────────────────────

#[test]
fn event_archive_project_emits_correct_topics_and_payload() {
    let env = Env::default();
    let (client, _) = setup(&env);
    let owner = Address::generate(&env);
    let project_id = register_project(&client, &env, &owner, "Emit969Arc");

    client.mock_all_auths().archive_project(&project_id, &owner);

    assert!(
        has_event::<ProjectArchivedEvent, _, _>(
            &env,
            (
                symbol_short!("PROJECT"),
                symbol_short!("ARCHIVED"),
                project_id,
            ),
            |evt| {
                evt.project_id == project_id
                    && evt.archived_by == owner
                    && evt.timestamp == TEST_TS
            },
        ),
        "ProjectArchivedEvent must be emitted with correct topics and payload"
    );
}

// ── #4 — reactivate_project emits ProjectReactivatedEvent ────────────────────

#[test]
fn event_reactivate_project_emits_correct_topics_and_payload() {
    let env = Env::default();
    let (client, _) = setup(&env);
    let owner = Address::generate(&env);
    let project_id = register_project(&client, &env, &owner, "Emit969Reac");

    client.mock_all_auths().archive_project(&project_id, &owner);
    client
        .mock_all_auths()
        .reactivate_project(&project_id, &owner);

    assert!(
        has_event::<ProjectReactivatedEvent, _, _>(
            &env,
            (
                symbol_short!("PROJECT"),
                symbol_short!("RESTORED"),
                project_id,
            ),
            |evt| {
                evt.project_id == project_id
                    && evt.caller == owner
                    && evt.timestamp == TEST_TS
            },
        ),
        "ProjectReactivatedEvent must be emitted with correct topics and payload"
    );
}

// ── #5 — add_admin emits AdminAddedEvent ─────────────────────────────────────

#[test]
fn event_add_admin_emits_correct_topics_and_payload() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let new_admin = Address::generate(&env);

    client.mock_all_auths().add_admin(&admin, &new_admin);

    assert!(
        has_event::<AdminAddedEvent, _, _>(
            &env,
            (symbol_short!("ADMIN"), symbol_short!("ADDED")),
            |evt| evt.admin == new_admin && evt.timestamp == TEST_TS,
        ),
        "AdminAddedEvent must be emitted when add_admin is called"
    );
}

// ── #6 — remove_admin emits AdminRemovedEvent ────────────────────────────────

#[test]
fn event_remove_admin_emits_correct_topics_and_payload() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let second_admin = Address::generate(&env);

    client.mock_all_auths().add_admin(&admin, &second_admin);
    client.mock_all_auths().remove_admin(&admin, &second_admin);

    assert!(
        has_event::<AdminRemovedEvent, _, _>(
            &env,
            (symbol_short!("ADMIN"), symbol_short!("REMOVED")),
            |evt| evt.admin == second_admin && evt.timestamp == TEST_TS,
        ),
        "AdminRemovedEvent must be emitted when remove_admin is called"
    );
}

// ── #7 — request_verification emits VerificationRequestedEvent ───────────────

#[test]
fn event_request_verification_emits_correct_topics_and_payload() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let owner = Address::generate(&env);
    let project_id = register_project(&client, &env, &owner, "Emit969VerReq");

    client.mock_all_auths().set_min_project_age(&admin, &0);

    let evidence = String::from_str(&env, "QmEvidence969VerReqTest123456789012345678901234");
    client
        .mock_all_auths()
        .request_verification(&project_id, &owner, &evidence);

    assert!(
        has_event::<VerificationRequestedEvent, _, _>(
            &env,
            (
                symbol_short!("VERIFY"),
                symbol_short!("REQ"),
                project_id,
            ),
            |evt| {
                evt.project_id == project_id
                    && evt.requester == owner
                    && evt.evidence_cid == evidence
                    && evt.timestamp == TEST_TS
            },
        ),
        "VerificationRequestedEvent must be emitted with correct payload"
    );
}

// ── #8 — approve_verification emits VerificationApprovedEvent ────────────────

#[test]
fn event_approve_verification_emits_correct_topics_and_payload() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let owner = Address::generate(&env);
    let project_id = register_project(&client, &env, &owner, "Emit969VerApp");

    client.mock_all_auths().set_min_project_age(&admin, &0);

    let evidence = String::from_str(&env, "QmEvidence969VerAppTest123456789012345678901234");
    client
        .mock_all_auths()
        .request_verification(&project_id, &owner, &evidence);
    client
        .mock_all_auths()
        .approve_verification(&project_id, &admin);

    assert!(
        has_event::<VerificationApprovedEvent, _, _>(
            &env,
            (
                symbol_short!("VERIFY"),
                symbol_short!("APP"),
                project_id,
            ),
            |evt| {
                evt.project_id == project_id
                    && evt.admin == admin
                    && evt.timestamp == TEST_TS
            },
        ),
        "VerificationApprovedEvent must be emitted after approve_verification"
    );
}

// ── #9 — reject_verification emits VerificationRejectedEvent ─────────────────

#[test]
fn event_reject_verification_emits_correct_topics_and_payload() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let owner = Address::generate(&env);
    let project_id = register_project(&client, &env, &owner, "Emit969VerRej");

    client.mock_all_auths().set_min_project_age(&admin, &0);

    let evidence = String::from_str(&env, "QmEvidence969VerRejTest123456789012345678901234");
    client
        .mock_all_auths()
        .request_verification(&project_id, &owner, &evidence);
    client
        .mock_all_auths()
        .reject_verification(&project_id, &admin);

    assert!(
        has_event::<VerificationRejectedEvent, _, _>(
            &env,
            (
                symbol_short!("VERIFY"),
                symbol_short!("REJ"),
                project_id,
            ),
            |evt| {
                evt.project_id == project_id
                    && evt.admin == admin
                    && evt.timestamp == TEST_TS
            },
        ),
        "VerificationRejectedEvent must be emitted after reject_verification"
    );
}

// ── #10 — hide_review emits ReviewHiddenEvent ─────────────────────────────────

#[test]
fn event_hide_review_emits_correct_topics_and_payload() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let owner = Address::generate(&env);
    let reviewer = Address::generate(&env);
    let project_id = register_project(&client, &env, &owner, "Emit969HideRev");

    client
        .mock_all_auths()
        .add_review(&project_id, &reviewer, &3, &None);
    client
        .mock_all_auths()
        .hide_review(&project_id, &reviewer, &admin);

    assert!(
        has_event::<ReviewHiddenEvent, _, _>(
            &env,
            (
                symbol_short!("REVIEW"),
                symbol_short!("HIDDEN"),
                project_id,
            ),
            |evt| {
                evt.project_id == project_id
                    && evt.reviewer == reviewer
                    && evt.admin == admin
                    && evt.timestamp == TEST_TS
            },
        ),
        "ReviewHiddenEvent must be emitted after hide_review"
    );
}

// ── #11 — restore_review emits ReviewRestoredEvent ───────────────────────────

#[test]
fn event_restore_review_emits_correct_topics_and_payload() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let owner = Address::generate(&env);
    let reviewer = Address::generate(&env);
    let project_id = register_project(&client, &env, &owner, "Emit969RestoreRev");

    client
        .mock_all_auths()
        .add_review(&project_id, &reviewer, &3, &None);
    client
        .mock_all_auths()
        .hide_review(&project_id, &reviewer, &admin);
    client
        .mock_all_auths()
        .restore_review(&project_id, &reviewer, &admin);

    assert!(
        has_event::<ReviewRestoredEvent, _, _>(
            &env,
            (
                symbol_short!("REVIEW"),
                symbol_short!("RESTORED"),
                project_id,
            ),
            |evt| {
                evt.project_id == project_id
                    && evt.reviewer == reviewer
                    && evt.admin == admin
                    && evt.timestamp == TEST_TS
            },
        ),
        "ReviewRestoredEvent must be emitted after restore_review"
    );
}

// ── #12 — delete_review (admin) emits ReviewDeletedByAdminEvent ──────────────

#[test]
fn event_admin_delete_review_emits_correct_topics_and_payload() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let owner = Address::generate(&env);
    let reviewer = Address::generate(&env);
    let project_id = register_project(&client, &env, &owner, "Emit969AdminDel");

    client
        .mock_all_auths()
        .add_review(&project_id, &reviewer, &2, &None);
    client
        .mock_all_auths()
        .admin_delete_review(&project_id, &reviewer, &admin);

    assert!(
        has_event::<ReviewDeletedByAdminEvent, _, _>(
            &env,
            (
                symbol_short!("REVIEW"),
                symbol_short!("ADMINDEL"),
                project_id,
            ),
            |evt| {
                evt.project_id == project_id
                    && evt.reviewer == reviewer
                    && evt.admin == admin
                    && evt.timestamp == TEST_TS
            },
        ),
        "ReviewDeletedByAdminEvent must be emitted when an admin deletes a review"
    );
}

// ── #13 — Event ordering: register then update emits events in order ─────────

#[test]
fn event_ordering_register_then_update_events_appear_in_call_order() {
    let env = Env::default();
    let (client, _) = setup(&env);
    let owner = Address::generate(&env);

    let project_id = register_project(&client, &env, &owner, "Emit969Order");

    client.mock_all_auths().update_project(&crate::types::ProjectUpdateParams {
        project_id,
        caller: owner.clone(),
        name: Some(String::from_str(&env, "Emit969OrderUpdated")),
        slug: None,
        description: None,
        category: None,
        website: None,
        license: None,
        logo_cid: None,
        metadata_cid: None,
        tags: None,
        social_links: None,
        launch_timestamp: None,
        bounty_url: None,
        repository_url: None,
        language_code: None,
        verification_requirements: None,
    });

    // Walk all events and confirm REG appears before UPDATED for this project.
    let all = env.events().all();
    let reg_pos = all.iter().position(|(_, topics, data)| {
        let topics_match = topics
            == soroban_sdk::Vec::try_from_val(
                &env,
                &(
                    symbol_short!("PROJECT"),
                    symbol_short!("CREATED"),
                    project_id,
                )
                    .into_val(&env),
            )
            .unwrap();
        topics_match
            && TryIntoVal::<_, ProjectRegisteredEvent>::try_into_val(&data, &env).is_ok()
    });

    let upd_pos = all.iter().position(|(_, topics, data)| {
        let topics_match = topics
            == soroban_sdk::Vec::try_from_val(
                &env,
                &(
                    symbol_short!("PROJECT"),
                    symbol_short!("UPDATED"),
                    project_id,
                )
                    .into_val(&env),
            )
            .unwrap();
        topics_match
            && TryIntoVal::<_, ProjectUpdatedEvent>::try_into_val(&data, &env).is_ok()
    });

    assert!(reg_pos.is_some(), "ProjectRegisteredEvent must exist");
    assert!(upd_pos.is_some(), "ProjectUpdatedEvent must exist");
    assert!(
        reg_pos.unwrap() < upd_pos.unwrap(),
        "ProjectRegisteredEvent must appear before ProjectUpdatedEvent in the event log"
    );
}

// ── #14 — Event filtering: topics uniquely identify action type ───────────────

#[test]
fn event_topics_uniquely_identify_action_type_for_filtered_retrieval() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let owner = Address::generate(&env);

    let pid1 = register_project(&client, &env, &owner, "FilterEvt969A");
    let pid2 = register_project(&client, &env, &owner, "FilterEvt969B");

    client.mock_all_auths().archive_project(&pid1, &owner);

    // Only pid1 should have an ARCHIVED event; pid2 must have none.
    let archived_count_pid1 = count_events_with_topics(
        &env,
        (symbol_short!("PROJECT"), symbol_short!("ARCHIVED"), pid1),
    );
    let archived_count_pid2 = count_events_with_topics(
        &env,
        (symbol_short!("PROJECT"), symbol_short!("ARCHIVED"), pid2),
    );

    assert_eq!(
        archived_count_pid1, 1,
        "exactly one ARCHIVED event must exist for pid1"
    );
    assert_eq!(
        archived_count_pid2, 0,
        "no ARCHIVED event must exist for pid2 which was never archived"
    );

    // Both must have exactly one REG event each.
    let reg_count_pid1 = count_events_with_topics(
        &env,
        (symbol_short!("PROJECT"), symbol_short!("CREATED"), pid1),
    );
    let reg_count_pid2 = count_events_with_topics(
        &env,
        (symbol_short!("PROJECT"), symbol_short!("CREATED"), pid2),
    );

    assert_eq!(reg_count_pid1, 1, "pid1 must have exactly one REG event");
    assert_eq!(reg_count_pid2, 1, "pid2 must have exactly one REG event");
}

// ── #15 — Event data accuracy: registration payload matches input ─────────────

#[test]
fn event_registration_payload_fields_match_registration_input_exactly() {
    let env = Env::default();
    let (client, _) = setup(&env);
    let owner = Address::generate(&env);

    let name = "Accuracy969Test";
    let category = "DeFi";
    let project_id = register_project(&client, &env, &owner, name);

    assert!(
        has_event::<ProjectRegisteredEvent, _, _>(
            &env,
            (
                symbol_short!("PROJECT"),
                symbol_short!("CREATED"),
                project_id,
            ),
            |evt| {
                evt.project_id == project_id
                    && evt.owner == owner
                    && evt.name == String::from_str(&env, name)
                    && evt.category == String::from_str(&env, category)
                    && evt.timestamp == TEST_TS
            },
        ),
        "all ProjectRegisteredEvent payload fields must match registration inputs"
    );
}

// ── #16 — Verification request/approve event pair for ordering ────────────────

#[test]
fn event_ordering_verification_request_before_approval() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let owner = Address::generate(&env);
    let project_id = register_project(&client, &env, &owner, "Emit969VerOrder");

    client.mock_all_auths().set_min_project_age(&admin, &0);
    let evidence = String::from_str(&env, "QmEvidence969VerOrderTest123456789012345678901");

    client
        .mock_all_auths()
        .request_verification(&project_id, &owner, &evidence);
    client
        .mock_all_auths()
        .approve_verification(&project_id, &admin);

    let all = env.events().all();

    let req_pos = all.iter().position(|(_, topics, _)| {
        topics
            == soroban_sdk::Vec::try_from_val(
                &env,
                &(
                    symbol_short!("VERIFY"),
                    symbol_short!("REQ"),
                    project_id,
                )
                    .into_val(&env),
            )
            .unwrap()
    });

    let app_pos = all.iter().position(|(_, topics, _)| {
        topics
            == soroban_sdk::Vec::try_from_val(
                &env,
                &(
                    symbol_short!("VERIFY"),
                    symbol_short!("APP"),
                    project_id,
                )
                    .into_val(&env),
            )
            .unwrap()
    });

    assert!(req_pos.is_some(), "VerificationRequestedEvent must exist");
    assert!(app_pos.is_some(), "VerificationApprovedEvent must exist");
    assert!(
        req_pos.unwrap() < app_pos.unwrap(),
        "VerificationRequestedEvent must precede VerificationApprovedEvent"
    );
}

// ── #17 — No spurious events emitted on a read-only call ─────────────────────

/// Calling a read-only function must not produce any new events.
/// \`get_project\` is used as the representative read-only call.
#[test]
fn event_read_only_call_emits_no_events() {
    let env = Env::default();
    let (client, _) = setup(&env);
    let owner = Address::generate(&env);
    let project_id = register_project(&client, &env, &owner, "NoEventRead969");

    let before = env.events().all().len();
    let _ = client.get_project(&project_id);
    let after = env.events().all().len();

    assert_eq!(
        before, after,
        "get_project must not emit any events"
    );
}

// ── #18 — Each registration emits exactly one ProjectRegisteredEvent ──────────

/// Registering two different projects must emit exactly two
/// ProjectRegisteredEvent entries — one per registration, no duplicates.
#[test]
fn event_each_registration_emits_exactly_one_event() {
    let env = Env::default();
    let (client, _) = setup(&env);
    let owner = Address::generate(&env);

    let pid1 = register_project(&client, &env, &owner, "ExactOne969A");
    let pid2 = register_project(&client, &env, &owner, "ExactOne969B");

    let count1 = count_events_with_topics(
        &env,
        (symbol_short!("PROJECT"), symbol_short!("CREATED"), pid1),
    );
    let count2 = count_events_with_topics(
        &env,
        (symbol_short!("PROJECT"), symbol_short!("CREATED"), pid2),
    );

    assert_eq!(count1, 1, "pid1 must have exactly one CREATED event");
    assert_eq!(count2, 1, "pid2 must have exactly one CREATED event");
}
