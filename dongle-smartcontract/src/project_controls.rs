//! Project controls: per-project ACLs (#766), collaboration tools (#767),
//! field-level encrypted metadata (#765), and project quarantine (#764).
//!
//! All state lives in the [`ProjectControlsKey`] namespace so the existing
//! `StorageKey` / `ExtensionKey` enums (both at Soroban's 50-variant cap)
//! are untouched.

use crate::admin_manager::AdminManager;
use crate::errors::ContractError;
use crate::project_registry::ProjectRegistry;
use crate::storage_keys::ProjectControlsKey;
use crate::types::Project;
use soroban_sdk::{contracttype, symbol_short, Address, Env, String, Vec};

// ── Types (#766: ACLs) ─────────────────────────────────────────────────────────

/// Who is allowed to interact with a project.
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AccessMode {
    /// Everyone can view.
    Public,
    /// Only the owner (and admins) can access.
    Private,
    /// Only addresses on the whitelist can access.
    Custom,
}

/// Granular permission granted to a whitelisted address.
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AclPermission {
    View,
    Edit,
    Admin,
}

/// Kind of access change recorded in the history log.
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AclAction {
    ModeSet,
    AddressAdded,
    AddressRemoved,
    PermissionChanged,
}

/// One whitelist entry.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AclEntry {
    pub address: Address,
    pub permission: AclPermission,
}

/// Full ACL record for a project.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectAcl {
    pub project_id: u64,
    pub mode: AccessMode,
    pub entries: Vec<AclEntry>,
}

/// Immutable audit record of an access change (#766 change history).
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AclChangeRecord {
    pub project_id: u64,
    pub actor: Address,
    pub target: Option<Address>,
    pub action: AclAction,
    pub mode: Option<AccessMode>,
    pub permission: Option<AclPermission>,
    pub timestamp: u64,
}

// ── Types (#767: collaboration) ────────────────────────────────────────────────

/// Role of a collaborator on a project.
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CollaboratorRole {
    Viewer,
    Editor,
    Maintainer,
}

/// A collaborator attached to a project.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Collaborator {
    pub project_id: u64,
    pub address: Address,
    pub role: CollaboratorRole,
    pub added_at: u64,
}

/// Lifecycle of a change proposal.
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProposalStatus {
    Pending,
    Approved,
    Rejected,
}

/// A proposed change awaiting approval (#767 approval workflow).
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChangeProposal {
    pub id: u64,
    pub project_id: u64,
    pub proposer: Address,
    pub description: String,
    pub status: ProposalStatus,
    pub approvals: Vec<Address>,
    pub created_at: u64,
    pub resolved_at: Option<u64>,
}

/// A shared changelog entry editable by collaborators (#767).
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SharedChangelogEntry {
    pub id: u64,
    pub project_id: u64,
    pub editor: Address,
    pub cid: String,
    pub note: String,
    pub created_at: u64,
    pub updated_at: u64,
}

/// One audit-trail action (#767 audit trail).
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuditEntry {
    pub project_id: u64,
    pub actor: Address,
    pub action: String,
    pub detail: String,
    pub timestamp: u64,
}

// ── Types (#765: encrypted metadata) ───────────────────────────────────────────

/// A field-level metadata entry that may be encrypted with the owner's
/// public key (#765).
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EncryptedMetadata {
    pub project_id: u64,
    pub field: String,
    pub is_encrypted: bool,
    pub pubkey: String,
    pub payload: String,
    pub updated_by: Address,
    pub updated_at: u64,
}

// ── Types (#764: quarantine) ───────────────────────────────────────────────────

/// Quarantine state for a suspicious project (#764).
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QuarantineState {
    pub project_id: u64,
    pub is_quarantined: bool,
    pub reason: String,
    pub quarantined_by: Address,
    pub quarantined_at: u64,
    /// 30-day review deadline (`quarantined_at + 30 days`).
    pub review_deadline: u64,
}

/// Seconds in the 30-day quarantine review period.
pub const QUARANTINE_REVIEW_PERIOD: u64 = 30 * 24 * 60 * 60;
/// Cap on stored ACL change records per project.
const MAX_ACL_HISTORY: u32 = 100;
/// Cap on stored audit-trail entries per project.
const MAX_AUDIT_TRAIL: u32 = 200;

pub struct ProjectControls;

impl ProjectControls {
    // ── storage helpers ───────────────────────────────────────────────────

