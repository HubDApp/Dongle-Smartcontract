//! Rule-based ("smart") collections — issue #906.
//!
//! A *smart collection* derives its membership from declarative
//! [`CurationRule`]s instead of being hand-picked by an admin. Each rule
//! carries the conditions the issue asks for:
//!
//! * `tag`        — the project must carry this tag,
//! * `min_rating` — the project's average rating must be at least this value
//!   (scaled by 100, so `350` means 3.5 stars),
//! * `category`   — the project must belong to this category.
//!
//! A rule is a **conjunction**: every condition that is set must hold. A
//! project is a member of the collection when it matches **at least one** of
//! the collection's rules.
//!
//! # Evaluation model
//!
//! Soroban has no background workers, so curation is evaluated lazily:
//!
//! * [`SmartCollectionRegistry::refresh_smart_collection`] recomputes the
//!   membership of a single collection on demand (any authenticated caller may
//!   trigger it — it is idempotent).
//! * [`SmartCollectionRegistry::run_scheduled_curation_refresh`] is the
//!   "background job": a keeper-callable sweep that refreshes every smart
//!   collection whose refresh is due.
//!
//! Only projects that curation itself added are removed again when they stop
//! matching (tracked in `SmartCollectionKey::AutoMembers`); manually curated
//! members are never dropped by a refresh.
//!
//! # Rule versioning and rollback
//!
//! Every rule change is stored as an immutable snapshot
//! (`SmartCollectionKey::RuleVersion`). The active rule set always points at
//! the current `rule_version`. Rolling back re-activates a stored snapshot as
//! a *new* version, so the full history stays auditable.
//!
//! See `docs/SMART_COLLECTION_CURATION.md` for the operator guide.

use crate::auth::require_admin_auth;
use crate::constants::{
    CURATION_REFRESH_INTERVAL_SECS, MAX_COLLECTION_DESCRIPTION_LEN, MAX_COLLECTION_NAME_LEN,
    MAX_CURATION_CATEGORY_LEN, MAX_CURATION_RATING_SCALED, MAX_CURATION_RULES,
    MAX_CURATION_SCAN_PROJECTS, MAX_CURATION_TAG_LEN, MAX_PROJECTS_PER_COLLECTION,
    MAX_SMART_COLLECTIONS,
};
use crate::errors::ContractError;
use crate::events::{
    publish_project_added_to_collection_event, publish_project_removed_from_collection_event,
    publish_smart_collection_created_event, publish_smart_collection_refreshed_event,
    publish_smart_collection_rolled_back_event, publish_smart_collection_rules_updated_event,
};
use crate::pagination::paginate;
use crate::project_registry::ProjectRegistry;
use crate::review_registry::ReviewRegistry;
use crate::storage_keys::{SmartCollectionKey, StorageKey};
use crate::types::{
    CurationRefreshResult, CurationRule, Project, SmartCollection, SmartCollectionRuleVersion,
};
use crate::utils::Utils;
use soroban_sdk::{Address, Env, String, Vec};

pub struct SmartCollectionRegistry;

impl SmartCollectionRegistry {
    // ── Creation ─────────────────────────────────────────────────────────

