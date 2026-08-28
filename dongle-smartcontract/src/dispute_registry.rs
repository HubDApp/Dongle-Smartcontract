//! # Duplicate Dispute State Machine (#654)
//!
//! Disputes follow a strict, one-way state machine:
//!
//! ```text
//!  ┌─────────────────────────────────────┐
//!  │                                     │
//!  │  open_duplicate_dispute             │
//!  │          │                          │
//!  │          ▼                          │
//!  │       Pending ──────────────────┐   │
//!  │          │                      │   │
//!  │   (admin resolves)        (admin rejects)
//!  │          │                      │   │
//!  │          ▼                      ▼   │
//!  │       Resolved             Rejected │
//!  │   (terminal state)     (terminal state)
//!  │                                     │
//!  └─────────────────────────────────────┘
//! ```
//!
//! ## Valid transitions
//!
//! | From    | To       | Trigger                        |
//! |---------|----------|--------------------------------|
//! | Pending | Resolved | `resolve_duplicate_dispute` with `ArchiveProject` or `LinkDuplicates` |
//! | Pending | Rejected | `resolve_duplicate_dispute` with `Reject` |
//!
//! ## Invalid transitions (return `DisputeNotPending`)
//!
//! | From     | Attempted | Error            |
//! |----------|-----------|------------------|
//! | Resolved | Resolve   | `DisputeNotPending` |
//! | Resolved | Reject    | `DisputeNotPending` |
//! | Rejected | Resolve   | `DisputeNotPending` |
//! | Rejected | Reject    | `DisputeNotPending` |
//!
//! Both `Resolved` and `Rejected` are **terminal states** — once reached no
//! further transitions are possible. A new dispute on the same project pair
//! requires calling `open_duplicate_dispute` again, which creates a fresh
//! dispute with a new ID.

use crate::admin_action_log::AdminActionLog;
use crate::admin_manager::AdminManager;
use crate::errors::ContractError;
use crate::events::{
    publish_duplicate_dispute_opened_event, publish_duplicate_dispute_resolved_event,
};
use crate::project_registry::ProjectRegistry;
use crate::storage_keys::ExtensionKey;
use crate::storage_manager::StorageManager;
use crate::types::{AdminActionType, DisputeResolutionAction, DisputeStatus, DuplicateDispute};
use crate::utils::Utils;
use soroban_sdk::{Address, Env, String, Vec};

pub struct DisputeRegistry;

impl DisputeRegistry {
    pub fn open_duplicate_dispute(
        env: &Env,
        project_id: u64,
        original_project_id: u64,
        creator: Address,
        evidence_cid: String,
    ) -> Result<u64, ContractError> {
        creator.require_auth();

        if project_id == original_project_id {
            return Err(ContractError::CannotLinkToSelf);
        }

        // Verify both projects exist
        let project =
            ProjectRegistry::get_project(env, project_id).ok_or(ContractError::ProjectNotFound)?;
        let _original_project = ProjectRegistry::get_project(env, original_project_id)
            .ok_or(ContractError::ProjectNotFound)?;

        if project.archived {
            return Err(ContractError::AlreadyArchived);
        }

        // Validate evidence CID
        if evidence_cid.is_empty() {
            return Err(ContractError::InvalidProjectData);
        }
        if !Utils::is_valid_ipfs_cid(&evidence_cid) {
            return Err(ContractError::InvalidProjectData);
        }

        // Generate next dispute ID
        let dispute_id: u64 = env
            .storage()
            .persistent()
            .get(&ExtensionKey::NextDuplicateDisputeId)
            .unwrap_or(1);

        let now = env.ledger().timestamp();
        let dispute = DuplicateDispute {
            id: dispute_id,
            project_id,
            original_project_id,
            creator: creator.clone(),
            evidence_cid: evidence_cid.clone(),
            status: DisputeStatus::Pending,
            created_at: now,
            resolved_at: 0,
        };

        // Store dispute
        env.storage()
            .persistent()
            .set(&ExtensionKey::DuplicateDispute(dispute_id), &dispute);

        // Add to project's disputes list
        let mut project_disputes: Vec<u64> = env
            .storage()
            .persistent()
            .get(&ExtensionKey::ProjectDuplicateDisputes(project_id))
            .unwrap_or_else(|| Vec::new(env));
        project_disputes.push_back(dispute_id);
        env.storage().persistent().set(
            &ExtensionKey::ProjectDuplicateDisputes(project_id),
            &project_disputes,
        );

        // Increment ID
        let next_id = dispute_id.saturating_add(1);
        env.storage()
            .persistent()
            .set(&ExtensionKey::NextDuplicateDisputeId, &next_id);

        // Extend TTL
        StorageManager::extend_project_ttl(env, project_id);

        if env
            .storage()
            .persistent()
            .has(&ExtensionKey::DuplicateDispute(dispute_id))
        {
            env.storage().persistent().extend_ttl(
                &ExtensionKey::DuplicateDispute(dispute_id),
                crate::constants::LEDGER_THRESHOLD_PROJECT,
                crate::constants::LEDGER_BUMP_PROJECT,
            );
        }

        publish_duplicate_dispute_opened_event(
            env,
            dispute_id,
            project_id,
            original_project_id,
            creator,
            evidence_cid,
        );

        Ok(dispute_id)
    }