    fn get_acl(env: &Env, project_id: u64) -> ProjectAcl {
        env.storage()
            .persistent()
            .get(&ProjectControlsKey::ProjectAcl(project_id))
            .unwrap_or_else(|| ProjectAcl {
                project_id,
                mode: AccessMode::Public,
                entries: Vec::new(env),
            })
    }

    fn set_acl(env: &Env, acl: &ProjectAcl) {
        env.storage()
            .persistent()
            .set(&ProjectControlsKey::ProjectAcl(acl.project_id), acl);
    }

    fn append_acl_change(env: &Env, record: &AclChangeRecord) {
        let key = ProjectControlsKey::AclChangeHistory(record.project_id);
        let mut history: Vec<AclChangeRecord> = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or_else(|| Vec::new(env));
        if history.len() >= MAX_ACL_HISTORY {
            history.remove(0);
        }
        history.push_back(record.clone());
        env.storage().persistent().set(&key, &history);
    }

    fn append_audit(env: &Env, project_id: u64, actor: &Address, action: &str, detail: String) {
        let key = ProjectControlsKey::AuditTrail(project_id);
        let mut trail: Vec<AuditEntry> = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or_else(|| Vec::new(env));
        if trail.len() >= MAX_AUDIT_TRAIL {
            trail.remove(0);
        }
        let entry = AuditEntry {
            project_id,
            actor: actor.clone(),
            action: String::from_str(env, action),
            detail,
            timestamp: env.ledger().timestamp(),
        };
        trail.push_back(entry);
        env.storage().persistent().set(&key, &trail);
    }

    fn require_project(env: &Env, project_id: u64) -> Result<Project, ContractError> {
        ProjectRegistry::get_project(env, project_id).ok_or(ContractError::ProjectNotFound)
    }

    fn is_owner_or_admin(env: &Env, project: &Project, caller: &Address) -> bool {
        project.owner == *caller || AdminManager::is_admin(env, caller)
    }

    // ── #766: access control lists ────────────────────────────────────────

    /// Effective permission `user` holds on `project_id`.
    /// Absent ACL defaults to `Public`/`View`; the owner and admins always
    /// hold `Admin`.
    pub fn acl_permission(env: &Env, project_id: u64, user: &Address) -> AclPermission {
        let project = match ProjectRegistry::get_project(env, project_id) {
            Some(p) => p,
            None => return AclPermission::View,
        };
        if project.owner == *user || AdminManager::is_admin(env, user) {
            return AclPermission::Admin;
        }
        let acl = Self::get_acl(env, project_id);
        match acl.mode {
            AccessMode::Public => AclPermission::View,
            AccessMode::Private => AclPermission::View, // not usable: gate below
            AccessMode::Custom => {
                for i in 0..acl.entries.len() {
                    if let Some(entry) = acl.entries.get(i) {
                        if entry.address == *user {
                            return entry.permission;
                        }
                    }
                }
                AclPermission::View
            }
        }
    }

    /// Whether `user` may view the project (owner/admin always yes).
    pub fn check_view_access(env: &Env, project_id: u64, user: &Address) -> bool {
        let project = match ProjectRegistry::get_project(env, project_id) {
            Some(p) => p,
            None => return false,
        };
        if Self::is_owner_or_admin(env, &project, user) {
            return true;
        }
        let acl = Self::get_acl(env, project_id);
        match acl.mode {
            AccessMode::Public => true,
            AccessMode::Private => false,
            AccessMode::Custom => Self::find_entry(env, project_id, user).is_some(),
        }
    }

    /// Whether `user` may edit the project.
    pub fn check_edit_access(env: &Env, project_id: u64, user: &Address) -> bool {
        let project = match ProjectRegistry::get_project(env, project_id) {
            Some(p) => p,
            None => return false,
        };
        if Self::is_owner_or_admin(env, &project, user)
            || ProjectRegistry::is_maintainer(env, project_id, user)
        {
            return true;
        }
        if let Some(entry) = Self::find_entry(env, project_id, user) {
            return matches!(entry.permission, AclPermission::Edit | AclPermission::Admin);
        }
        false
    }

    /// Whether `user` may administer the project ACL / collaborators.
    pub fn check_admin_access(env: &Env, project_id: u64, user: &Address) -> bool {
        let project = match ProjectRegistry::get_project(env, project_id) {
            Some(p) => p,
            None => return false,
        };
        if Self::is_owner_or_admin(env, &project, user) {
            return true;
        }
        matches!(
            Self::find_entry(env, project_id, user).map(|e| e.permission),
            Some(AclPermission::Admin)
        )
    }

