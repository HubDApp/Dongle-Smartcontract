use crate::admin_action_log::AdminActionLog;
use crate::auth::require_admin_auth;
use crate::constants::{
    MAX_COMMUNITY_COLLECTIONS, MAX_COMMUNITY_COL_CREATOR_SHARE_BPS, MAX_COMMUNITY_COL_CURATORS,
    MAX_COMMUNITY_COL_DESCRIPTION_LEN, MAX_COMMUNITY_COL_NAME_LEN, MAX_COMMUNITY_COL_PROJECTS,
    MAX_COMMUNITY_COL_TAGS_LEN, MAX_FEATURED_COMMUNITY_COLLECTIONS,
    DEFAULT_COMMUNITY_COL_APPROVAL_THRESHOLD, DEFAULT_COMMUNITY_COL_CREATOR_SHARE_BPS,
    DEFAULT_COMMUNITY_COL_DISAPPROVAL_THRESHOLD, MIN_COMMUNITY_COL_CREATOR_SHARE_BPS,
};
use crate::errors::ContractError;
use crate::events::{
    publish_community_collection_created_event, publish_community_collection_updated_event,
    publish_community_col_curators_changed_event, publish_community_col_featured_event,
    publish_community_col_proj_added_event, publish_community_col_proj_removed_event,
    publish_community_col_revenue_attributed_event, publish_community_col_vote_cast_event,
};
use crate::pagination::paginate;
use crate::project_registry::ProjectRegistry;
use crate::storage_keys::CommunityCollectionKey as CCKey;
use crate::storage_keys::StorageKey;
use crate::storage_manager::StorageManager;
use crate::types::{
    AdminActionType, CommunityCollection, CommunityCollectionRole,
    CommunityCollectionTemplateId, CommunityCollectionVote, CommunityColInclusionStatus,
    CommunityColRevenueSnapshot,
};
use crate::utils::Utils;
use soroban_sdk::{Address, Env, String, Vec};

pub struct CommunityCollectionRegistry;

impl CommunityCollectionRegistry {
    // ── Internal helpers ──────────────────────────────────────────────────

    fn next_id(env: &Env) -> u64 {
        let current: u64 = env.storage().persistent().get(&CCKey::NextId).unwrap_or(1);
        env.storage().persistent().set(&CCKey::NextId, &(current + 1));
        StorageManager::extend_community_collection_global_ttl(env);
        current
    }

    fn normalize_name(name: &String) -> String {
        Utils::normalize_project_name(&env(name), name)
    }

    fn validate_metadata(
        name: &String,
        description: &String,
        tags: &Option<String>,
    ) -> Result<(), ContractError> {
        let nlen = name.len() as usize;
        let dlen = description.len() as usize;
        if nlen == 0 || nlen > MAX_COMMUNITY_COL_NAME_LEN {
            return Err(ContractError::CommunityColInvalidMetadata);
        }
        if dlen == 0 || dlen > MAX_COMMUNITY_COL_DESCRIPTION_LEN {
            return Err(ContractError::CommunityColInvalidMetadata);
        }
        if let Some(t) = tags {
            let tlen = t.len() as usize;
            if tlen > MAX_COMMUNITY_COL_TAGS_LEN {
                return Err(ContractError::CommunityColInvalidMetadata);
            }
        }
        Ok(())
    }

    fn validate_thresholds(
        approval: u32,
        disapproval: u32,
    ) -> Result<(), ContractError> {
        // Both zero = fully-gated (voting disabled); both > 0 = symmetric gate.
        if (approval == 0) != (disapproval == 0) {
            return Err(ContractError::CommunityColThresholdInvalid);
        }
        Ok(())
    }

    fn validate_revenue_share(
        creator_share_bps: u32,
        num_curators: u32,
    ) -> Result<(), ContractError> {
        if creator_share_bps < MIN_COMMUNITY_COL_CREATOR_SHARE_BPS
            || creator_share_bps > MAX_COMMUNITY_COL_CREATOR_SHARE_BPS
        {
            return Err(ContractError::CommunityColRevenueShareInvalid);
        }
        // Leave at least a 10% curator share when curators exist.
        let remaining = 10_000u32.saturating_sub(creator_share_bps);
        if num_curators > 0 && remaining < 1_000u32 {
            return Err(ContractError::CommunityColRevenueShareInvalid);
        }
        Ok(())
    }

    fn check_curator(col: &CommunityCollection, addr: &Address) -> bool {
        if &col.creator == addr {
            return true;
        }
        col.curators.iter().any(|c| &c == addr)
    }

    fn require_curator(
        col: &CommunityCollection,
        addr: &Address,
    ) -> Result<(), ContractError> {
        if Self::check_curator(col, addr) {
            Ok(())
        } else {
            Err(ContractError::CommunityColNotCurator)
        }
    }

    fn require_not_template(col: &CommunityCollection) -> Result<(), ContractError> {
        if col.is_template {
            Err(ContractError::CommunityColIsTemplate)
        } else {
            Ok(())
        }
    }

