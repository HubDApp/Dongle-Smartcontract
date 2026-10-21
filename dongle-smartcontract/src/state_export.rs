//! Verifiable contract-state snapshots (closes #933).
//!
//! Snapshots support regulatory audits and data recovery. The flow is:
//!
//! 1. An admin calls `export_contract_state_snapshot` (page by page for large
//!    registries). The result carries the exported data and a `state_hash`:
//!    the SHA-256 of the XDR encoding of everything in the page.
//! 2. Off-chain, the admin saves the CLI's JSON output, pins it to IPFS, and
//!    computes the SHA-256 of the JSON file.
//! 3. The admin calls `record_state_snapshot` with the same scope and page
//!    plus the CID and file hash. The contract recomputes `state_hash` itself
//!    (so a caller cannot record a hash for data that never existed) and
//!    stores a `StateSnapshotRecord` with the ledger timestamp.
//! 4. Auditors fetch the record with `get_state_snapshot`, download the CID,
//!    and run `scripts/verify_state_snapshot.py` to check the file against
//!    the record. `verify_state_snapshot` reports whether on-chain state has
//!    changed since the snapshot.
//!
//! See `docs/STATE_SNAPSHOTS.md` for the full procedure.

use crate::admin_manager::AdminManager;
use crate::constants::{
    LEDGER_BUMP_CRITICAL, LEDGER_THRESHOLD_CRITICAL, MAX_SNAPSHOT_PAGE_SIZE,
    STATE_SNAPSHOT_FORMAT_VERSION,
};
use crate::emergency_pause::EmergencyPause;
use crate::errors::ContractError;
use crate::events::publish_state_snapshot_recorded_event;
use crate::fee_manager::FeeManager;
use crate::project_registry::ProjectRegistry;
use crate::storage_keys::{StateExportKey, StorageKey};
use crate::types::{
    ContractConfigSnapshot, ContractStateSnapshot, Project, Review, SnapshotScope,
    SnapshotVerification, StateSnapshotRecord, VerificationRecord,
};
use crate::utils::Utils;
use crate::verification_registry::VerificationRegistry;
use soroban_sdk::{xdr::ToXdr, Address, BytesN, Env, String, Vec};

/// Raw state for one page of project IDs, shared by snapshots and migration
/// bundles.
pub(crate) struct StatePage {
    pub start_project_id: u64,
    /// Last project ID covered by the page (inclusive). Below
    /// `start_project_id` when the page is empty.
    pub end_project_id: u64,
    pub projects: Vec<Project>,
    pub reviews: Vec<Review>,
    pub verifications: Vec<VerificationRecord>,
}

pub struct StateExport;

impl StateExport {
    pub fn export_contract_state_snapshot(
        env: &Env,
        admin: Address,
        scope: SnapshotScope,
        start_project_id: u64,
        limit: u32,
    ) -> Result<ContractStateSnapshot, ContractError> {
        admin.require_auth();
        AdminManager::require_admin(env, &admin)?;
        Ok(Self::build_snapshot(env, scope, start_project_id, limit))
    }

    /// Recompute the snapshot for `scope` and page, then store a record
    /// linking its hash to the IPFS export at `ipfs_cid`.
    pub fn record_state_snapshot(
        env: &Env,
        admin: Address,
        scope: SnapshotScope,
        start_project_id: u64,
        limit: u32,
        ipfs_cid: String,
        export_sha256: BytesN<32>,
    ) -> Result<u64, ContractError> {
        admin.require_auth();
        AdminManager::require_admin(env, &admin)?;

        if !Utils::is_valid_ipfs_cid(&ipfs_cid) {
            return Err(ContractError::InvalidCid);
        }

        let snapshot = Self::build_snapshot(env, scope, start_project_id, limit);

        let id = Self::get_state_snapshot_count(env).saturating_add(1);
        let record = StateSnapshotRecord {
            id,
            format_version: snapshot.format_version,
            scope,
            start_project_id: snapshot.start_project_id,
            end_project_id: snapshot.end_project_id,
            limit: Self::effective_limit(limit),
            state_hash: snapshot.state_hash.clone(),
            export_sha256,
            ipfs_cid: ipfs_cid.clone(),
            published_by: admin.clone(),
            published_at: env.ledger().timestamp(),
            ledger_sequence: env.ledger().sequence(),
        };

        let key = StateExportKey::Snapshot(id);
        env.storage().persistent().set(&key, &record);
        env.storage()
            .persistent()
            .extend_ttl(&key, LEDGER_THRESHOLD_CRITICAL, LEDGER_BUMP_CRITICAL);
        env.storage()
            .persistent()
            .set(&StateExportKey::SnapshotCount, &id);

        publish_state_snapshot_recorded_event(
            env,
            id,
            scope,
            admin,
            snapshot.state_hash,
            ipfs_cid,
        );
        Ok(id)
    }

    pub fn get_state_snapshot(env: &Env, snapshot_id: u64) -> Option<StateSnapshotRecord> {
        env.storage()
            .persistent()
            .get(&StateExportKey::Snapshot(snapshot_id))
    }

    pub fn get_state_snapshot_count(env: &Env) -> u64 {
        env.storage()
            .persistent()
            .get(&StateExportKey::SnapshotCount)
            .unwrap_or(0)
    }