    fn find_entry(env: &Env, project_id: u64, user: &Address) -> Option<AclEntry> {
        let acl = Self::get_acl(env, project_id);
        for i in 0..acl.entries.len() {
            if let Some(entry) = acl.entries.get(i) {
                if entry.address == *user {
                    return Some(entry);
                }
            }
        }
        None
    }

    /// Owner/admin: set the access mode. Records an access-change entry.
    pub fn set_acl_mode(
        env: &Env,
        caller: Address,
        project_id: u64,
        mode: AccessMode,
    ) -> Result<(), ContractError> {
        caller.require_auth();
        let project = Self::require_project(env, project_id)?;
        if !Self::is_owner_or_admin(env, &project, &caller) {
            return Err(ContractError::Unauthorized);
        }
        let mut acl = Self::get_acl(env, project_id);
        acl.mode = mode;
        Self::set_acl(env, &acl);
        Self::append_acl_change(
            env,
            &AclChangeRecord {
                project_id,
                actor: caller.clone(),
                target: None,
                action: AclAction::ModeSet,
                mode: Some(mode),
                permission: None,
                timestamp: env.ledger().timestamp(),
            },
        );
        Self::append_audit(
            env,
            project_id,
            &caller,
            "acl_mode_set",
            String::from_str(env, ""),
        );
        env.events().publish(
            (symbol_short!("ACL"), symbol_short!("MODE"), project_id),
            acl,
        );
        Ok(())
    }

    /// Owner/admin: add (or update) an address on the whitelist.
    pub fn acl_add_address(
        env: &Env,
        caller: Address,
        project_id: u64,
        target: Address,
        permission: AclPermission,
    ) -> Result<(), ContractError> {
        caller.require_auth();
        let project = Self::require_project(env, project_id)?;
        if !Self::is_owner_or_admin(env, &project, &caller) {
            return Err(ContractError::Unauthorized);
        }
        let mut acl = Self::get_acl(env, project_id);
        let mut found = false;
        for i in 0..acl.entries.len() {
            if let Some(mut entry) = acl.entries.get(i) {
                if entry.address == target {
                    entry.permission = permission;
                    acl.entries.set(i, entry);
                    found = true;
                    break;
                }
            }
        }
        if !found {
            acl.entries.push_back(AclEntry {
                address: target.clone(),
                permission,
            });
        }
        Self::set_acl(env, &acl);
        Self::append_acl_change(
            env,
            &AclChangeRecord {
                project_id,
                actor: caller.clone(),
                target: Some(target.clone()),
                action: if found {
                    AclAction::PermissionChanged
                } else {
                    AclAction::AddressAdded
                },
                mode: None,
                permission: Some(permission),
                timestamp: env.ledger().timestamp(),
            },
        );
        Self::append_audit(
            env,
            project_id,
            &caller,
            "acl_address_added",
            String::from_str(env, ""),
        );
        env.events().publish(
            (symbol_short!("ACL"), symbol_short!("ADD"), project_id),
            acl,
        );
        Ok(())
    }

    /// Owner/admin: remove an address from the whitelist.
    pub fn acl_remove_address(
        env: &Env,
        caller: Address,
        project_id: u64,
        target: Address,
    ) -> Result<(), ContractError> {
        caller.require_auth();
        let project = Self::require_project(env, project_id)?;
        if !Self::is_owner_or_admin(env, &project, &caller) {
            return Err(ContractError::Unauthorized);
        }
        let mut acl = Self::get_acl(env, project_id);
        let mut idx: Option<u32> = None;
        for i in 0..acl.entries.len() {
            if let Some(entry) = acl.entries.get(i) {
                if entry.address == target {
                    idx = Some(i);
                    break;
                }
            }
        }
        let idx = idx.ok_or(ContractError::Unauthorized)?;
        acl.entries.remove(idx);
        Self::set_acl(env, &acl);
        Self::append_acl_change(
            env,
            &AclChangeRecord {
                project_id,
                actor: caller.clone(),
                target: Some(target.clone()),
                action: AclAction::AddressRemoved,
                mode: None,
                permission: None,
                timestamp: env.ledger().timestamp(),
            },
        );
        Self::append_audit(
            env,
            project_id,
            &caller,
            "acl_address_removed",
            String::from_str(env, ""),
        );
        env.events().publish(
            (symbol_short!("ACL"), symbol_short!("REM"), project_id),
            acl,
        );
        Ok(())
    }

    /// Returns the ACL for a project (defaults to Public when unset).
    pub fn get_acl_for(env: &Env, project_id: u64) -> ProjectAcl {
        Self::get_acl(env, project_id)
    }