    fn ensure_name_unique(
        env: &Env,
        name: &String,
        exclude_id: Option<u64>,
    ) -> Result<(), ContractError> {
        let norm = Self::normalize_name(name);
        if let Some(existing_id) = env.storage().persistent().get::<_, u64>(&CCKey::NameIndex(norm))
        {
            if exclude_id.map_or(true, |eid| eid != existing_id) {
                return Err(ContractError::CommunityColNameExists);
            }
        }
        Ok(())
    }

    fn require_collection(env: &Env, id: u64) -> Result<CommunityCollection, ContractError> {
        env.storage()
            .persistent()
            .get(&CCKey::Collection(id))
            .ok_or(ContractError::CommunityColNotFound)
    }

    fn get_project_ids(env: &Env, id: u64) -> Vec<u64> {
        env.storage()
            .persistent()
            .get(&CCKey::ProjectIds(id))
            .unwrap_or_else(|| Vec::new(env))
    }

    fn set_project_ids(env: &Env, id: u64, ids: Vec<u64>) {
        env.storage().persistent().set(&CCKey::ProjectIds(id), &ids);
    }

    fn append_to_list(
        env: &Env,
        key: CCKey,
        item: u64,
    ) {
        let mut list: Vec<u64> = env.storage().persistent().get(&key).unwrap_or_else(|| Vec::new(env));
        list.push_back(item);
        env.storage().persistent().set(&key, &list);
    }

    fn remove_from_list(
        env: &Env,
        key: CCKey,
        item: &u64,
    ) -> Vec<u64> {
        let list: Vec<u64> = env.storage().persistent().get(&key).unwrap_or_else(|| Vec::new(env));
        let updated = Utils::remove_item_from_vec(env, &list, item);
        env.storage().persistent().set(&key, &updated);
        updated
    }

    fn include_project(
        env: &Env,
        col: &CommunityCollection,
        project_id: u64,
        actor: Address,
        by_curator: bool,
    ) -> Result<(), ContractError> {
        let mut ids = Self::get_project_ids(env, col.id);
        if ids.iter().any(|pid| pid == project_id) {
            return Err(ContractError::CommunityColAlreadyIncluded);
        }
        if ids.len() >= MAX_COMMUNITY_COL_PROJECTS {
            return Err(ContractError::CommunityColFull);
        }
        ids.push_back(project_id);
        Self::set_project_ids(env, col.id, ids);
        if by_curator {
            env.storage()
                .persistent()
                .set(&CCKey::CuratorIncluded(col.id, project_id), &true);
        }
        publish_community_col_proj_added_event(env, col.id, project_id, actor, by_curator);
        Ok(())
    }

    fn exclude_project(
        env: &Env,
        col: &CommunityCollection,
        project_id: u64,
        actor: Address,
        by_curator: bool,
    ) -> Result<(), ContractError> {
        let ids = Self::get_project_ids(env, col.id);
        if !ids.iter().any(|pid| pid == project_id) {
            return Err(ContractError::CommunityColNotIncluded);
        }
        let updated = Utils::remove_item_from_vec(env, &ids, &project_id);
        Self::set_project_ids(env, col.id, updated);
        if by_curator {
            env.storage()
                .persistent()
                .set(&CCKey::CuratorExcluded(col.id, project_id), &true);
        }
        publish_community_col_proj_removed_event(env, col.id, project_id, actor, by_curator);
        Ok(())
    }

    fn write_collection(env: &Env, col: &CommunityCollection) {
        env.storage().persistent().set(&CCKey::Collection(col.id), col);
        StorageManager::extend_community_collection_ttl(env, col.id);
    }

    fn template_defaults(
        env: &Env,
        template: CommunityCollectionTemplateId,
    ) -> (String, String, Option<String>) {
        match template {
            CommunityCollectionTemplateId::Defi => (
                String::from_str(env, "Stellar DeFi Darlings"),
                String::from_str(
                    env,
                    "Well-known liquidity pools, DEXs, lending and borrowing products on Stellar.",
                ),
                Some(String::from_str(env, "defi,dex,lending,liquidity")),
            ),
            CommunityCollectionTemplateId::Nft => (
                String::from_str(env, "NFT & Marketplaces"),
                String::from_str(
                    env,
                    "Marketplaces, trading venues and minting tools for Stellar NFTs.",
                ),
                Some(String::from_str(env, "nft,marketplace,minting")),
            ),
            CommunityCollectionTemplateId::Dao => (
                String::from_str(env, "DAO & Governance Tools"),
                String::from_str(
                    env,
                    "DAO frameworks, voting systems, treasury management and multisig contracts.",
                ),
                Some(String::from_str(env, "dao,governance,multisig,treasury")),
            ),
            CommunityCollectionTemplateId::Gaming => (
                String::from_str(env, "Gaming & Metaverse"),
                String::from_str(
                    env,
                    "On-chain game worlds, land, in-game assets and metaverse projects.",
                ),
                Some(String::from_str(env, "gaming,metaverse,game-assets")),
            ),
            CommunityCollectionTemplateId::Infra => (
                String::from_str(env, "Infrastructure & Tooling"),
                String::from_str(
                    env,
                    "Oracles, RPC, bridges, block explorers, indexing and SDKs for Stellar.",
                ),
                Some(String::from_str(env, "infra,tooling,oracle,rpc,bridge")),
            ),
            CommunityCollectionTemplateId::Stablecoins => (
                String::from_str(env, "Stablecoins & Payments"),
                String::from_str(
                    env,
                    "Fiat-backed and algorithmic stablecoins plus payment-focused contracts.",
                ),
                Some(String::from_str(env, "stablecoin,stablecoins, payments")),
            ),
            CommunityCollectionTemplateId::PublicGoods => (
                String::from_str(env, "Public Goods & Sustainability"),
                String::from_str(
                    env,
                    "Retroactive public-goods funding, carbon, R&D grants and open-source stewards.",
                ),
                Some(String::from_str(env, "public-goods,sustainability,grants")),
            ),
            CommunityCollectionTemplateId::Verified => (
                String::from_str(env, "Audited & Verified"),
                String::from_str(
                    env,
                    "A starter collection curated from the registry's verified set — trust but verify.",
                ),
                Some(String::from_str(env, "verified,audited,trusted")),
            ),
        }
    }

