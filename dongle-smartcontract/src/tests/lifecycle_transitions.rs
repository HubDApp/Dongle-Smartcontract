//! N² lifecycle status transition matrix tests (#674).
//!
//! ## Valid Transition Matrix
//!
//! ```text
//!               │ Active │ Beta │ Paused │ Deprecated │ Sunset │
//! ──────────────┼────────┼──────┼────────┼────────────┼────────┤
//! Active        │   —    │  ✓   │   ✓    │     ✓      │   ✓    │
//! Beta          │   ✓    │  —   │   ✓    │     ✓      │   ✓    │
//! Paused        │   ✓    │  ✓   │   —    │     ✓      │   ✓    │
//! Deprecated    │   ✓    │  ✗   │   ✗    │     —      │   ✓    │
//! Sunset        │   ✓    │  ✗   │   ✗    │     ✗      │   —    │
//! ```
//!
//! `✓` = valid, `✗` = `InvalidStatusTransition`, `—` = same-state no-op.
//!
//! All 25 (5×5) combinations are tested below. Same-state cases are no-ops
//! at the contract level (handled before `validate_lifecycle_transition` is
//! called), but the validation function itself rejects them to prevent
//! bypasses.

use crate::errors::ContractError;
use crate::project_registry::ProjectRegistry;
use crate::tests::fixtures::{create_test_project, setup_contract};
use crate::types::ProjectLifecycleStatus;
use soroban_sdk::{testutils::Address as _, Address, Env};

// ─── Unit: validate_lifecycle_transition (all 25 combinations) ───────────

fn ok(from: ProjectLifecycleStatus, to: ProjectLifecycleStatus) {
    assert!(
        ProjectRegistry::validate_lifecycle_transition(from, to).is_ok(),
        "{:?} → {:?} should be valid",
        from,
        to
    );
}

fn err(from: ProjectLifecycleStatus, to: ProjectLifecycleStatus) {
    assert_eq!(
        ProjectRegistry::validate_lifecycle_transition(from, to),
        Err(ContractError::InvalidStatusTransition),
        "{:?} → {:?} should return InvalidStatusTransition",
        from,
        to
    );
}

// ── Row: Active ──────────────────────────────────────────────────────────
#[test] fn unit_active_same()       { err(ProjectLifecycleStatus::Active, ProjectLifecycleStatus::Active); }
#[test] fn unit_active_to_beta()    { ok(ProjectLifecycleStatus::Active, ProjectLifecycleStatus::Beta); }
#[test] fn unit_active_to_paused()  { ok(ProjectLifecycleStatus::Active, ProjectLifecycleStatus::Paused); }
#[test] fn unit_active_to_depr()    { ok(ProjectLifecycleStatus::Active, ProjectLifecycleStatus::Deprecated); }
#[test] fn unit_active_to_sunset()  { ok(ProjectLifecycleStatus::Active, ProjectLifecycleStatus::Sunset); }

// ── Row: Beta ────────────────────────────────────────────────────────────
#[test] fn unit_beta_to_active()    { ok(ProjectLifecycleStatus::Beta, ProjectLifecycleStatus::Active); }
#[test] fn unit_beta_same()         { err(ProjectLifecycleStatus::Beta, ProjectLifecycleStatus::Beta); }
#[test] fn unit_beta_to_paused()    { ok(ProjectLifecycleStatus::Beta, ProjectLifecycleStatus::Paused); }
#[test] fn unit_beta_to_depr()      { ok(ProjectLifecycleStatus::Beta, ProjectLifecycleStatus::Deprecated); }
#[test] fn unit_beta_to_sunset()    { ok(ProjectLifecycleStatus::Beta, ProjectLifecycleStatus::Sunset); }

// ── Row: Paused ──────────────────────────────────────────────────────────
#[test] fn unit_paused_to_active()  { ok(ProjectLifecycleStatus::Paused, ProjectLifecycleStatus::Active); }
#[test] fn unit_paused_to_beta()    { ok(ProjectLifecycleStatus::Paused, ProjectLifecycleStatus::Beta); }
#[test] fn unit_paused_same()       { err(ProjectLifecycleStatus::Paused, ProjectLifecycleStatus::Paused); }
#[test] fn unit_paused_to_depr()    { ok(ProjectLifecycleStatus::Paused, ProjectLifecycleStatus::Deprecated); }
#[test] fn unit_paused_to_sunset()  { ok(ProjectLifecycleStatus::Paused, ProjectLifecycleStatus::Sunset); }

