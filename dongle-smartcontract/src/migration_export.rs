//! Migration bundle export and validation (closes #935).
//!
//! `export_for_migration` packages one page of contract state, plus contract
//! configuration, into a `MigrationBundle` tagged with the source and target
//! contract versions and a content hash. `validate_migration_bundle` checks a
//! bundle against a target version before it is imported into a new
//! deployment. See `docs/CONTRACT_MIGRATION.md` for the full procedure.
//!
//! ## Versioning
//!
//! Bundles carry `format_version`, the layout of the bundle itself, chosen
//! from the target version by [`MigrationExport::format_for_target`]. Only
//! format 1 exists today. A future layout gets a new number and an entry in
//! `SUPPORTED_MIGRATION_FORMATS`, so validators can reject bundles they do
//! not understand.
//!
//! ## Compatibility rules
//!
//! - The target version must be `MAJOR.MINOR.PATCH` with numeric parts.
//! - Downgrades (target below `CONTRACT_VERSION`) are rejected.
//! - The target may be at most one major version ahead of the source.

use crate::admin_manager::AdminManager;
use crate::constants::{CONTRACT_VERSION, MIGRATION_FORMAT_V1, SUPPORTED_MIGRATION_FORMATS};
use crate::errors::ContractError;
use crate::events::publish_migration_bundle_exported_event;
use crate::state_export::StateExport;
use crate::types::{
    ContractConfigSnapshot, MigrationBundle, MigrationCompatibilityReport, MigrationIssue,
    MigrationMetadata, Project, Review, VerificationRecord,
};
use soroban_sdk::{xdr::ToXdr, Address, BytesN, Env, String, Vec};

/// Longest accepted version string, e.g. `"4294967295.4294967295.4294967295"`.
const MAX_VERSION_LEN: usize = 32;

pub struct MigrationExport;

impl MigrationExport {
    pub fn export_for_migration(
        env: &Env,
        admin: Address,
        version: String,
        start_project_id: u64,
        limit: u32,
    ) -> Result<MigrationBundle, ContractError> {
        admin.require_auth();
        AdminManager::require_admin(env, &admin)?;

        let source = String::from_str(env, CONTRACT_VERSION);
        let mut issues = Vec::new(env);
        Self::check_versions(&source, &version, &mut issues);
        if let Some(issue) = issues.get(0) {
            return Err(match issue {
                MigrationIssue::InvalidVersionString => ContractError::InvalidInput,
                _ => ContractError::IncompatibleMigrationTarget,
            });
        }
        let format_version = Self::format_for_target(&version);

        let page = StateExport::collect_page(env, start_project_id, limit);
        let config = StateExport::config_snapshot(env);
        let project_count = StateExport::project_count(env);
        let content_hash = Self::hash_contents(
            env,
            format_version,
            &version,
            page.start_project_id,
            page.end_project_id,
            &page.projects,
            &page.reviews,
            &page.verifications,
            &config,
        );

        let metadata = MigrationMetadata {
            format_version,
            source_contract: env.current_contract_address(),
            source_contract_version: source,
            target_version: version.clone(),
            exported_by: admin.clone(),
            created_at: env.ledger().timestamp(),
            ledger_sequence: env.ledger().sequence(),
            project_count,
            start_project_id: page.start_project_id,
            end_project_id: page.end_project_id,
            next_project_id: if page.end_project_id < project_count {
                Some(page.end_project_id + 1)
            } else {
                None
            },
            project_records: page.projects.len(),
            review_records: page.reviews.len(),
            verification_records: page.verifications.len(),
            content_hash: content_hash.clone(),
        };

        publish_migration_bundle_exported_event(
            env,
            admin,
            version,
            page.start_project_id,
            page.end_project_id,
            content_hash,
        );

        Ok(MigrationBundle {
            metadata,
            projects: page.projects,
            reviews: page.reviews,
            verifications: page.verifications,
            config,
        })
    }