    // ── Public CRUD ───────────────────────────────────────────────────────

    pub fn create(
        env: &Env,
        creator: Address,
        name: String,
        description: String,
        tags: Option<String>,
        approval_threshold: Option<u32>,
        disapproval_threshold: Option<u32>,
        creator_revenue_share_bps: Option<u32>,
        initial_curators: Option<Vec<Address>>,
    ) -> Result<u64, ContractError> {
        creator.require_auth();
        Self::validate_metadata(&name, &description, &tags)?;
        let a_t = approval_threshold.unwrap_or(DEFAULT_COMMUNITY_COL_APPROVAL_THRESHOLD);
        let d_t = disapproval_threshold.unwrap_or(DEFAULT_COMMUNITY_COL_DISAPPROVAL_THRESHOLD);
        Self::validate_thresholds(a_t, d_t)?;
        Self::ensure_name_unique(env, &name, None)?;

        let curators = initial_curators.unwrap_or_else(|| Vec::new(env));
        if curators.len() > MAX_COMMUNITY_COL_CURATORS {
            return Err(ContractError::CommunityColCuratorsEmpty);
        }
        let share_bps = creator_revenue_share_bps.unwrap_or(DEFAULT_COMMUNITY_COL_CREATOR_SHARE_BPS);
        Self::validate_revenue_share(share_bps, curators.len())?;

        let global_list: Vec<u64> = env
            .storage()
            .persistent()
            .get(&CCKey::CollectionList)
            .unwrap_or_else(|| Vec::new(env));
        if global_list.len() >= MAX_COMMUNITY_COLLECTIONS {
            return Err(ContractError::MaxProjectsExceeded);
        }

        let id = Self::next_id(env);
        let ts = env.ledger().timestamp();
        let col = CommunityCollection {
            id,
            creator: creator.clone(),
            curators: curators.clone(),
            approval_threshold: a_t,
            disapproval_threshold: d_t,
            name: name.clone(),
            description,
            tags: tags.clone(),
            is_template: false,
            template_source: None,
            is_featured: false,
            creator_revenue_share_bps: share_bps,
            created_at: ts,
            updated_at: ts,
        };
        Self::write_collection(env, &col);

        let norm = Self::normalize_name(&name);
        env.storage().persistent().set(&CCKey::NameIndex(norm), &id);
        Self::set_project_ids(env, id, Vec::new(env));

        let mut global = global_list;
        global.push_back(id);
        env.storage().persistent().set(&CCKey::CollectionList, &global);
        StorageManager::extend_community_collection_global_ttl(env);

        Self::append_to_list(env, CCKey::ByCreator(creator.clone()), id);
        for c in curators.iter() {
            Self::append_to_list(env, CCKey::ByCurator(c.clone()), id);
        }
        StorageManager::extend_community_collection_creator_curator_ttl(env, &creator, &curators);

        publish_community_collection_created_event(env, id, creator.clone(), name, false, None);
        Ok(id)
    }

