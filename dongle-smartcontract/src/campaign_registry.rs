//! Time-bound feature campaigns (issue #926).
//!
//! Admins curate themed campaigns (special events, ecosystem themes) that
//! highlight a set of projects between `start_date` (inclusive) and
//! `end_date` (exclusive). Viewers' impressions and engagements are counted
//! per campaign and per project while the campaign is active.

use crate::admin_action_log::AdminActionLog;
use crate::auth::require_admin_auth;
use crate::constants::{
    MAX_CAMPAIGN_DURATION_SECS, MAX_CAMPAIGN_NAME_LEN, MAX_CAMPAIGN_THEME_LEN,
    MAX_PROJECTS_PER_CAMPAIGN,
};
use crate::errors::ContractError;
use crate::events::{
    publish_campaign_interaction_event, publish_feature_campaign_created_event,
    publish_project_added_to_campaign_event,
};
use crate::pagination::paginate;
use crate::storage_keys::{CampaignKey, StorageKey};
use crate::types::{
    AdminActionType, CampaignInteraction, CampaignStats, FeatureCampaign, FeatureCampaignConfig,
    Project,
};
use soroban_sdk::{Address, Env, Vec};

pub struct CampaignRegistry;

impl CampaignRegistry {
    /// Create a feature campaign. Admin-only. Returns the new campaign ID.
    pub fn create_feature_campaign(
        env: &Env,
        admin: Address,
        config: FeatureCampaignConfig,
    ) -> Result<u64, ContractError> {
        require_admin_auth(env, &admin)?;

        if config.name.is_empty()
            || config.name.len() > MAX_CAMPAIGN_NAME_LEN
            || config.theme.is_empty()
            || config.theme.len() > MAX_CAMPAIGN_THEME_LEN
        {
            return Err(ContractError::InvalidInput);
        }
        let now = env.ledger().timestamp();
        if config.end_date <= config.start_date
            || config.end_date <= now
            || config.end_date - config.start_date > MAX_CAMPAIGN_DURATION_SECS
        {
            return Err(ContractError::InvalidInput);
        }

        let id: u64 = env
            .storage()
            .persistent()
            .get(&CampaignKey::NextId)
            .unwrap_or(1);
        env.storage()
            .persistent()
            .set(&CampaignKey::NextId, &id.saturating_add(1));

        let campaign = FeatureCampaign {
            id,
            name: config.name,
            theme: config.theme,
            start_date: config.start_date,
            end_date: config.end_date,
            created_by: admin.clone(),
            created_at: now,
            project_count: 0,
        };
        env.storage()
            .persistent()
            .set(&CampaignKey::Campaign(id), &campaign);

        let mut ids: Vec<u64> = env
            .storage()
            .persistent()
            .get(&CampaignKey::CampaignIds)
            .unwrap_or_else(|| Vec::new(env));
        ids.push_back(id);
        env.storage().persistent().set(&CampaignKey::CampaignIds, &ids);

        publish_feature_campaign_created_event(env, &campaign);
        AdminActionLog::record_action(
            env,
            admin,
            AdminActionType::FeatureCampaignCreated,
            Some(id),
            None,
            None,
        );

        Ok(id)
    }

    /// Add a project to a campaign. Admin-only. Projects can be added until
    /// the campaign ends.
    pub fn add_to_campaign(
        env: &Env,
        admin: Address,
        project_id: u64,
        campaign_id: u64,
    ) -> Result<(), ContractError> {
        require_admin_auth(env, &admin)?;

        let mut campaign =
            Self::get_feature_campaign(env, campaign_id).ok_or(ContractError::CampaignNotFound)?;
        if env.ledger().timestamp() >= campaign.end_date {
            return Err(ContractError::CampaignNotActive);
        }
        if !env
            .storage()
            .persistent()
            .has(&StorageKey::Project(project_id))
        {
            return Err(ContractError::ProjectNotFound);
        }

        let mut projects = Self::campaign_project_ids(env, campaign_id);
        if projects.contains(project_id) {
            return Err(ContractError::AlreadyInCollection);
        }
        if projects.len() >= MAX_PROJECTS_PER_CAMPAIGN {
            return Err(ContractError::CampaignFull);
        }
        projects.push_back(project_id);
        env.storage()
            .persistent()
            .set(&CampaignKey::Projects(campaign_id), &projects);

        campaign.project_count = projects.len();
        env.storage()
            .persistent()
            .set(&CampaignKey::Campaign(campaign_id), &campaign);

        publish_project_added_to_campaign_event(env, campaign_id, project_id, admin.clone());
        AdminActionLog::record_action(
            env,
            admin,
            AdminActionType::ProjectAddedToCampaign,
            Some(project_id),
            None,
            None,
        );

        Ok(())
    }