    /// Check that `bundle` is intact and can be imported into a contract at
    /// `target_version`. Never fails; problems are listed in the report.
    pub fn validate_migration_bundle(
        env: &Env,
        bundle: MigrationBundle,
        target_version: String,
    ) -> MigrationCompatibilityReport {
        let meta = &bundle.metadata;
        let mut issues = Vec::new(env);

        if !SUPPORTED_MIGRATION_FORMATS.contains(&meta.format_version) {
            issues.push_back(MigrationIssue::UnsupportedFormatVersion);
        }
        if meta.target_version != target_version {
            issues.push_back(MigrationIssue::TargetVersionMismatch);
        }
        Self::check_versions(&meta.source_contract_version, &target_version, &mut issues);

        if bundle.projects.len() != meta.project_records
            || bundle.reviews.len() != meta.review_records
            || bundle.verifications.len() != meta.verification_records
        {
            issues.push_back(MigrationIssue::RecordCountMismatch);
        }

        let in_range =
            |pid: u64| pid >= meta.start_project_id && pid <= meta.end_project_id;
        let out_of_range = bundle.projects.iter().any(|p| !in_range(p.id))
            || bundle.reviews.iter().any(|r| !in_range(r.project_id))
            || bundle.verifications.iter().any(|v| !in_range(v.project_id));
        if out_of_range {
            issues.push_back(MigrationIssue::RecordOutOfRange);
        }

        let hash = Self::hash_contents(
            env,
            meta.format_version,
            &meta.target_version,
            meta.start_project_id,
            meta.end_project_id,
            &bundle.projects,
            &bundle.reviews,
            &bundle.verifications,
            &bundle.config,
        );
        if hash != meta.content_hash {
            issues.push_back(MigrationIssue::ContentHashMismatch);
        }

        MigrationCompatibilityReport {
            compatible: issues.is_empty(),
            format_version: meta.format_version,
            source_contract_version: meta.source_contract_version.clone(),
            target_version,
            issues,
        }
    }

    /// Bundle layout to use when migrating to `target`. Every supported
    /// target uses format 1 until a new layout is introduced.
    pub fn format_for_target(_target: &String) -> u32 {
        MIGRATION_FORMAT_V1
    }

    fn check_versions(
        source: &String,
        target: &String,
        issues: &mut Vec<MigrationIssue>,
    ) {
        let (Some(src), Some(tgt)) = (Self::parse_version(source), Self::parse_version(target))
        else {
            issues.push_back(MigrationIssue::InvalidVersionString);
            return;
        };
        if tgt < src {
            issues.push_back(MigrationIssue::DowngradeNotSupported);
        } else if tgt.0 > src.0.saturating_add(1) {
            issues.push_back(MigrationIssue::MajorVersionJumpTooLarge);
        }
    }

    /// Parse `MAJOR.MINOR.PATCH` into a tuple that orders like the version.
    pub(crate) fn parse_version(version: &String) -> Option<(u32, u32, u32)> {
        let len = version.len() as usize;
        if len == 0 || len > MAX_VERSION_LEN {
            return None;
        }
        let mut buf = [0u8; MAX_VERSION_LEN];
        version.copy_into_slice(&mut buf[..len]);

        let mut parts = [0u32; 3];
        let mut part = 0usize;
        let mut digits = 0u32;
        for &b in &buf[..len] {
            match b {
                b'0'..=b'9' => {
                    parts[part] = parts[part]
                        .checked_mul(10)?
                        .checked_add((b - b'0') as u32)?;
                    digits += 1;
                }
                b'.' if digits > 0 && part < 2 => {
                    part += 1;
                    digits = 0;
                }
                _ => return None,
            }
        }
        if part != 2 || digits == 0 {
            return None;
        }
        Some((parts[0], parts[1], parts[2]))
    }

    #[allow(clippy::too_many_arguments)]
    fn hash_contents(
        env: &Env,
        format_version: u32,
        target_version: &String,
        start: u64,
        end: u64,
        projects: &Vec<Project>,
        reviews: &Vec<Review>,
        verifications: &Vec<VerificationRecord>,
        config: &ContractConfigSnapshot,
    ) -> BytesN<32> {
        let payload = (
            format_version,
            target_version.clone(),
            start,
            end,
            projects.clone(),
            reviews.clone(),
            verifications.clone(),
            config.clone(),
        );
        env.crypto().sha256(&payload.to_xdr(env)).into()
    }
}