    /// Admin-only: create a pre-defined template collection (AC4). Seeded
    /// templates have `is_template = true` so they cannot accept votes,
    /// revenue, or member edits — they exist purely to be cloned via
    /// `create_from_template`.
    pub fn create_template(
        env: &Env,
        admin: Address,
        template_id: CommunityCollectionTemplateId,
        override_name: Option<String>,
        override_description: Option<String>,
        seed_projects: Option<Vec<u64>>,
    ) -> Result<u64, ContractError> {
        require_admin_auth(env, &admin)?;

        let (def_name, def_desc, def_tags) = Self::template_defaults(env, template_id);
        let name = override_name.unwrap_or(def_name);
        let description = override_description.unwrap_or(def_desc);
        Self::validate_metadata(&name, &description, &def_tags)?;
        Self::ensure_name_unique(env, &name, None)?;

        let id = Self::next_id(env);
        let ts = env.ledger().timestamp();
        let col = CommunityCollection {
            id,
            creator: admin.clone(),
            curators: Vec::new(env),
            approval_threshold: 0,
            disapproval_threshold: 0,
            name: name.clone(),
            description,
            tags: def_tags,
            is_template: true,
            template_source: None,
            is_featured: false,
            creator_revenue_share_bps: DEFAULT_COMMUNITY_COL_CREATOR_SHARE_BPS,
            created_at: ts,
            updated_at: ts,
        };
        Self::write_collection(env, &col);

        let norm = Self::normalize_name(&name);
        env.storage().persistent().set(&CCKey::NameIndex(norm), &id);

        let mut seed_ids = Vec::new(env);
        if let Some(pids) = seed_projects {
            for pid in pids.iter() {
                if ProjectRegistry::get_project(env, pid).is_some()
                    && !seed_ids.iter().any(|x| x == pid)
                {
                    if seed_ids.len() < MAX_COMMUNITY_COL_PROJECTS {
                        seed_ids.push_back(pid);
                    }
                }
            }
        }
        Self::set_project_ids(env, id, seed_ids);

        publish_community_collection_created_event(
            env,
            id,
            admin.clone(),
            name,
            true,
            Some(template_id),
        );
        Ok(id)
    }

    /// Clone an existing template collection (or any community collection as
    /// a de-facto template) into a new non-template collection owned by
    /// `caller`. Copies the project set and description so users don't have
    /// to hand-populate common collections (AC4).
    pub fn create_from_template(
        env: &Env,
        caller: Address,
        source_collection_id: u64,
        name: String,
        description: String,
        tags: Option<String>,
        approval_threshold: Option<u32>,
        disapproval_threshold: Option<u32>,
        creator_revenue_share_bps: Option<u32>,
        initial_curators: Option<Vec<Address>>,
    ) -> Result<u64, ContractError> {
        caller.require_auth();
        Self::validate_metadata(&name, &description, &tags)?;
        Self::ensure_name_unique(env, &name, None)?;
        let a_t = approval_threshold.unwrap_or(DEFAULT_COMMUNITY_COL_APPROVAL_THRESHOLD);
        let d_t = disapproval_threshold.unwrap_or(DEFAULT_COMMUNITY_COL_DISAPPROVAL_THRESHOLD);
        Self::validate_thresholds(a_t, d_t)?;

        let source = Self::require_collection(env, source_collection_id)?;

        let curators = initial_curators.unwrap_or_else(|| Vec::new(env));
        if curators.len() > MAX_COMMUNITY_COL_CURATORS {
            return Err(ContractError::CommunityColCuratorsEmpty);
        }
        let share_bps = creator_revenue_share_bps.unwrap_or(DEFAULT_COMMUNITY_COL_CREATOR_SHARE_BPS);
        Self::validate_revenue_share(share_bps, curators.len())?;

        let global_list: Vec<u64> = env
            .storage()
            .persistent()
            .get(&CCKey::CollectionList)
            .unwrap_or_else(|| Vec::new(env));
        if global_list.len() >= MAX_COMMUNITY_COLLECTIONS {
            return Err(ContractError::MaxProjectsExceeded);
        }

        let id = Self::next_id(env);
        let ts = env.ledger().timestamp();
        let col = CommunityCollection {
            id,
            creator: caller.clone(),
            curators: curators.clone(),
            approval_threshold: a_t,
            disapproval_threshold: d_t,
            name: name.clone(),
            description,
            tags: tags.clone(),
            is_template: false,
            template_source: source.template_source,
            is_featured: false,
            creator_revenue_share_bps: share_bps,
            created_at: ts,
            updated_at: ts,
        };
        Self::write_collection(env, &col);

        let norm = Self::normalize_name(&name);
        env.storage().persistent().set(&CCKey::NameIndex(norm), &id);

        // Copy project set from source (dedup, enforce cap).
        let source_pids = Self::get_project_ids(env, source_collection_id);
        let mut cloned_pids: Vec<u64> = Vec::new(env);
        for pid in source_pids.iter() {
            if ProjectRegistry::get_project(env, pid).is_some()
                && !cloned_pids.iter().any(|x| x == pid)
            {
                if cloned_pids.len() < MAX_COMMUNITY_COL_PROJECTS {
                    cloned_pids.push_back(pid);
                }
            }
        }
        Self::set_project_ids(env, id, cloned_pids);

        let mut global = global_list;
        global.push_back(id);
        env.storage().persistent().set(&CCKey::CollectionList, &global);
        StorageManager::extend_community_collection_global_ttl(env);

        Self::append_to_list(env, CCKey::ByCreator(caller.clone()), id);
        for c in curators.iter() {
            Self::append_to_list(env, CCKey::ByCurator(c.clone()), id);
        }
        StorageManager::extend_community_collection_creator_curator_ttl(env, &caller, &curators);

        publish_community_collection_created_event(
            env,
            id,
            caller.clone(),
            name,
            false,
            source.template_source,
        );
        Ok(id)
    }

    pub fn get(env: &Env, id: u64) -> Option<CommunityCollection> {
        let col: Option<CommunityCollection> = env.storage().persistent().get(&CCKey::Collection(id));
        if col.is_some() {
            StorageManager::extend_community_collection_ttl(env, id);
        }
        col
    }