    /// Change history for access modifications (#766).
    pub fn get_acl_history(env: &Env, project_id: u64) -> Vec<AclChangeRecord> {
        env.storage()
            .persistent()
            .get(&ProjectControlsKey::AclChangeHistory(project_id))
            .unwrap_or_else(|| Vec::new(env))
    }

    // ── #767: collaboration tools ─────────────────────────────────────────

    fn get_collaborators(env: &Env, project_id: u64) -> Vec<Collaborator> {
        env.storage()
            .persistent()
            .get(&ProjectControlsKey::Collaborators(project_id))
            .unwrap_or_else(|| Vec::new(env))
    }

    fn set_collaborators(env: &Env, project_id: u64, list: &Vec<Collaborator>) {
        env.storage()
            .persistent()
            .set(&ProjectControlsKey::Collaborators(project_id), list);
    }

    fn find_collaborator(list: &Vec<Collaborator>, address: &Address) -> Option<u32> {
        for i in 0..list.len() {
            if let Some(c) = list.get(i) {
                if c.address == *address {
                    return Some(i);
                }
            }
        }
        None
    }

    /// Owner/admin: add a collaborator with a role.
    pub fn add_collaborator(
        env: &Env,
        caller: Address,
        project_id: u64,
        collaborator: Address,
        role: CollaboratorRole,
    ) -> Result<(), ContractError> {
        caller.require_auth();
        let project = Self::require_project(env, project_id)?;
        if !Self::is_owner_or_admin(env, &project, &caller) {
            return Err(ContractError::Unauthorized);
        }
        let mut list = Self::get_collaborators(env, project_id);
        if Self::find_collaborator(&list, &collaborator).is_some() {
            return Err(ContractError::AlreadyMaintainerAdded);
        }
        list.push_back(Collaborator {
            project_id,
            address: collaborator.clone(),
            role,
            added_at: env.ledger().timestamp(),
        });
        Self::set_collaborators(env, project_id, &list);
        Self::append_audit(
            env,
            project_id,
            &caller,
            "collaborator_added",
            String::from_str(env, ""),
        );
        env.events().publish(
            (symbol_short!("COLLAB"), symbol_short!("ADD"), project_id),
            list,
        );
        Ok(())
    }

    /// Owner/admin: change a collaborator's role.
    pub fn set_collaborator_role(
        env: &Env,
        caller: Address,
        project_id: u64,
        collaborator: Address,
        role: CollaboratorRole,
    ) -> Result<(), ContractError> {
        caller.require_auth();
        let project = Self::require_project(env, project_id)?;
        if !Self::is_owner_or_admin(env, &project, &caller) {
            return Err(ContractError::Unauthorized);
        }
        let mut list = Self::get_collaborators(env, project_id);
        let idx =
            Self::find_collaborator(&list, &collaborator).ok_or(ContractError::Unauthorized)?;
        if let Some(mut c) = list.get(idx) {
            c.role = role;
            list.set(idx, c);
        }
        Self::set_collaborators(env, project_id, &list);
        Self::append_audit(
            env,
            project_id,
            &caller,
            "collaborator_role_set",
            String::from_str(env, ""),
        );
        env.events().publish(
            (symbol_short!("COLLAB"), symbol_short!("ROLE"), project_id),
            list,
        );
        Ok(())
    }

    /// Owner/admin: remove a collaborator.
    pub fn remove_collaborator(
        env: &Env,
        caller: Address,
        project_id: u64,
        collaborator: Address,
    ) -> Result<(), ContractError> {
        caller.require_auth();
        let project = Self::require_project(env, project_id)?;
        if !Self::is_owner_or_admin(env, &project, &caller) {
            return Err(ContractError::Unauthorized);
        }
        let mut list = Self::get_collaborators(env, project_id);
        let idx =
            Self::find_collaborator(&list, &collaborator).ok_or(ContractError::Unauthorized)?;
        list.remove(idx);
        Self::set_collaborators(env, project_id, &list);
        Self::append_audit(
            env,
            project_id,
            &caller,
            "collaborator_removed",
            String::from_str(env, ""),
        );
        env.events().publish(
            (symbol_short!("COLLAB"), symbol_short!("REM"), project_id),
            list,
        );
        Ok(())
    }

    pub fn get_collaborator_list(env: &Env, project_id: u64) -> Vec<Collaborator> {
        Self::get_collaborators(env, project_id)
    }

