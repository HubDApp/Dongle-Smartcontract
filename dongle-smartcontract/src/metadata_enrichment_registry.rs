//! Automatic metadata enrichment (#760).
//!
//! External metadata (tags from GitHub topics, social links from a repository,
//! etc.) is fetched off-chain and submitted on-chain as a *pending suggestion*
//! by a trusted oracle/relayer. The suggestion sits in `Pending` state until
//! the project owner explicitly approves or rejects it.
//!
//! ## Workflow
//!
//! 1. A relayer submits enrichment data via `submit_enrichment_suggestion`.
//! 2. The owner calls `approve_enrichment_suggestion` or
//!    `reject_enrichment_suggestion`.
//! 3. On approval the suggested fields are written to the project via
//!    `ProjectRegistry::update_project` (owner-auth is satisfied by the
//!    approval call).
//!
//! ## Sources
//!
//! The `source` field is a free-text label (e.g. `"github"`, `"npm"`,
//! `"crates_io"`). The contract does not validate it further.
//!
//! ## Limits
//!
//! Each project holds at most `MAX_ENRICHMENT_SUGGESTIONS` pending suggestions
//! to prevent unbounded storage growth.

use crate::errors::ContractError;
use crate::project_registry::ProjectRegistry;
use crate::storage_keys::MetadataEnrichmentKey as MEK;
use crate::storage_manager::StorageManager;
use crate::types::{
    EnrichmentSuggestion, EnrichmentSuggestionStatus, MetadataEnrichmentFields,
    ProjectUpdateParams,
};
use soroban_sdk::{symbol_short, Address, Env, String, Vec};

/// Maximum number of pending suggestions retained per project.
pub const MAX_ENRICHMENT_SUGGESTIONS: u32 = 20;

pub struct MetadataEnrichmentRegistry;

impl MetadataEnrichmentRegistry {
    // ── Helpers ───────────────────────────────────────────────────────────

    fn load_suggestions(env: &Env, project_id: u64) -> Vec<EnrichmentSuggestion> {
        env.storage()
            .persistent()
            .get(&MEK::EnrichmentSuggestions(project_id))
            .unwrap_or_else(|| Vec::new(env))
    }

    fn save_suggestions(
        env: &Env,
        project_id: u64,
        suggestions: &Vec<EnrichmentSuggestion>,
    ) {
        env.storage()
            .persistent()
            .set(&MEK::EnrichmentSuggestions(project_id), suggestions);
        StorageManager::extend_if_exists_mek(env, &MEK::EnrichmentSuggestions(project_id));
    }

    fn load_counter(env: &Env) -> u64 {
        env.storage()
            .persistent()
            .get(&MEK::NextSuggestionId)
            .unwrap_or(0u64)
    }

    fn increment_counter(env: &Env) -> u64 {
        let id = Self::load_counter(env);
        let next = id.saturating_add(1);
        env.storage()
            .persistent()
            .set(&MEK::NextSuggestionId, &next);
        id
    }

    fn find_suggestion_index(
        suggestions: &Vec<EnrichmentSuggestion>,
        suggestion_id: u64,
    ) -> Option<u32> {
        for i in 0..suggestions.len() {
            if let Some(s) = suggestions.get(i) {
                if s.id == suggestion_id {
                    return Some(i);
                }
            }
        }
        None
    }

    // ── Write ─────────────────────────────────────────────────────────────

    /// Submit a metadata enrichment suggestion for a project.
    ///
    /// The caller must be a contract admin (trusted relayer). The suggestion
    /// is stored with `Pending` status awaiting owner approval.
    pub fn submit_suggestion(
        env: &Env,
        project_id: u64,
        caller: &Address,
        source: String,
        fields: MetadataEnrichmentFields,
    ) -> Result<EnrichmentSuggestion, ContractError> {
        caller.require_auth();
        crate::admin_manager::AdminManager::require_admin(env, caller)?;

        ProjectRegistry::get_project(env, project_id)
            .ok_or(ContractError::ProjectNotFound)?;

        let mut suggestions = Self::load_suggestions(env, project_id);

        // Count only pending suggestions toward the cap.
        let pending_count = suggestions
            .iter()
            .filter(|s| s.status == EnrichmentSuggestionStatus::Pending)
            .count() as u32;

        if pending_count >= MAX_ENRICHMENT_SUGGESTIONS {
            return Err(ContractError::MaxProjectsExceeded);
        }

        let id = Self::increment_counter(env);
        let now = env.ledger().timestamp();

        let suggestion = EnrichmentSuggestion {
            id,
            project_id,
            source: source.clone(),
            fields: fields.clone(),
            status: EnrichmentSuggestionStatus::Pending,
            submitted_by: caller.clone(),
            submitted_at: now,
            reviewed_at: 0,
        };

        suggestions.push_back(suggestion.clone());
        Self::save_suggestions(env, project_id, &suggestions);

        env.events().publish(
            (symbol_short!("ENRICH_S"), project_id),
            (caller.clone(), source, id),
        );

        Ok(suggestion)
    }