    pub fn get_count(env: &Env) -> u32 {
        env.storage()
            .persistent()
            .get::<_, Vec<u64>>(&CCKey::CollectionList)
            .map(|v| v.len())
            .unwrap_or(0)
    }

    pub fn list(env: &Env, start_index: u32, limit: u32) -> Vec<CommunityCollection> {
        let ids: Vec<u64> = env
            .storage()
            .persistent()
            .get(&CCKey::CollectionList)
            .unwrap_or_else(|| Vec::new(env));
        let paged = paginate(env, &ids, start_index, limit);
        let mut out = Vec::new(env);
        for id in paged.iter() {
            if let Some(col) = Self::get(env, id) {
                out.push_back(col);
            }
        }
        out
    }

    pub fn list_by_creator(
        env: &Env,
        creator: Address,
        start_index: u32,
        limit: u32,
    ) -> Vec<CommunityCollection> {
        let ids: Vec<u64> = env
            .storage()
            .persistent()
            .get(&CCKey::ByCreator(creator))
            .unwrap_or_else(|| Vec::new(env));
        let paged = paginate(env, &ids, start_index, limit);
        let mut out = Vec::new(env);
        for id in paged.iter() {
            if let Some(col) = Self::get(env, id) {
                out.push_back(col);
            }
        }
        out
    }

    pub fn list_by_curator(
        env: &Env,
        curator: Address,
        start_index: u32,
        limit: u32,
    ) -> Vec<CommunityCollection> {
        let ids: Vec<u64> = env
            .storage()
            .persistent()
            .get(&CCKey::ByCurator(curator))
            .unwrap_or_else(|| Vec::new(env));
        let paged = paginate(env, &ids, start_index, limit);
        let mut out = Vec::new(env);
        for id in paged.iter() {
            if let Some(col) = Self::get(env, id) {
                out.push_back(col);
            }
        }
        out
    }

    pub fn get_project_count(env: &Env, id: u64) -> u32 {
        Self::get_project_ids(env, id).len()
    }

    pub fn list_projects(env: &Env, id: u64, start_index: u32, limit: u32) -> Vec<u64> {
        let ids = Self::get_project_ids(env, id);
        paginate(env, &ids, start_index, limit)
    }

    // ── Curator mutations ─────────────────────────────────────────────────

    pub fn update_metadata(
        env: &Env,
        id: u64,
        updater: Address,
        name: String,
        description: String,
        tags: Option<String>,
    ) -> Result<(), ContractError> {
        updater.require_auth();
        let mut col = Self::require_collection(env, id)?;
        Self::require_curator(&col, &updater)?;

        if col.name != name {
            Self::ensure_name_unique(env, &name, Some(id))?;
            // remove old name-index, add new
            let old_norm = Self::normalize_name(&col.name);
            env.storage().persistent().remove(&CCKey::NameIndex(old_norm));
            let new_norm = Self::normalize_name(&name);
            env.storage().persistent().set(&CCKey::NameIndex(new_norm), &id);
        }

        col.name = name;
        col.description = description;
        col.tags = tags;
        col.updated_at = env.ledger().timestamp();
        Self::write_collection(env, &col);

        publish_community_collection_updated_event(env, id, updater);
        Ok(())
    }

    pub fn set_thresholds(
        env: &Env,
        id: u64,
        updater: Address,
        approval: u32,
        disapproval: u32,
    ) -> Result<(), ContractError> {
        updater.require_auth();
        let mut col = Self::require_collection(env, id)?;
        Self::require_not_template(&col)?;
        Self::require_curator(&col, &updater)?;
        Self::validate_thresholds(approval, disapproval)?;
        col.approval_threshold = approval;
        col.disapproval_threshold = disapproval;
        col.updated_at = env.ledger().timestamp();
        Self::write_collection(env, &col);
        publish_community_collection_updated_event(env, id, updater);
        Ok(())
    }

    pub fn set_revenue_share(
        env: &Env,
        id: u64,
        updater: Address,
        creator_share_bps: u32,
    ) -> Result<(), ContractError> {
        updater.require_auth();
        let mut col = Self::require_collection(env, id)?;
        Self::require_not_template(&col)?;
        if &col.creator != &updater {
            return Err(ContractError::CommunityColNotCurator);
        }
        Self::validate_revenue_share(creator_share_bps, col.curators.len())?;
        col.creator_revenue_share_bps = creator_share_bps;
        col.updated_at = env.ledger().timestamp();
        Self::write_collection(env, &col);
        publish_community_collection_updated_event(env, id, updater);
        Ok(())
    }