    /// Create a rule-based collection (admin-only).
    ///
    /// The initial rule set is stored as version `1` so that it can be
    /// restored later via [`SmartCollectionRegistry::rollback_smart_collection_rules`].
    pub fn create_smart_collection(
        env: &Env,
        admin: Address,
        name: String,
        description: String,
        rules: Vec<CurationRule>,
        is_public: bool,
    ) -> Result<u64, ContractError> {
        require_admin_auth(env, &admin)?;

        Self::validate_name(&name)?;
        Self::validate_description(&description)?;
        Self::validate_rules(&rules)?;

        let norm = Utils::normalize_project_name(env, &name);
        if env
            .storage()
            .persistent()
            .get::<_, u64>(&SmartCollectionKey::NameIndex(norm))
            .is_some()
        {
            return Err(ContractError::SmartCollectionNameExists);
        }

        let list: Vec<u64> = env
            .storage()
            .persistent()
            .get(&SmartCollectionKey::CollectionList)
            .unwrap_or_else(|| Vec::new(env));
        if list.len() >= MAX_SMART_COLLECTIONS {
            return Err(ContractError::MaxProjectsExceeded);
        }

        let id: u64 = env
            .storage()
            .persistent()
            .get(&SmartCollectionKey::NextId)
            .unwrap_or(1);
        let now = env.ledger().timestamp();

        let collection = SmartCollection {
            id,
            owner: admin.clone(),
            name: name.clone(),
            description,
            rules: rules.clone(),
            rule_version: 1,
            is_public,
            created_at: now,
            updated_at: now,
            last_refreshed_at: None,
        };

        env.storage()
            .persistent()
            .set(&SmartCollectionKey::Collection(id), &collection);
        env.storage()
            .persistent()
            .set(&SmartCollectionKey::NameIndex(norm), &id);
        env.storage()
            .persistent()
            .set(&SmartCollectionKey::NextId, &(id + 1));

        // Membership lives in the shared collection member list so that the
        // generic collection endpoints (`list_collection_projects`, …) work
        // for smart collections as well.
        env.storage()
            .persistent()
            .set(&StorageKey::CollectionProjectIds(id), &Vec::<u64>::new(env));
        env.storage()
            .persistent()
            .set(&SmartCollectionKey::AutoMembers(id), &Vec::<u64>::new(env));

        let mut new_list = list;
        new_list.push_back(id);
        env.storage()
            .persistent()
            .set(&SmartCollectionKey::CollectionList, &new_list);

        // Snapshot version 1 — the initial rule set.
        Self::store_rule_version(env, id, 1, &rules, &admin, None);

        publish_smart_collection_created_event(env, id, name, admin, rules.len(), 1);

        Ok(id)
    }

    // ── Rule management ──────────────────────────────────────────────────

    /// Replace the active rule set of a smart collection (admin-only).
    ///
    /// The new rule set is stored as the next version; the previous version is
    /// kept so it can be restored with a rollback.
    pub fn update_smart_collection_rules(
        env: &Env,
        admin: Address,
        collection_id: u64,
        rules: Vec<CurationRule>,
    ) -> Result<u32, ContractError> {
        require_admin_auth(env, &admin)?;
        Self::validate_rules(&rules)?;

        let mut collection = Self::require_collection(env, collection_id)?;
        let previous_version = collection.rule_version;
        let new_version = previous_version.saturating_add(1);

        collection.rules = rules.clone();
        collection.rule_version = new_version;
        collection.updated_at = env.ledger().timestamp();
        env.storage()
            .persistent()
            .set(&SmartCollectionKey::Collection(collection_id), &collection);

        Self::store_rule_version(env, collection_id, new_version, &rules, &admin, None);

        publish_smart_collection_rules_updated_event(
            env,
            collection_id,
            admin,
            previous_version,
            new_version,
            rules.len(),
        );

        Ok(new_version)
    }

    /// Roll the rule set back to a previously stored version (admin-only).
    ///
    /// The restored rule set is stored as a **new** version, so the history of
    /// rule sets remains complete and auditable.
    pub fn rollback_smart_collection_rules(
        env: &Env,
        admin: Address,
        collection_id: u64,
        version: u32,
    ) -> Result<u32, ContractError> {
        require_admin_auth(env, &admin)?;

        let mut collection = Self::require_collection(env, collection_id)?;
        if version == collection.rule_version {
            return Err(ContractError::SmartCollectionRuleVersionNotFound);
        }

        let snapshot = Self::get_rule_version(env, collection_id, version)
            .ok_or(ContractError::SmartCollectionRuleVersionNotFound)?;
        Self::validate_rules(&snapshot.rules)?;

        let new_version = collection.rule_version.saturating_add(1);
        collection.rules = snapshot.rules.clone();
        collection.rule_version = new_version;
        collection.updated_at = env.ledger().timestamp();
        env.storage()
            .persistent()
            .set(&SmartCollectionKey::Collection(collection_id), &collection);

        let note = String::from_str(env, "rollback");
        Self::store_rule_version(
            env,
            collection_id,
            new_version,
            &snapshot.rules,
            &admin,
            Some(note),
        );

        publish_smart_collection_rolled_back_event(env, collection_id, admin, version, new_version);

        Ok(new_version)
    }

    // ── Evaluation ───────────────────────────────────────────────────────