// ── Row: Deprecated ──────────────────────────────────────────────────────
#[test] fn unit_depr_to_active()    { ok(ProjectLifecycleStatus::Deprecated, ProjectLifecycleStatus::Active); }
#[test] fn unit_depr_to_beta()      { err(ProjectLifecycleStatus::Deprecated, ProjectLifecycleStatus::Beta); }
#[test] fn unit_depr_to_paused()    { err(ProjectLifecycleStatus::Deprecated, ProjectLifecycleStatus::Paused); }
#[test] fn unit_depr_same()         { err(ProjectLifecycleStatus::Deprecated, ProjectLifecycleStatus::Deprecated); }
#[test] fn unit_depr_to_sunset()    { ok(ProjectLifecycleStatus::Deprecated, ProjectLifecycleStatus::Sunset); }

// ── Row: Sunset ──────────────────────────────────────────────────────────
#[test] fn unit_sunset_to_active()  { ok(ProjectLifecycleStatus::Sunset, ProjectLifecycleStatus::Active); }
#[test] fn unit_sunset_to_beta()    { err(ProjectLifecycleStatus::Sunset, ProjectLifecycleStatus::Beta); }
#[test] fn unit_sunset_to_paused()  { err(ProjectLifecycleStatus::Sunset, ProjectLifecycleStatus::Paused); }
#[test] fn unit_sunset_to_depr()    { err(ProjectLifecycleStatus::Sunset, ProjectLifecycleStatus::Deprecated); }
#[test] fn unit_sunset_same()       { err(ProjectLifecycleStatus::Sunset, ProjectLifecycleStatus::Sunset); }

// ─── Integration: invalid transitions via the contract client ────────────

#[test]
fn integration_deprecated_to_beta_blocked() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _) = setup_contract(&env);
    let owner = Address::generate(&env);
    let p = create_test_project(&client, &owner, "DeprToBeta");
    client.set_project_lifecycle_status(&p, &owner, &ProjectLifecycleStatus::Deprecated);
    assert_eq!(
        client.try_set_project_lifecycle_status(&p, &owner, &ProjectLifecycleStatus::Beta),
        Err(Ok(ContractError::InvalidStatusTransition.into()))
    );
}

#[test]
fn integration_deprecated_to_paused_blocked() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _) = setup_contract(&env);
    let owner = Address::generate(&env);
    let p = create_test_project(&client, &owner, "DeprToPaused");
    client.set_project_lifecycle_status(&p, &owner, &ProjectLifecycleStatus::Deprecated);
    assert_eq!(
        client.try_set_project_lifecycle_status(&p, &owner, &ProjectLifecycleStatus::Paused),
        Err(Ok(ContractError::InvalidStatusTransition.into()))
    );
}

#[test]
fn integration_sunset_to_beta_blocked() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _) = setup_contract(&env);
    let owner = Address::generate(&env);
    let p = create_test_project(&client, &owner, "SunsetToBeta");
    client.set_project_lifecycle_status(&p, &owner, &ProjectLifecycleStatus::Sunset);
    assert_eq!(
        client.try_set_project_lifecycle_status(&p, &owner, &ProjectLifecycleStatus::Beta),
        Err(Ok(ContractError::InvalidStatusTransition.into()))
    );
}

#[test]
fn integration_sunset_to_paused_blocked() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _) = setup_contract(&env);
    let owner = Address::generate(&env);
    let p = create_test_project(&client, &owner, "SunsetToPaused");
    client.set_project_lifecycle_status(&p, &owner, &ProjectLifecycleStatus::Sunset);
    assert_eq!(
        client.try_set_project_lifecycle_status(&p, &owner, &ProjectLifecycleStatus::Paused),
        Err(Ok(ContractError::InvalidStatusTransition.into()))
    );
}

#[test]
fn integration_sunset_to_deprecated_blocked() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _) = setup_contract(&env);
    let owner = Address::generate(&env);
    let p = create_test_project(&client, &owner, "SunsetToDepr");
    client.set_project_lifecycle_status(&p, &owner, &ProjectLifecycleStatus::Sunset);
    assert_eq!(
        client.try_set_project_lifecycle_status(&p, &owner, &ProjectLifecycleStatus::Deprecated),
        Err(Ok(ContractError::InvalidStatusTransition.into()))
    );
}