    pub fn add_curator(
        env: &Env,
        id: u64,
        actor: Address,
        new_curator: Address,
    ) -> Result<(), ContractError> {
        actor.require_auth();
        let mut col = Self::require_collection(env, id)?;
        Self::require_curator(&col, &actor)?;
        if &col.creator == &new_curator || col.curators.iter().any(|c| &c == &new_curator) {
            // Already a curator/creator; idempotent no-op with error to surface intent.
            return Err(ContractError::AlreadyMaintainerAdded);
        }
        if col.curators.len() >= MAX_COMMUNITY_COL_CURATORS {
            return Err(ContractError::CommunityColCuratorsEmpty);
        }
        Self::validate_revenue_share(col.creator_revenue_share_bps, col.curators.len() + 1)?;
        col.curators.push_back(new_curator.clone());
        col.updated_at = env.ledger().timestamp();
        Self::write_collection(env, &col);
        Self::append_to_list(env, CCKey::ByCurator(new_curator.clone()), id);
        StorageManager::extend_if_curator_list(env, &col.curators);
        publish_community_col_curators_changed_event(env, id, actor);
        Ok(())
    }

    pub fn remove_curator(
        env: &Env,
        id: u64,
        actor: Address,
        curator_to_remove: Address,
    ) -> Result<(), ContractError> {
        actor.require_auth();
        let mut col = Self::require_collection(env, id)?;
        Self::require_curator(&col, &actor)?;
        if &col.creator == &curator_to_remove {
            return Err(ContractError::CommunityColCreatorIsImmutable);
        }
        if col.curators.len() == 1 {
            // After removal the collection would have zero curators. The creator still
            // acts as curator, but we keep at least one named curator + creator.
            return Err(ContractError::CommunityColCuratorsEmpty);
        }
        let mut new_curators: Vec<Address> = Vec::new(env);
        let mut removed = false;
        for c in col.curators.iter() {
            if !removed && &c == &curator_to_remove {
                removed = true;
                continue;
            }
            new_curators.push_back(c);
        }
        if !removed {
            return Err(ContractError::CommunityColNotCurator);
        }
        col.curators = new_curators;
        col.updated_at = env.ledger().timestamp();
        Self::write_collection(env, &col);
        Self::remove_from_list(env, CCKey::ByCurator(curator_to_remove), &id);
        publish_community_col_curators_changed_event(env, id, actor);
        Ok(())
    }

    pub fn curator_add_project(
        env: &Env,
        id: u64,
        curator: Address,
        project_id: u64,
    ) -> Result<(), ContractError> {
        curator.require_auth();
        let col = Self::require_collection(env, id)?;
        Self::require_not_template(&col)?;
        Self::require_curator(&col, &curator)?;
        if ProjectRegistry::get_project(env, project_id).is_none() {
            return Err(ContractError::ProjectNotFound);
        }
        Self::include_project(env, &col, project_id, curator, true)
    }

    pub fn curator_remove_project(
        env: &Env,
        id: u64,
        curator: Address,
        project_id: u64,
    ) -> Result<(), ContractError> {
        curator.require_auth();
        let col = Self::require_collection(env, id)?;
        Self::require_not_template(&col)?;
        Self::require_curator(&col, &curator)?;
        Self::exclude_project(env, &col, project_id, curator, true)
    }

    // ── Voting / Curation (AC2) ───────────────────────────────────────────

    pub fn cast_vote(
        env: &Env,
        id: u64,
        voter: Address,
        project_id: u64,
        approve: bool,
    ) -> Result<(), ContractError> {
        voter.require_auth();
        let col = Self::require_collection(env, id)?;
        Self::require_not_template(&col)?;
        // Templates don't accept votes (already covered above).
        if ProjectRegistry::get_project(env, project_id).is_none() {
            return Err(ContractError::ProjectNotFound);
        }
        if col.approval_threshold == 0 {
            // Voting disabled for this collection — use curator direct-add instead.
            return Err(ContractError::CommunityColThresholdInvalid);
        }

        let vote_key = CCKey::Vote(id, project_id, voter.clone());
        if env.storage().persistent().has(&vote_key) {
            return Err(ContractError::CommunityColVoteAlreadyCast);
        }

        let vote = CommunityCollectionVote {
            collection_id: id,
            project_id,
            voter: voter.clone(),
            approve,
            created_at: env.ledger().timestamp(),
        };
        env.storage().persistent().set(&vote_key, &vote);

        let (new_ap, new_dp) = if approve {
            let a = env
                .storage()
                .persistent()
                .get::<_, u32>(&CCKey::ApprovalCount(id, project_id))
                .unwrap_or(0)
                .saturating_add(1);
            env.storage().persistent().set(&CCKey::ApprovalCount(id, project_id), &a);
            let d = env
                .storage()
                .persistent()
                .get::<_, u32>(&CCKey::DisapprovalCount(id, project_id))
                .unwrap_or(0);
            (a, d)
        } else {
            let d = env
                .storage()
                .persistent()
                .get::<_, u32>(&CCKey::DisapprovalCount(id, project_id))
                .unwrap_or(0)
                .saturating_add(1);
            env.storage().persistent().set(&CCKey::DisapprovalCount(id, project_id), &d);
            let a = env
                .storage()
                .persistent()
                .get::<_, u32>(&CCKey::ApprovalCount(id, project_id))
                .unwrap_or(0);
            (a, d)
        };
        publish_community_col_vote_cast_event(env, id, project_id, voter, approve, new_ap, new_dp);

        // Apply threshold-crossing effects if curator has not already decided.
        let curator_included: bool = env
            .storage()
            .persistent()
            .get(&CCKey::CuratorIncluded(id, project_id))
            .unwrap_or(false);
        let curator_excluded: bool = env
            .storage()
            .persistent()
            .get(&CCKey::CuratorExcluded(id, project_id))
            .unwrap_or(false);

        if approve && !curator_included && !curator_excluded && new_ap >= col.approval_threshold {
            // Use a "system" actor = col.creator. Event will show creator as the actor,
            // but by_curator = false lets indexers know it came from the vote threshold.
            let _ = Self::include_project(env, &col, project_id, col.creator.clone(), false);
        } else if !approve
            && !curator_included
            && !curator_excluded
            && new_dp >= col.disapproval_threshold
        {
            // Best-effort exclude: only errors if already not included, which we can ignore
            // (a project can cross disapproval threshold before it was ever added, meaning
            //  we just never add it).
            let _ = Self::exclude_project(env, &col, project_id, col.creator.clone(), false);
        }

        Ok(())
    }