    /// Recompute the membership of a smart collection from its active rules.
    ///
    /// Idempotent: calling it twice in a row produces the same membership.
    /// Any authenticated caller may trigger a refresh; the refresh itself is
    /// deterministic and only ever adds projects that match the rules and
    /// removes projects that curation itself had added.
    pub fn refresh_smart_collection(
        env: &Env,
        caller: Address,
        collection_id: u64,
    ) -> Result<CurationRefreshResult, ContractError> {
        caller.require_auth();

        let mut collection = Self::require_collection(env, collection_id)?;

        let total_projects = ProjectRegistry::get_project_count(env);
        let scan_limit = core::cmp::min(total_projects, MAX_CURATION_SCAN_PROJECTS as u64);

        let mut matched: Vec<u64> = Vec::new(env);
        let mut scanned: u32 = 0;
        let mut id: u64 = 1;
        while id <= scan_limit {
            if let Some(project) = ProjectRegistry::get_project(env, id) {
                scanned = scanned.saturating_add(1);
                if !project.archived && Self::project_matches(env, &project, &collection.rules) {
                    matched.push_back(project.id);
                }
            }
            id += 1;
        }

        let members = Self::get_members(env, collection_id);
        let auto_members = Self::get_auto_members(env, collection_id);

        let mut added: u32 = 0;
        let mut removed: u32 = 0;
        let mut unchanged: u32 = 0;
        let mut new_members = members.clone();
        let mut new_auto_members: Vec<u64> = Vec::new(env);

        // ── Add the projects that newly match ────────────────────────────
        //
        // A project that curation adds becomes an *auto-managed* member: it
        // is removed again when it stops matching. A project that was added
        // manually and happens to match stays a manual member — a refresh
        // never curates away the owner's own choices.
        for project_id in matched.iter() {
            let was_member = new_members.iter().any(|m| m == project_id);
            let was_auto_managed = auto_members.iter().any(|m| m == project_id);

            if !was_member {
                if new_members.len() < MAX_PROJECTS_PER_COLLECTION {
                    new_members.push_back(project_id);
                    added = added.saturating_add(1);
                    publish_project_added_to_collection_event(
                        env,
                        collection_id,
                        project_id,
                        caller.clone(),
                    );
                    new_auto_members.push_back(project_id);
                }
            } else {
                unchanged = unchanged.saturating_add(1);
                if was_auto_managed {
                    new_auto_members.push_back(project_id);
                }
            }
        }

        // ── Drop the auto-managed members that no longer match ───────────
        for project_id in auto_members.iter() {
            if !matched.iter().any(|m| m == project_id) {
                let updated = Utils::remove_item_from_vec(env, &new_members, &project_id);
                new_members = updated;
                removed = removed.saturating_add(1);
                publish_project_removed_from_collection_event(
                    env,
                    collection_id,
                    project_id,
                    caller.clone(),
                );
            }
        }

        env.storage().persistent().set(
            &StorageKey::CollectionProjectIds(collection_id),
            &new_members,
        );
        env.storage().persistent().set(
            &SmartCollectionKey::AutoMembers(collection_id),
            &new_auto_members,
        );

        let now = env.ledger().timestamp();
        collection.last_refreshed_at = Some(now);
        collection.updated_at = now;
        env.storage()
            .persistent()
            .set(&SmartCollectionKey::Collection(collection_id), &collection);

        let result = CurationRefreshResult {
            collection_id,
            rule_version: collection.rule_version,
            added,
            removed,
            unchanged,
            total_matched: matched.len(),
            scanned,
            refreshed_at: now,
        };

        publish_smart_collection_refreshed_event(
            env,
            collection_id,
            caller,
            collection.rule_version,
            added,
            removed,
            unchanged,
            matched.len(),
            scanned,
        );

        Ok(result)
    }