    /// Owner approves a pending enrichment suggestion.
    ///
    /// The approved fields are applied to the project via `update_project`.
    /// Only non-`None` fields in the suggestion are updated.
    pub fn approve_suggestion(
        env: &Env,
        project_id: u64,
        suggestion_id: u64,
        owner: &Address,
    ) -> Result<(), ContractError> {
        owner.require_auth();

        let project = ProjectRegistry::get_project(env, project_id)
            .ok_or(ContractError::ProjectNotFound)?;
        if project.owner != *owner {
            return Err(ContractError::Unauthorized);
        }

        let mut suggestions = Self::load_suggestions(env, project_id);
        let idx = Self::find_suggestion_index(&suggestions, suggestion_id)
            .ok_or(ContractError::InvalidInput)?;

        let suggestion = suggestions.get(idx).ok_or(ContractError::InvalidInput)?;

        if suggestion.status != EnrichmentSuggestionStatus::Pending {
            return Err(ContractError::InvalidStatus);
        }

        // Build a ProjectUpdateParams from approved fields.
        let params = ProjectUpdateParams {
            project_id,
            caller: owner.clone(),
            name: None,
            slug: None,
            description: suggestion.fields.description.clone().map(Some),
            category: None,
            website: suggestion.fields.website.clone().map(Some),
            license: None,
            logo_cid: None,
            metadata_cid: None,
            tags: suggestion.fields.tags.clone().map(Some),
            social_links: suggestion.fields.social_links.clone().map(Some),
            launch_timestamp: None,
            bounty_url: None,
            repository_url: suggestion.fields.repository_url.clone().map(Some),
        };

        ProjectRegistry::update_project(env, params)?;

        // Mark suggestion as approved.
        let now = env.ledger().timestamp();
        let mut updated = suggestions.get(idx).ok_or(ContractError::InvalidInput)?;
        updated.status = EnrichmentSuggestionStatus::Approved;
        updated.reviewed_at = now;
        suggestions.set(idx, updated);
        Self::save_suggestions(env, project_id, &suggestions);

        env.events().publish(
            (symbol_short!("ENRICH_A"), project_id),
            (owner.clone(), suggestion_id, now),
        );

        Ok(())
    }

    /// Owner rejects a pending enrichment suggestion.
    pub fn reject_suggestion(
        env: &Env,
        project_id: u64,
        suggestion_id: u64,
        owner: &Address,
    ) -> Result<(), ContractError> {
        owner.require_auth();

        let project = ProjectRegistry::get_project(env, project_id)
            .ok_or(ContractError::ProjectNotFound)?;
        if project.owner != *owner {
            return Err(ContractError::Unauthorized);
        }

        let mut suggestions = Self::load_suggestions(env, project_id);
        let idx = Self::find_suggestion_index(&suggestions, suggestion_id)
            .ok_or(ContractError::InvalidInput)?;

        let suggestion = suggestions.get(idx).ok_or(ContractError::InvalidInput)?;
        if suggestion.status != EnrichmentSuggestionStatus::Pending {
            return Err(ContractError::InvalidStatus);
        }

        let now = env.ledger().timestamp();
        let mut updated = suggestions.get(idx).ok_or(ContractError::InvalidInput)?;
        updated.status = EnrichmentSuggestionStatus::Rejected;
        updated.reviewed_at = now;
        suggestions.set(idx, updated);
        Self::save_suggestions(env, project_id, &suggestions);

        env.events().publish(
            (symbol_short!("ENRICH_R"), project_id),
            (owner.clone(), suggestion_id, now),
        );

        Ok(())
    }

    // ── Read ──────────────────────────────────────────────────────────────

    /// Return all suggestions for a project (pending and reviewed).
    pub fn get_suggestions(
        env: &Env,
        project_id: u64,
    ) -> Result<Vec<EnrichmentSuggestion>, ContractError> {
        ProjectRegistry::get_project(env, project_id)
            .ok_or(ContractError::ProjectNotFound)?;
        Ok(Self::load_suggestions(env, project_id))
    }

    /// Return only pending suggestions for a project.
    pub fn get_pending_suggestions(
        env: &Env,
        project_id: u64,
    ) -> Result<Vec<EnrichmentSuggestion>, ContractError> {
        ProjectRegistry::get_project(env, project_id)
            .ok_or(ContractError::ProjectNotFound)?;
        let all = Self::load_suggestions(env, project_id);
        let mut result = Vec::new(env);
        for s in all.iter() {
            if s.status == EnrichmentSuggestionStatus::Pending {
                result.push_back(s);
            }
        }
        Ok(result)
    }
}