// ─── Integration: valid transitions via the contract client ──────────────

#[test]
fn integration_full_lifecycle_state_diagram() {
    // Active → Beta → Paused → Deprecated → Sunset → Active
    let env = Env::default();
    env.mock_all_auths();
    let (client, _) = setup_contract(&env);
    let owner = Address::generate(&env);
    let p = create_test_project(&client, &owner, "FullDiagram");

    let s = client.set_project_lifecycle_status(&p, &owner, &ProjectLifecycleStatus::Beta);
    assert_eq!(s.lifecycle_status, ProjectLifecycleStatus::Beta);

    let s = client.set_project_lifecycle_status(&p, &owner, &ProjectLifecycleStatus::Paused);
    assert_eq!(s.lifecycle_status, ProjectLifecycleStatus::Paused);

    let s = client.set_project_lifecycle_status(&p, &owner, &ProjectLifecycleStatus::Deprecated);
    assert_eq!(s.lifecycle_status, ProjectLifecycleStatus::Deprecated);

    let s = client.set_project_lifecycle_status(&p, &owner, &ProjectLifecycleStatus::Sunset);
    assert_eq!(s.lifecycle_status, ProjectLifecycleStatus::Sunset);

    let s = client.set_project_lifecycle_status(&p, &owner, &ProjectLifecycleStatus::Active);
    assert_eq!(s.lifecycle_status, ProjectLifecycleStatus::Active);
}

#[test]
fn integration_deprecated_revive_requires_active_first() {
    // Deprecated → Active → Beta (valid two-step revival)
    let env = Env::default();
    env.mock_all_auths();
    let (client, _) = setup_contract(&env);
    let owner = Address::generate(&env);
    let p = create_test_project(&client, &owner, "DeprecatedRevive");
    client.set_project_lifecycle_status(&p, &owner, &ProjectLifecycleStatus::Deprecated);

    // Direct to Beta blocked
    assert_eq!(
        client.try_set_project_lifecycle_status(&p, &owner, &ProjectLifecycleStatus::Beta),
        Err(Ok(ContractError::InvalidStatusTransition.into()))
    );

    // Via Active allowed
    client.set_project_lifecycle_status(&p, &owner, &ProjectLifecycleStatus::Active);
    let s = client.set_project_lifecycle_status(&p, &owner, &ProjectLifecycleStatus::Beta);
    assert_eq!(s.lifecycle_status, ProjectLifecycleStatus::Beta);
}

#[test]
fn integration_sunset_revive_requires_active_first() {
    // Sunset → Active → Paused (valid two-step revival)
    let env = Env::default();
    env.mock_all_auths();
    let (client, _) = setup_contract(&env);
    let owner = Address::generate(&env);
    let p = create_test_project(&client, &owner, "SunsetRevive");
    client.set_project_lifecycle_status(&p, &owner, &ProjectLifecycleStatus::Sunset);

    // Direct to Paused blocked
    assert_eq!(
        client.try_set_project_lifecycle_status(&p, &owner, &ProjectLifecycleStatus::Paused),
        Err(Ok(ContractError::InvalidStatusTransition.into()))
    );

    // Via Active allowed
    client.set_project_lifecycle_status(&p, &owner, &ProjectLifecycleStatus::Active);
    let s = client.set_project_lifecycle_status(&p, &owner, &ProjectLifecycleStatus::Paused);
    assert_eq!(s.lifecycle_status, ProjectLifecycleStatus::Paused);
}

#[test]
fn integration_same_status_is_noop_no_error() {
    // Same-status calls return Ok without modifying the project
    let env = Env::default();
    env.mock_all_auths();
    let (client, _) = setup_contract(&env);
    let owner = Address::generate(&env);
    let p = create_test_project(&client, &owner, "SameStatus");
    let before = client.get_project(&p).unwrap().updated_at;
    let result = client.set_project_lifecycle_status(&p, &owner, &ProjectLifecycleStatus::Active);
    assert_eq!(result.lifecycle_status, ProjectLifecycleStatus::Active);
    // updated_at must be unchanged (no mutation happened)
    assert_eq!(result.updated_at, before);
}