    /// Keeper-callable background sweep: refresh every smart collection whose
    /// refresh is due.
    ///
    /// A collection is due when it was never refreshed or when the last
    /// refresh is older than `CURATION_REFRESH_INTERVAL_SECS`. `max_collections`
    /// bounds the number of collections processed by a single call so the
    /// transaction never exceeds the resource budget.
    pub fn run_scheduled_curation_refresh(
        env: &Env,
        caller: Address,
        max_collections: u32,
    ) -> Result<Vec<CurationRefreshResult>, ContractError> {
        caller.require_auth();

        let limit = if max_collections == 0 {
            1
        } else {
            core::cmp::min(max_collections, MAX_SMART_COLLECTIONS)
        };

        let ids: Vec<u64> = env
            .storage()
            .persistent()
            .get(&SmartCollectionKey::CollectionList)
            .unwrap_or_else(|| Vec::new(env));

        let now = env.ledger().timestamp();
        let mut results: Vec<CurationRefreshResult> = Vec::new(env);

        for collection_id in ids.iter() {
            if results.len() >= limit {
                break;
            }
            let collection = match env
                .storage()
                .persistent()
                .get::<_, SmartCollection>(&SmartCollectionKey::Collection(collection_id))
            {
                Some(c) => c,
                None => continue,
            };
            let due = match collection.last_refreshed_at {
                Some(last) => now.saturating_sub(last) >= CURATION_REFRESH_INTERVAL_SECS,
                None => true,
            };
            if !due {
                continue;
            }
            let result = Self::refresh_smart_collection(env, caller.clone(), collection_id)?;
            results.push_back(result);
        }

        Ok(results)
    }

    // ── Reads ────────────────────────────────────────────────────────────

    pub fn get_smart_collection(env: &Env, collection_id: u64) -> Option<SmartCollection> {
        env.storage()
            .persistent()
            .get(&SmartCollectionKey::Collection(collection_id))
    }

    pub fn get_smart_collection_rules(env: &Env, collection_id: u64) -> Vec<CurationRule> {
        match Self::get_smart_collection(env, collection_id) {
            Some(c) => c.rules,
            None => Vec::new(env),
        }
    }

    /// List the stored rule-set versions of a smart collection (oldest first).
    pub fn list_smart_collection_rule_versions(
        env: &Env,
        collection_id: u64,
    ) -> Vec<SmartCollectionRuleVersion> {
        let versions: Vec<u32> = env
            .storage()
            .persistent()
            .get(&SmartCollectionKey::RuleVersionList(collection_id))
            .unwrap_or_else(|| Vec::new(env));
        let mut out = Vec::new(env);
        for version in versions.iter() {
            if let Some(snapshot) = Self::get_rule_version(env, collection_id, version) {
                out.push_back(snapshot);
            }
        }
        out
    }

    /// Read a single stored rule-set version.
    pub fn get_rule_version(
        env: &Env,
        collection_id: u64,
        version: u32,
    ) -> Option<SmartCollectionRuleVersion> {
        env.storage()
            .persistent()
            .get(&SmartCollectionKey::RuleVersion(collection_id, version))
    }

    /// List all smart collections with pagination.
    pub fn list_smart_collections(env: &Env, start_index: u32, limit: u32) -> Vec<SmartCollection> {
        let ids: Vec<u64> = env
            .storage()
            .persistent()
            .get(&SmartCollectionKey::CollectionList)
            .unwrap_or_else(|| Vec::new(env));
        let page = paginate(env, &ids, start_index, limit);
        let mut out = Vec::new(env);
        for id in page.iter() {
            if let Some(collection) = Self::get_smart_collection(env, id) {
                out.push_back(collection);
            }
        }
        out
    }

    /// List the project ids currently curated into a smart collection.
    pub fn list_smart_collection_projects(env: &Env, collection_id: u64) -> Vec<u64> {
        Self::get_members(env, collection_id)
    }

    // ── Internal helpers ─────────────────────────────────────────────────

    fn get_members(env: &Env, collection_id: u64) -> Vec<u64> {
        env.storage()
            .persistent()
            .get(&StorageKey::CollectionProjectIds(collection_id))
            .unwrap_or_else(|| Vec::new(env))
    }

    fn get_auto_members(env: &Env, collection_id: u64) -> Vec<u64> {
        env.storage()
            .persistent()
            .get(&SmartCollectionKey::AutoMembers(collection_id))
            .unwrap_or_else(|| Vec::new(env))
    }

    fn require_collection(env: &Env, collection_id: u64) -> Result<SmartCollection, ContractError> {
        env.storage()
            .persistent()
            .get(&SmartCollectionKey::Collection(collection_id))
            .ok_or(ContractError::SmartCollectionNotFound)
    }