    /// Whether `user` may act on proposals/changelog (owner, admin,
    /// maintainer, or an Editor/Maintainer collaborator).
    fn can_contribute(env: &Env, project: &Project, project_id: u64, user: &Address) -> bool {
        if Self::is_owner_or_admin(env, project, user)
            || ProjectRegistry::is_maintainer(env, project_id, user)
        {
            return true;
        }
        let list = Self::get_collaborators(env, project_id);
        if let Some(idx) = Self::find_collaborator(&list, user) {
            if let Some(c) = list.get(idx) {
                return matches!(
                    c.role,
                    CollaboratorRole::Editor | CollaboratorRole::Maintainer
                );
            }
        }
        false
    }

    /// Owner/admin/maintainer/editor: create a change proposal.
    pub fn create_proposal(
        env: &Env,
        caller: Address,
        project_id: u64,
        description: String,
    ) -> Result<u64, ContractError> {
        caller.require_auth();
        let project = Self::require_project(env, project_id)?;
        if !Self::can_contribute(env, &project, project_id, &caller) {
            return Err(ContractError::Unauthorized);
        }
        let proposal_id: u64 = env
            .storage()
            .persistent()
            .get(&ProjectControlsKey::NextProposalId)
            .unwrap_or(1);
        env.storage()
            .persistent()
            .set(&ProjectControlsKey::NextProposalId, &(proposal_id + 1));

        let mut approvals: Vec<Address> = Vec::new(env);
        approvals.push_back(caller.clone());
        let proposal = ChangeProposal {
            id: proposal_id,
            project_id,
            proposer: caller.clone(),
            description,
            status: ProposalStatus::Pending,
            approvals,
            created_at: env.ledger().timestamp(),
            resolved_at: None,
        };
        env.storage()
            .persistent()
            .set(&ProjectControlsKey::Proposal(proposal_id), &proposal);

        let mut ids: Vec<u64> = env
            .storage()
            .persistent()
            .get(&ProjectControlsKey::ProjectProposals(project_id))
            .unwrap_or_else(|| Vec::new(env));
        ids.push_back(proposal_id);
        env.storage()
            .persistent()
            .set(&ProjectControlsKey::ProjectProposals(project_id), &ids);

        Self::append_audit(
            env,
            project_id,
            &caller,
            "proposal_created",
            String::from_str(env, ""),
        );
        env.events().publish(
            (symbol_short!("PROPOSAL"), symbol_short!("NEW"), project_id),
            proposal_id,
        );
        Ok(proposal_id)
    }

    fn load_proposal(env: &Env, proposal_id: u64) -> Result<ChangeProposal, ContractError> {
        env.storage()
            .persistent()
            .get(&ProjectControlsKey::Proposal(proposal_id))
            .ok_or(ContractError::InvalidInput)
    }

    /// Owner/admin/maintainer/editor: approve a pending proposal.
    pub fn approve_proposal(
        env: &Env,
        caller: Address,
        proposal_id: u64,
    ) -> Result<(), ContractError> {
        caller.require_auth();
        let mut proposal = Self::load_proposal(env, proposal_id)?;
        let project = Self::require_project(env, proposal.project_id)?;
        if !Self::can_contribute(env, &project, proposal.project_id, &caller) {
            return Err(ContractError::Unauthorized);
        }
        if proposal.status != ProposalStatus::Pending {
            return Err(ContractError::InvalidStatusTransition);
        }
        if proposal.approvals.contains(&caller) {
            return Err(ContractError::DuplicateReview);
        }
        proposal.approvals.push_back(caller.clone());
        // Owner (or contract admin) approval resolves the proposal.
        if project.owner == caller || AdminManager::is_admin(env, &caller) {
            proposal.status = ProposalStatus::Approved;
            proposal.resolved_at = Some(env.ledger().timestamp());
        }
        env.storage()
            .persistent()
            .set(&ProjectControlsKey::Proposal(proposal_id), &proposal);
        Self::append_audit(
            env,
            proposal.project_id,
            &caller,
            "proposal_approved",
            String::from_str(env, ""),
        );
        env.events().publish(
            (
                symbol_short!("PROPOSAL"),
                symbol_short!("APPR"),
                proposal.project_id,
            ),
            proposal_id,
        );
        Ok(())
    }