    /// Compare a recorded snapshot's hash with the hash of the same scope and
    /// page computed from current state. `matches == false` means the state
    /// changed after the snapshot was taken, not that the snapshot is corrupt.
    pub fn verify_state_snapshot(
        env: &Env,
        snapshot_id: u64,
    ) -> Result<SnapshotVerification, ContractError> {
        let record =
            Self::get_state_snapshot(env, snapshot_id).ok_or(ContractError::SnapshotNotFound)?;
        let current =
            Self::build_snapshot(env, record.scope, record.start_project_id, record.limit);
        Ok(SnapshotVerification {
            snapshot_id,
            matches: current.state_hash == record.state_hash,
            recorded_hash: record.state_hash,
            current_hash: current.state_hash,
        })
    }

    fn build_snapshot(
        env: &Env,
        scope: SnapshotScope,
        start_project_id: u64,
        limit: u32,
    ) -> ContractStateSnapshot {
        let project_count = Self::project_count(env);
        let config = match scope {
            SnapshotScope::Full | SnapshotScope::Configs => Some(Self::config_snapshot(env)),
            _ => None,
        };

        let (start, end, mut projects, mut reviews, mut verifications) = match scope {
            SnapshotScope::Configs => (0, 0, Vec::new(env), Vec::new(env), Vec::new(env)),
            _ => {
                let page = Self::collect_page(env, start_project_id, limit);
                (
                    page.start_project_id,
                    page.end_project_id,
                    page.projects,
                    page.reviews,
                    page.verifications,
                )
            }
        };
        match scope {
            SnapshotScope::Projects => {
                reviews = Vec::new(env);
                verifications = Vec::new(env);
            }
            SnapshotScope::Reviews => {
                projects = Vec::new(env);
                verifications = Vec::new(env);
            }
            SnapshotScope::Verifications => {
                projects = Vec::new(env);
                reviews = Vec::new(env);
            }
            _ => {}
        }

        let state_hash = Self::hash_state(
            env,
            scope,
            start,
            end,
            &projects,
            &reviews,
            &verifications,
            &config,
        );

        ContractStateSnapshot {
            format_version: STATE_SNAPSHOT_FORMAT_VERSION,
            scope,
            generated_at: env.ledger().timestamp(),
            ledger_sequence: env.ledger().sequence(),
            project_count,
            start_project_id: start,
            end_project_id: end,
            projects,
            reviews,
            verifications,
            config,
            state_hash,
        }
    }

    /// Hash only the exported data, not when it was generated, so identical
    /// state always yields the same hash.
    #[allow(clippy::too_many_arguments)]
    fn hash_state(
        env: &Env,
        scope: SnapshotScope,
        start: u64,
        end: u64,
        projects: &Vec<Project>,
        reviews: &Vec<Review>,
        verifications: &Vec<VerificationRecord>,
        config: &Option<ContractConfigSnapshot>,
    ) -> BytesN<32> {
        let payload = (
            STATE_SNAPSHOT_FORMAT_VERSION,
            scope,
            start,
            end,
            projects.clone(),
            reviews.clone(),
            verifications.clone(),
            config.clone(),
        );
        env.crypto().sha256(&payload.to_xdr(env)).into()
    }

    pub(crate) fn effective_limit(limit: u32) -> u32 {
        if limit == 0 || limit > MAX_SNAPSHOT_PAGE_SIZE {
            MAX_SNAPSHOT_PAGE_SIZE
        } else {
            limit
        }
    }

    pub(crate) fn project_count(env: &Env) -> u64 {
        env.storage()
            .persistent()
            .get(&StorageKey::ProjectCount)
            .unwrap_or(0)
    }

    /// Collect up to `limit` project IDs starting at `start_project_id`
    /// (IDs start at 1; 0 is treated as 1), with every review (hidden ones
    /// included) and every verification record for those projects.
    pub(crate) fn collect_page(env: &Env, start_project_id: u64, limit: u32) -> StatePage {
        let start = if start_project_id == 0 { 1 } else { start_project_id };
        let last = core::cmp::min(
            start.saturating_add(Self::effective_limit(limit) as u64 - 1),
            Self::project_count(env),
        );

        let mut projects = Vec::new(env);
        let mut reviews = Vec::new(env);
        let mut verifications = Vec::new(env);
        let mut id = start;
        while id <= last {
            if let Some(project) = ProjectRegistry::get_project(env, id) {
                projects.push_back(project);

                let reviewers: Vec<Address> = env
                    .storage()
                    .persistent()
                    .get(&StorageKey::ProjectReviews(id))
                    .unwrap_or_else(|| Vec::new(env));
                for reviewer in reviewers.iter() {
                    if let Some(review) = env
                        .storage()
                        .persistent()
                        .get::<_, Review>(&StorageKey::Review(id, reviewer))
                    {
                        reviews.push_back(review);
                    }
                }

                for record in VerificationRegistry::get_verification_history(env, id).iter() {
                    verifications.push_back(record);
                }
            }
            id += 1;
        }

        StatePage {
            start_project_id: start,
            end_project_id: if last >= start { last } else { start - 1 },
            projects,
            reviews,
            verifications,
        }
    }

    pub(crate) fn config_snapshot(env: &Env) -> ContractConfigSnapshot {
        ContractConfigSnapshot {
            admins: AdminManager::get_admin_list(env),
            admin_approval_threshold: AdminManager::get_admin_approval_threshold(env),
            verification_duration_secs: AdminManager::get_verification_duration(env),
            fee_config: FeeManager::get_fee_config(env).ok(),
            paused: EmergencyPause::is_paused(env),
        }
    }
}