    fn store_rule_version(
        env: &Env,
        collection_id: u64,
        version: u32,
        rules: &Vec<CurationRule>,
        created_by: &Address,
        note: Option<String>,
    ) {
        let snapshot = SmartCollectionRuleVersion {
            collection_id,
            version,
            rules: rules.clone(),
            created_at: env.ledger().timestamp(),
            created_by: created_by.clone(),
            note,
        };
        env.storage().persistent().set(
            &SmartCollectionKey::RuleVersion(collection_id, version),
            &snapshot,
        );

        let mut versions: Vec<u32> = env
            .storage()
            .persistent()
            .get(&SmartCollectionKey::RuleVersionList(collection_id))
            .unwrap_or_else(|| Vec::new(env));
        if !versions.iter().any(|v| v == version) {
            versions.push_back(version);
        }
        env.storage().persistent().set(
            &SmartCollectionKey::RuleVersionList(collection_id),
            &versions,
        );
    }

    /// Evaluate the rule set against a project.
    ///
    /// A rule matches when **every** populated condition holds. The project
    /// matches the collection when **at least one** rule matches.
    fn project_matches(env: &Env, project: &Project, rules: &Vec<CurationRule>) -> bool {
        for rule in rules.iter() {
            if Self::rule_matches(env, project, &rule) {
                return true;
            }
        }
        false
    }

    /// Evaluate a single rule against a project (conjunction of conditions).
    fn rule_matches(env: &Env, project: &Project, rule: &CurationRule) -> bool {
        if let Some(tag) = rule.tag.clone() {
            let wanted = Utils::to_lowercase(env, &tag);
            let mut found = false;
            if let Some(tags) = project.tags.clone() {
                for project_tag in tags.iter() {
                    if Utils::to_lowercase(env, &project_tag) == wanted {
                        found = true;
                        break;
                    }
                }
            }
            if !found {
                return false;
            }
        }

        if let Some(min_rating) = rule.min_rating {
            let stats = ReviewRegistry::get_project_stats(env, project.id);
            if stats.average_rating < min_rating {
                return false;
            }
        }

        if let Some(category) = rule.category.clone() {
            if Utils::to_lowercase(env, &project.category) != Utils::to_lowercase(env, &category) {
                return false;
            }
        }

        true
    }

    fn validate_name(name: &String) -> Result<(), ContractError> {
        if name.len() == 0 || name.len() as usize > MAX_COLLECTION_NAME_LEN {
            return Err(ContractError::InvalidProjectData);
        }
        Ok(())
    }

    fn validate_description(description: &String) -> Result<(), ContractError> {
        if description.len() == 0 || description.len() as usize > MAX_COLLECTION_DESCRIPTION_LEN {
            return Err(ContractError::InvalidProjectData);
        }
        Ok(())
    }

    /// Validate the rule set of a smart collection.
    ///
    /// * at least one rule and at most `MAX_CURATION_RULES` rules,
    /// * every rule must set at least one condition,
    /// * `min_rating` must not exceed 5.00 stars (500),
    /// * tag / category values must respect the length caps.
    pub fn validate_rules(rules: &Vec<CurationRule>) -> Result<(), ContractError> {
        if rules.len() == 0 {
            return Err(ContractError::EmptyCurationRules);
        }
        if rules.len() > MAX_CURATION_RULES {
            return Err(ContractError::TooManyCurationRules);
        }
        for rule in rules.iter() {
            let mut has_condition = false;

            if let Some(tag) = rule.tag.clone() {
                if tag.len() == 0 || tag.len() as usize > MAX_CURATION_TAG_LEN {
                    return Err(ContractError::InvalidCurationRule);
                }
                has_condition = true;
            }

            if let Some(min_rating) = rule.min_rating {
                if min_rating > MAX_CURATION_RATING_SCALED {
                    return Err(ContractError::InvalidCurationRule);
                }
                has_condition = true;
            }

            if let Some(category) = rule.category.clone() {
                if category.len() == 0 || category.len() as usize > MAX_CURATION_CATEGORY_LEN {
                    return Err(ContractError::InvalidCurationRule);
                }
                has_condition = true;
            }

            if !has_condition {
                return Err(ContractError::InvalidCurationRule);
            }
        }
        Ok(())
    }
}