    /// Owner/admin/maintainer/editor: reject a pending proposal.
    pub fn reject_proposal(
        env: &Env,
        caller: Address,
        proposal_id: u64,
    ) -> Result<(), ContractError> {
        caller.require_auth();
        let mut proposal = Self::load_proposal(env, proposal_id)?;
        let project = Self::require_project(env, proposal.project_id)?;
        if !Self::can_contribute(env, &project, proposal.project_id, &caller) {
            return Err(ContractError::Unauthorized);
        }
        if proposal.status != ProposalStatus::Pending {
            return Err(ContractError::InvalidStatusTransition);
        }
        proposal.status = ProposalStatus::Rejected;
        proposal.resolved_at = Some(env.ledger().timestamp());
        env.storage()
            .persistent()
            .set(&ProjectControlsKey::Proposal(proposal_id), &proposal);
        Self::append_audit(
            env,
            proposal.project_id,
            &caller,
            "proposal_rejected",
            String::from_str(env, ""),
        );
        env.events().publish(
            (
                symbol_short!("PROPOSAL"),
                symbol_short!("REJ"),
                proposal.project_id,
            ),
            proposal_id,
        );
        Ok(())
    }

    pub fn get_proposal(env: &Env, proposal_id: u64) -> Option<ChangeProposal> {
        env.storage()
            .persistent()
            .get(&ProjectControlsKey::Proposal(proposal_id))
    }

    pub fn get_project_proposals(env: &Env, project_id: u64) -> Vec<u64> {
        env.storage()
            .persistent()
            .get(&ProjectControlsKey::ProjectProposals(project_id))
            .unwrap_or_else(|| Vec::new(env))
    }

    /// Shared changelog: collaborators with edit rights may add entries.
    pub fn add_changelog(
        env: &Env,
        caller: Address,
        project_id: u64,
        cid: String,
        note: String,
    ) -> Result<u64, ContractError> {
        caller.require_auth();
        let project = Self::require_project(env, project_id)?;
        if !Self::can_contribute(env, &project, project_id, &caller) {
            return Err(ContractError::Unauthorized);
        }
        let entry_id: u64 = env
            .storage()
            .persistent()
            .get(&ProjectControlsKey::NextSharedChangelogId)
            .unwrap_or(1);
        env.storage()
            .persistent()
            .set(&ProjectControlsKey::NextSharedChangelogId, &(entry_id + 1));
        let now = env.ledger().timestamp();
        let entry = SharedChangelogEntry {
            id: entry_id,
            project_id,
            editor: caller.clone(),
            cid,
            note,
            created_at: now,
            updated_at: now,
        };
        env.storage()
            .persistent()
            .set(&ProjectControlsKey::SharedChangelog(entry_id), &entry);
        let mut ids: Vec<u64> = env
            .storage()
            .persistent()
            .get(&ProjectControlsKey::ProjectSharedChangelog(project_id))
            .unwrap_or_else(|| Vec::new(env));
        ids.push_back(entry_id);
        env.storage().persistent().set(
            &ProjectControlsKey::ProjectSharedChangelog(project_id),
            &ids,
        );

        Self::append_audit(
            env,
            project_id,
            &caller,
            "changelog_added",
            String::from_str(env, ""),
        );
        env.events().publish(
            (symbol_short!("CHANGELOG"), symbol_short!("ADD"), project_id),
            entry_id,
        );
        Ok(entry_id)
    }

    /// Shared changelog: collaborators with edit rights may edit entries.
    pub fn edit_changelog(
        env: &Env,
        caller: Address,
        entry_id: u64,
        cid: String,
        note: String,
    ) -> Result<(), ContractError> {
        caller.require_auth();
        let mut entry: SharedChangelogEntry = env
            .storage()
            .persistent()
            .get(&ProjectControlsKey::SharedChangelog(entry_id))
            .ok_or(ContractError::InvalidInput)?;
        let project = Self::require_project(env, entry.project_id)?;
        if !Self::can_contribute(env, &project, entry.project_id, &caller) {
            return Err(ContractError::Unauthorized);
        }
        entry.cid = cid;
        entry.note = note;
        entry.editor = caller.clone();
        entry.updated_at = env.ledger().timestamp();
        env.storage()
            .persistent()
            .set(&ProjectControlsKey::SharedChangelog(entry_id), &entry);
        Self::append_audit(
            env,
            entry.project_id,
            &caller,
            "changelog_edited",
            String::from_str(env, ""),
        );
        env.events().publish(
            (
                symbol_short!("CHANGELOG"),
                symbol_short!("EDIT"),
                entry.project_id,
            ),
            entry_id,
        );
        Ok(())
    }

    pub fn get_changelog_entry(env: &Env, entry_id: u64) -> Option<SharedChangelogEntry> {
        env.storage()
            .persistent()
            .get(&ProjectControlsKey::SharedChangelog(entry_id))
    }