    /// Resolve or reject a pending duplicate dispute (admin only).
    ///
    /// # State machine (#654)
    ///
    /// This function drives the dispute from `Pending` to either `Resolved` or
    /// `Rejected` (see module-level state machine diagram). Calling this on a
    /// dispute that is already in a terminal state (`Resolved` or `Rejected`)
    /// returns [`ContractError::DisputeNotPending`] immediately — no storage is
    /// mutated and no event is emitted.
    ///
    /// # Actions
    ///
    /// - `DisputeResolutionAction::Reject` → status becomes `Rejected`
    /// - `DisputeResolutionAction::ArchiveProject(id)` → archives the project and
    ///   status becomes `Resolved`
    /// - `DisputeResolutionAction::LinkDuplicates` → links the two projects and
    ///   status becomes `Resolved`
    pub fn resolve_duplicate_dispute(
        env: &Env,
        dispute_id: u64,
        admin: Address,
        action: DisputeResolutionAction,
    ) -> Result<(), ContractError> {
        admin.require_auth();
        if !AdminManager::is_admin(env, &admin) {
            return Err(ContractError::AdminOnly);
        }

        let mut dispute: DuplicateDispute = env
            .storage()
            .persistent()
            .get(&ExtensionKey::DuplicateDispute(dispute_id))
            .ok_or(ContractError::ProjectNotFound)?;

        // State machine guard: only Pending → Resolved/Rejected is a valid transition.
        // Resolved and Rejected are terminal states. Attempting to re-resolve or
        // re-reject them returns DisputeNotPending (not the generic InvalidStatus)
        // so callers can distinguish "wrong kind of error" from "dispute not found".
        if dispute.status != DisputeStatus::Pending {
            return Err(ContractError::DisputeNotPending);
        }

        let now = env.ledger().timestamp();

        match action {
            DisputeResolutionAction::Reject => {
                dispute.status = DisputeStatus::Rejected;
                dispute.resolved_at = now;
                env.storage()
                    .persistent()
                    .set(&ExtensionKey::DuplicateDispute(dispute_id), &dispute);

                AdminActionLog::record_action(
                    env,
                    admin.clone(),
                    AdminActionType::DuplicateDisputeRejected,
                    Some(dispute.project_id),
                    None,
                    None,
                );
            }
            DisputeResolutionAction::ArchiveProject(project_to_archive) => {
                if project_to_archive != dispute.project_id
                    && project_to_archive != dispute.original_project_id
                {
                    return Err(ContractError::ProjectNotFound);
                }

                // Archive the project
                ProjectRegistry::archive_project_unauthorized(
                    env,
                    project_to_archive,
                    admin.clone(),
                )?;

                dispute.status = DisputeStatus::Resolved;
                dispute.resolved_at = now;
                env.storage()
                    .persistent()
                    .set(&ExtensionKey::DuplicateDispute(dispute_id), &dispute);

                AdminActionLog::record_action(
                    env,
                    admin.clone(),
                    AdminActionType::DuplicateDisputeResolved,
                    Some(dispute.project_id),
                    None,
                    None,
                );
            }
            DisputeResolutionAction::LinkDuplicates => {
                // Link the projects
                ProjectRegistry::link_project_unauthorized(
                    env,
                    dispute.project_id,
                    admin.clone(),
                    dispute.original_project_id,
                )?;

                dispute.status = DisputeStatus::Resolved;
                dispute.resolved_at = now;
                env.storage()
                    .persistent()
                    .set(&ExtensionKey::DuplicateDispute(dispute_id), &dispute);

                AdminActionLog::record_action(
                    env,
                    admin.clone(),
                    AdminActionType::DuplicateDisputeResolved,
                    Some(dispute.project_id),
                    None,
                    None,
                );
            }
        }

        publish_duplicate_dispute_resolved_event(env, dispute_id, admin, action);
        Ok(())
    }

    pub fn get_duplicate_dispute(env: &Env, dispute_id: u64) -> Option<DuplicateDispute> {
        env.storage()
            .persistent()
            .get(&ExtensionKey::DuplicateDispute(dispute_id))
    }

    pub fn get_disputes_for_project(env: &Env, project_id: u64) -> Vec<DuplicateDispute> {
        let mut disputes = Vec::new(env);
        if let Some(dispute_ids) = env
            .storage()
            .persistent()
            .get::<_, Vec<u64>>(&ExtensionKey::ProjectDuplicateDisputes(project_id))
        {
            for i in 0..dispute_ids.len() {
                if let Some(id) = dispute_ids.get(i) {
                    if let Some(dispute) = Self::get_duplicate_dispute(env, id) {
                        disputes.push_back(dispute);
                    }
                }
            }
        }
        disputes
    }
}