    /// Projects in a campaign, in insertion order, paginated.
    pub fn get_projects_by_campaign(
        env: &Env,
        campaign_id: u64,
        start_index: u32,
        limit: u32,
    ) -> Result<Vec<Project>, ContractError> {
        if !env
            .storage()
            .persistent()
            .has(&CampaignKey::Campaign(campaign_id))
        {
            return Err(ContractError::CampaignNotFound);
        }
        let ids = Self::campaign_project_ids(env, campaign_id);
        let page = paginate(env, &ids, start_index, limit);
        let mut out = Vec::new(env);
        for project_id in page.iter() {
            if let Some(project) = env
                .storage()
                .persistent()
                .get::<_, Project>(&StorageKey::Project(project_id))
            {
                out.push_back(project);
            }
        }
        Ok(out)
    }

    /// Record a viewer impression or engagement for a project shown as part
    /// of an active campaign. The viewer must sign the call.
    pub fn record_campaign_interaction(
        env: &Env,
        viewer: Address,
        campaign_id: u64,
        project_id: u64,
        interaction: CampaignInteraction,
    ) -> Result<(), ContractError> {
        viewer.require_auth();

        let campaign =
            Self::get_feature_campaign(env, campaign_id).ok_or(ContractError::CampaignNotFound)?;
        if !Self::is_active(env, &campaign) {
            return Err(ContractError::CampaignNotActive);
        }
        if !Self::campaign_project_ids(env, campaign_id).contains(project_id) {
            return Err(ContractError::NotInCollection);
        }

        let mut total = Self::get_campaign_stats(env, campaign_id);
        let mut per_project = Self::get_campaign_project_stats(env, campaign_id, project_id);
        match interaction {
            CampaignInteraction::Impression => {
                total.impressions = total.impressions.saturating_add(1);
                per_project.impressions = per_project.impressions.saturating_add(1);
            }
            CampaignInteraction::Engagement => {
                total.engagements = total.engagements.saturating_add(1);
                per_project.engagements = per_project.engagements.saturating_add(1);
            }
        }
        env.storage()
            .persistent()
            .set(&CampaignKey::Stats(campaign_id), &total);
        env.storage().persistent().set(
            &CampaignKey::ProjectStats(campaign_id, project_id),
            &per_project,
        );

        publish_campaign_interaction_event(env, campaign_id, project_id, viewer, interaction);
        Ok(())
    }

    pub fn get_feature_campaign(env: &Env, campaign_id: u64) -> Option<FeatureCampaign> {
        env.storage()
            .persistent()
            .get(&CampaignKey::Campaign(campaign_id))
    }

    /// All campaigns in creation order, paginated.
    pub fn list_feature_campaigns(env: &Env, start_index: u32, limit: u32) -> Vec<FeatureCampaign> {
        let ids: Vec<u64> = env
            .storage()
            .persistent()
            .get(&CampaignKey::CampaignIds)
            .unwrap_or_else(|| Vec::new(env));
        let page = paginate(env, &ids, start_index, limit);
        let mut out = Vec::new(env);
        for id in page.iter() {
            if let Some(campaign) = Self::get_feature_campaign(env, id) {
                out.push_back(campaign);
            }
        }
        out
    }

    /// Campaigns whose `start_date..end_date` window contains the current
    /// ledger timestamp.
    pub fn get_active_campaigns(env: &Env) -> Vec<FeatureCampaign> {
        let ids: Vec<u64> = env
            .storage()
            .persistent()
            .get(&CampaignKey::CampaignIds)
            .unwrap_or_else(|| Vec::new(env));
        let mut out = Vec::new(env);
        for id in ids.iter() {
            if let Some(campaign) = Self::get_feature_campaign(env, id) {
                if Self::is_active(env, &campaign) {
                    out.push_back(campaign);
                }
            }
        }
        out
    }

    pub fn get_campaign_stats(env: &Env, campaign_id: u64) -> CampaignStats {
        env.storage()
            .persistent()
            .get(&CampaignKey::Stats(campaign_id))
            .unwrap_or(CampaignStats {
                impressions: 0,
                engagements: 0,
            })
    }

    pub fn get_campaign_project_stats(
        env: &Env,
        campaign_id: u64,
        project_id: u64,
    ) -> CampaignStats {
        env.storage()
            .persistent()
            .get(&CampaignKey::ProjectStats(campaign_id, project_id))
            .unwrap_or(CampaignStats {
                impressions: 0,
                engagements: 0,
            })
    }

    fn is_active(env: &Env, campaign: &FeatureCampaign) -> bool {
        let now = env.ledger().timestamp();
        now >= campaign.start_date && now < campaign.end_date
    }

    fn campaign_project_ids(env: &Env, campaign_id: u64) -> Vec<u64> {
        env.storage()
            .persistent()
            .get(&CampaignKey::Projects(campaign_id))
            .unwrap_or_else(|| Vec::new(env))
    }
}