    pub fn get_project_changelog(env: &Env, project_id: u64) -> Vec<u64> {
        env.storage()
            .persistent()
            .get(&ProjectControlsKey::ProjectSharedChangelog(project_id))
            .unwrap_or_else(|| Vec::new(env))
    }

    /// Audit trail of all collaboration/ACL/quarantine changes (#767).
    pub fn get_audit_trail(env: &Env, project_id: u64) -> Vec<AuditEntry> {
        env.storage()
            .persistent()
            .get(&ProjectControlsKey::AuditTrail(project_id))
            .unwrap_or_else(|| Vec::new(env))
    }

    // ── #765: encrypted metadata ──────────────────────────────────────────

    /// Owner (or admin): store a field, optionally encrypted with `pubkey`.
    pub fn set_metadata_field(
        env: &Env,
        caller: Address,
        project_id: u64,
        field: String,
        is_encrypted: bool,
        pubkey: String,
        payload: String,
    ) -> Result<(), ContractError> {
        caller.require_auth();
        let project = Self::require_project(env, project_id)?;
        if project.owner != caller && !AdminManager::is_admin(env, &caller) {
            return Err(ContractError::Unauthorized);
        }
        if is_encrypted && pubkey.is_empty() {
            return Err(ContractError::InvalidInput);
        }
        let record = EncryptedMetadata {
            project_id,
            field: field.clone(),
            is_encrypted,
            pubkey: pubkey.clone(),
            payload,
            updated_by: caller.clone(),
            updated_at: env.ledger().timestamp(),
        };
        env.storage().persistent().set(
            &ProjectControlsKey::MetadataField(project_id, field),
            &record,
        );

        let mut keys: Vec<String> = env
            .storage()
            .persistent()
            .get(&ProjectControlsKey::MetadataFieldKeys(project_id))
            .unwrap_or_else(|| Vec::new(env));
        if !keys.contains(&field) {
            keys.push_back(field);
            env.storage()
                .persistent()
                .set(&ProjectControlsKey::MetadataFieldKeys(project_id), &keys);
        }
        Self::append_audit(env, project_id, &caller, "metadata_field_set", pubkey);
        env.events().publish(
            (symbol_short!("METADATA"), symbol_short!("SET"), project_id),
            record,
        );
        Ok(())
    }

    /// Public view of a metadata field (flags + pubkey; payload included as
    /// stored — encrypted payloads are ciphertext anyway).
    pub fn get_metadata_field(
        env: &Env,
        project_id: u64,
        field: String,
    ) -> Option<EncryptedMetadata> {
        env.storage()
            .persistent()
            .get(&ProjectControlsKey::MetadataField(project_id, field))
    }

    /// Owner-only access check for reading a field's payload (#765).
    /// Returns the payload only when `caller` is the project owner (or an
    /// admin). Non-encrypted fields are readable by any address with view
    /// access.
    pub fn read_metadata_payload(
        env: &Env,
        caller: Address,
        project_id: u64,
        field: String,
    ) -> Result<String, ContractError> {
        caller.require_auth();
        let record: EncryptedMetadata = env
            .storage()
            .persistent()
            .get(&ProjectControlsKey::MetadataField(project_id, field))
            .ok_or(ContractError::InvalidInput)?;
        if record.is_encrypted {
            if record.updated_by != caller {
                let project = Self::require_project(env, project_id)?;
                if project.owner != caller {
                    return Err(ContractError::Unauthorized);
                }
            }
        } else if !Self::check_view_access(env, project_id, &caller) {
            return Err(ContractError::Unauthorized);
        }
        Ok(record.payload)
    }

    pub fn list_metadata_fields(env: &Env, project_id: u64) -> Vec<String> {
        env.storage()
            .persistent()
            .get(&ProjectControlsKey::MetadataFieldKeys(project_id))
            .unwrap_or_else(|| Vec::new(env))
    }

    // ── #764: quarantine ──────────────────────────────────────────────────