    pub fn get_vote(
        env: &Env,
        id: u64,
        project_id: u64,
        voter: Address,
    ) -> Option<CommunityCollectionVote> {
        env.storage().persistent().get(&CCKey::Vote(id, project_id, voter))
    }

    pub fn get_inclusion_status(
        env: &Env,
        id: u64,
        project_id: u64,
    ) -> CommunityColInclusionStatus {
        let ids = Self::get_project_ids(env, id);
        let is_in = ids.iter().any(|pid| pid == project_id);
        if is_in {
            return CommunityColInclusionStatus::Included;
        }
        let ap = env
            .storage()
            .persistent()
            .get::<_, u32>(&CCKey::ApprovalCount(id, project_id))
            .unwrap_or(0);
        let dp = env
            .storage()
            .persistent()
            .get::<_, u32>(&CCKey::DisapprovalCount(id, project_id))
            .unwrap_or(0);
        if ap == 0 && dp == 0 {
            return CommunityColInclusionStatus::Excluded;
        }
        CommunityColInclusionStatus::Pending
    }

    // ── Featured Collections (AC1) ────────────────────────────────────────

    pub fn set_featured(
        env: &Env,
        admin: Address,
        id: u64,
        featured: bool,
    ) -> Result<(), ContractError> {
        require_admin_auth(env, &admin)?;
        let mut col = Self::require_collection(env, id)?;
        Self::require_not_template(&col)?;

        let mut ids: Vec<u64> = env
            .storage()
            .persistent()
            .get(&CCKey::FeaturedList)
            .unwrap_or_else(|| Vec::new(env));
        let already = ids.iter().any(|x| x == id);

        if featured && !already {
            if ids.len() >= MAX_FEATURED_COMMUNITY_COLLECTIONS {
                // FIFO evict oldest (front).
                let mut replaced = Vec::new(env);
                let l = ids.len();
                for i in 1..l {
                    if let Some(x) = ids.get(i) {
                        replaced.push_back(x);
                    }
                }
                ids = replaced;
            }
            ids.push_back(id);
            env.storage().persistent().set(&CCKey::FeaturedList, &ids);
            if let Some(oldest) = ids.get(0) {
                let oldest_id = oldest;
                // If we just inserted and list was over cap and evicted, reset
                // is_featured on the evicted entry.
                if let Some(mut evicted) = env.storage().persistent().get::<_, CommunityCollection>(
                    &CCKey::Collection(oldest_id),
                ) {
                    // If this specific call evicted a different entry than the
                    // newly featured one, unflag it.
                    let count_now = env
                        .storage()
                        .persistent()
                        .get::<_, Vec<u64>>(&CCKey::FeaturedList)
                        .unwrap_or_else(|| Vec::new(env))
                        .len();
                    // Because we evicted *before* push, FeaturedList length is now
                    // MAX_FEATURED_COMMUNITY_COLLECTIONS. The evicted id is no longer present.
                    // Check existence:
                    if evicted.is_featured && !env
                        .storage()
                        .persistent()
                        .get::<_, Vec<u64>>(&CCKey::FeaturedList)
                        .unwrap_or_else(|| Vec::new(env))
                        .iter()
                        .any(|x| x == evicted.id)
                    {
                        evicted.is_featured = false;
                        evicted.updated_at = env.ledger().timestamp();
                        Self::write_collection(env, &evicted);
                    }
                    // Silence unused var warning on edge case path.
                    let _ = count_now;
                }
            }
            col.is_featured = true;
            col.updated_at = env.ledger().timestamp();
            Self::write_collection(env, &col);
            publish_community_col_featured_event(env, id, admin.clone(), true);
            let at = if featured {
                AdminActionType::ProjectFeatured
            } else {
                AdminActionType::ProjectUnfeatured
            };
            AdminActionLog::record_action(env, admin, at, Some(id), None, None);
        } else if !featured && already {
            let updated = Utils::remove_item_from_vec(env, &ids, &id);
            env.storage().persistent().set(&CCKey::FeaturedList, &updated);
            col.is_featured = false;
            col.updated_at = env.ledger().timestamp();
            Self::write_collection(env, &col);
            publish_community_col_featured_event(env, id, admin.clone(), false);
            AdminActionLog::record_action(
                env,
                admin,
                AdminActionType::ProjectUnfeatured,
                Some(id),
                None,
                None,
            );
        }
        Ok(())
    }