    /// Admin: quarantine a project with a reason. Sets a 30-day review
    /// deadline. Quarantined projects are hidden from search.
    pub fn quarantine_project(
        env: &Env,
        admin: Address,
        project_id: u64,
        reason: String,
    ) -> Result<(), ContractError> {
        admin.require_auth();
        if !AdminManager::is_admin(env, &admin) {
            return Err(ContractError::AdminOnly);
        }
        Self::require_project(env, project_id)?;
        if reason.is_empty() {
            return Err(ContractError::InvalidInput);
        }
        if Self::get_quarantine_state(env, project_id).is_some_and(|q| q.is_quarantined) {
            return Err(ContractError::InvalidStatusTransition);
        }
        let now = env.ledger().timestamp();
        let state = QuarantineState {
            project_id,
            is_quarantined: true,
            reason: reason.clone(),
            quarantined_by: admin.clone(),
            quarantined_at: now,
            review_deadline: now + QUARANTINE_REVIEW_PERIOD,
        };
        env.storage()
            .persistent()
            .set(&ProjectControlsKey::QuarantineState(project_id), &state);
        let mut list: Vec<u64> = env
            .storage()
            .persistent()
            .get(&ProjectControlsKey::QuarantinedProjects)
            .unwrap_or_else(|| Vec::new(env));
        if !list.contains(&project_id) {
            list.push_back(project_id);
            env.storage()
                .persistent()
                .set(&ProjectControlsKey::QuarantinedProjects, &list);
        }
        Self::append_audit(env, project_id, &admin, "project_quarantined", reason);
        env.events().publish(
            (symbol_short!("QUARANT"), symbol_short!("ADD"), project_id),
            state,
        );
        Ok(())
    }

    pub fn get_quarantine_state(env: &Env, project_id: u64) -> Option<QuarantineState> {
        env.storage()
            .persistent()
            .get(&ProjectControlsKey::QuarantineState(project_id))
    }

    pub fn is_quarantined(env: &Env, project_id: u64) -> bool {
        Self::get_quarantine_state(env, project_id).is_some_and(|q| q.is_quarantined)
    }

    /// Whether the 30-day review period has elapsed.
    pub fn review_period_elapsed(env: &Env, project_id: u64) -> bool {
        match Self::get_quarantine_state(env, project_id) {
            Some(q) => env.ledger().timestamp() >= q.review_deadline,
            None => false,
        }
    }

    pub fn list_quarantined(env: &Env) -> Vec<u64> {
        env.storage()
            .persistent()
            .get(&ProjectControlsKey::QuarantinedProjects)
            .unwrap_or_else(|| Vec::new(env))
    }

    fn clear_quarantine(env: &Env, project_id: u64) {
        env.storage()
            .persistent()
            .remove(&ProjectControlsKey::QuarantineState(project_id));
        let mut list: Vec<u64> = env
            .storage()
            .persistent()
            .get(&ProjectControlsKey::QuarantinedProjects)
            .unwrap_or_else(|| Vec::new(env));
        if let Some(pos) = (0..list.len()).find(|&i| list.get(i) == Some(project_id)) {
            list.remove(pos);
        }
        env.storage()
            .persistent()
            .set(&ProjectControlsKey::QuarantinedProjects, &list);
    }

    /// Admin decision after review: restore the project to normal state.
    pub fn restore_quarantine(
        env: &Env,
        admin: Address,
        project_id: u64,
    ) -> Result<(), ContractError> {
        admin.require_auth();
        if !AdminManager::is_admin(env, &admin) {
            return Err(ContractError::AdminOnly);
        }
        if !Self::is_quarantined(env, project_id) {
            return Err(ContractError::InvalidStatusTransition);
        }
        Self::clear_quarantine(env, project_id);
        Self::append_audit(
            env,
            project_id,
            &admin,
            "project_restored",
            String::from_str(env, ""),
        );
        env.events().publish(
            (
                symbol_short!("QUARANT"),
                symbol_short!("RESTOR"),
                project_id,
            ),
            project_id,
        );
        Ok(())
    }

    /// Admin decision after review: delete (archive) the quarantined project.
    pub fn delete_quarantine(
        env: &Env,
        admin: Address,
        project_id: u64,
    ) -> Result<(), ContractError> {
        admin.require_auth();
        if !AdminManager::is_admin(env, &admin) {
            return Err(ContractError::AdminOnly);
        }
        if !Self::is_quarantined(env, project_id) {
            return Err(ContractError::InvalidStatusTransition);
        }
        let project = Self::require_project(env, project_id)?;
        if !project.archived {
            ProjectRegistry::archive_project_unauthorized(env, project_id, admin.clone())?;
        }
        Self::clear_quarantine(env, project_id);
        Self::append_audit(
            env,
            project_id,
            &admin,
            "project_deleted",
            String::from_str(env, ""),
        );
        env.events().publish(
            (
                symbol_short!("QUARANT"),
                symbol_short!("DELETE"),
                project_id,
            ),
            project_id,
        );
        Ok(())
    }
}