    pub fn list_featured(
        env: &Env,
        start_index: u32,
        limit: u32,
    ) -> Vec<CommunityCollection> {
        let ids: Vec<u64> = env
            .storage()
            .persistent()
            .get(&CCKey::FeaturedList)
            .unwrap_or_else(|| Vec::new(env));
        let paged = paginate(env, &ids, start_index, limit);
        let mut out = Vec::new(env);
        for id in paged.iter() {
            if let Some(col) = Self::get(env, id) {
                out.push_back(col);
            }
        }
        out
    }

    pub fn get_featured_count(env: &Env) -> u32 {
        env.storage()
            .persistent()
            .get::<_, Vec<u64>>(&CCKey::FeaturedList)
            .map(|v| v.len())
            .unwrap_or(0)
    }

    // ── Revenue Sharing (AC3) ─────────────────────────────────────────────

    /// Record revenue attribution to a collection. Does not perform token
    /// transfers on-chain (the current contract has no token mint/transfer
    /// capability and fees are handled off-ledger by `FeeManager`). Instead,
    /// this accumulates cumulative attributed totals that an off-chain
    /// indexer + payout script can consume. Revenues are split according to
    /// `creator_revenue_share_bps`; the remaining portion is split evenly
    /// across the curator set (the indexer can do the final per-curator math
    /// at payout time using `curators` from the `CommunityCollection` struct).
    pub fn attribute_revenue(
        env: &Env,
        caller: Address,
        id: u64,
        total_amount_scaled: u128,
    ) -> Result<CommunityColRevenueSnapshot, ContractError> {
        // Attributor must be authenticated. In a future version this might be
        // restricted to the fee collector admin; today any authenticated
        // caller may record attributions so third-party indexers can plug in.
        caller.require_auth();
        let col = Self::require_collection(env, id)?;
        Self::require_not_template(&col)?;

        let share_bps = col.creator_revenue_share_bps as u128;
        let creator_amount = total_amount_scaled.saturating_mul(share_bps) / 10_000u128;
        let curators_amount = total_amount_scaled.saturating_sub(creator_amount);

        let existing_creator: u128 = env
            .storage()
            .persistent()
            .get(&CCKey::CreatorRevenueCumulative(id))
            .unwrap_or(0);
        let existing_curators: u128 = env
            .storage()
            .persistent()
            .get(&CCKey::CuratorsRevenueCumulative(id))
            .unwrap_or(0);
        let existing_count: u64 = env
            .storage()
            .persistent()
            .get(&CCKey::RevenueEventCount(id))
            .unwrap_or(0);

        let new_creator = existing_creator.saturating_add(creator_amount);
        let new_curators = existing_curators.saturating_add(curators_amount);
        let new_total = new_creator.saturating_add(new_curators);
        let new_count = existing_count.saturating_add(1);

        env.storage()
            .persistent()
            .set(&CCKey::CreatorRevenueCumulative(id), &new_creator);
        env.storage()
            .persistent()
            .set(&CCKey::CuratorsRevenueCumulative(id), &new_curators);
        env.storage()
            .persistent()
            .set(&CCKey::RevenueEventCount(id), &new_count);

        publish_community_col_revenue_attributed_event(
            env,
            id,
            caller,
            total_amount_scaled,
            creator_amount,
            curators_amount,
            new_creator,
            new_curators,
            new_total,
        );

        let snap = CommunityColRevenueSnapshot {
            collection_id: id,
            creator_cumulative_attributed: new_creator,
            curators_cumulative_attributed: new_curators,
            total_cumulative_attributed: new_total,
            as_of_timestamp: env.ledger().timestamp(),
        };
        StorageManager::extend_community_collection_ttl(env, id);
        Ok(snap)
    }

    pub fn get_revenue_snapshot(
        env: &Env,
        id: u64,
    ) -> Option<CommunityColRevenueSnapshot> {
        let col = Self::require_collection(env, id).ok()?;
        if col.is_template {
            return None;
        }
        let creator_cum: u128 = env
            .storage()
            .persistent()
            .get(&CCKey::CreatorRevenueCumulative(id))
            .unwrap_or(0);
        let curators_cum: u128 = env
            .storage()
            .persistent()
            .get(&CCKey::CuratorsRevenueCumulative(id))
            .unwrap_or(0);
        Some(CommunityColRevenueSnapshot {
            collection_id: id,
            creator_cumulative_attributed: creator_cum,
            curators_cumulative_attributed: curators_cum,
            total_cumulative_attributed: creator_cum.saturating_add(curators_cum),
            as_of_timestamp: env.ledger().timestamp(),
        })
    }
}

// Local helper: turn `Env` into an owned Env for name-normalization — matches
// `Utils::normalize_project_name(env, name)` signature.
fn env(e: &Env) -> Env {
    e.clone()
}
