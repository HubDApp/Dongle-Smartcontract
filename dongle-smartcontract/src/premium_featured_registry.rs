//! Premium "featured plus" placement (closes #928).
//!
//! Project owners bid for a time-boxed featured placement in one of three
//! tiers ([`FeaturedTier::Standard`], [`FeaturedTier::Premium`],
//! [`FeaturedTier::Platinum`]). Admins configure each tier's fee structure
//! (`fee_per_day`) and capacity (`max_slots`).
//!
//! ## Guaranteed exposure
//! A successful bid creates a [`PremiumFeaturedCampaign`] that occupies one of
//! the tier's slots until `ends_at`. Active campaigns are never evicted or
//! outbid: when a tier is full, new bids are rejected with `PremiumTierFull`
//! until a slot frees up. Expired campaigns are pruned lazily on the next bid.
//!
//! ## Fees
//! The minimum bid is `fee_per_day * ceil(duration / 1 day)`. Bidders may bid
//! more; within a tier, higher bids are listed first. The full bid amount is
//! transferred to the treasury in the configured fee token.
//!
//! ## Tracking
//! Every campaign is kept under `ProjectCampaigns(project_id)` and aggregate
//! bid statistics (count, total, highest) are kept in [`PremiumFeaturedStats`].

use crate::admin_action_log::AdminActionLog;
use crate::auth::require_admin_auth;
use crate::constants::{
    MAX_PREMIUM_FEATURED_DURATION, MAX_PREMIUM_FEATURED_SLOTS, MIN_PREMIUM_FEATURED_DURATION,
    PREMIUM_FEATURED_DAY_SECONDS,
};
use crate::errors::ContractError;
use crate::events::{publish_premium_featured_bid_event, publish_premium_tier_configured_event};
use crate::fee_manager::FeeManager;
use crate::pagination::paginate;
use crate::project_registry::ProjectRegistry;
use crate::storage_keys::{PremiumFeaturedKey, StorageKey};
use crate::types::{
    AdminActionType, FeaturedTier, FeaturedTierConfig, PremiumFeaturedCampaign,
    PremiumFeaturedStats,
};
use crate::utils::Utils;
use soroban_sdk::{Address, Env, Vec};

pub struct PremiumFeaturedRegistry;

impl PremiumFeaturedRegistry {
    /// Configure the fee structure and slot capacity for a tier. Admin-only.
    ///
    /// Lowering `max_slots` below the number of active campaigns does not
    /// evict anyone; it only blocks new bids until campaigns expire.
    pub fn set_featured_tier_config(
        env: &Env,
        admin: Address,
        tier: FeaturedTier,
        fee_per_day: u128,
        max_slots: u32,
    ) -> Result<(), ContractError> {
        require_admin_auth(env, &admin)?;

        if max_slots == 0 || max_slots > MAX_PREMIUM_FEATURED_SLOTS {
            return Err(ContractError::InvalidInput);
        }

        let config = FeaturedTierConfig {
            tier,
            fee_per_day,
            max_slots,
        };
        env.storage()
            .persistent()
            .set(&PremiumFeaturedKey::TierConfig(tier), &config);

        publish_premium_tier_configured_event(env, admin.clone(), tier, fee_per_day, max_slots);
        AdminActionLog::record_action(
            env,
            admin,
            AdminActionType::FeeChanged,
            None,
            None,
            None,
        );
        Ok(())
    }

    pub fn get_featured_tier_config(env: &Env, tier: FeaturedTier) -> Option<FeaturedTierConfig> {
        env.storage()
            .persistent()
            .get(&PremiumFeaturedKey::TierConfig(tier))
    }

    /// Minimum bid for `duration` seconds in `tier`: `fee_per_day` times the
    /// number of started days.
    pub fn quote_premium_featured(
        env: &Env,
        tier: FeaturedTier,
        duration: u64,
    ) -> Result<u128, ContractError> {
        let config =
            Self::get_featured_tier_config(env, tier).ok_or(ContractError::PremiumTierNotConfigured)?;
        if !(MIN_PREMIUM_FEATURED_DURATION..=MAX_PREMIUM_FEATURED_DURATION).contains(&duration) {
            return Err(ContractError::InvalidPremiumDuration);
        }
        let days = duration.div_ceil(PREMIUM_FEATURED_DAY_SECONDS) as u128;
        config
            .fee_per_day
            .checked_mul(days)
            .ok_or(ContractError::ArithmeticOverflow)
    }

    /// Bid for a premium featured placement. Only the project owner may bid.
    ///
    /// On success the bid amount is transferred to the treasury and a campaign
    /// starting now and lasting `duration` seconds is created.
    pub fn bid_for_premium_featured(
        env: &Env,
        bidder: Address,
        project_id: u64,
        tier: FeaturedTier,
        duration: u64,
        bid_amount: u128,
    ) -> Result<u64, ContractError> {
        bidder.require_auth();

        let project =
            ProjectRegistry::get_project(env, project_id).ok_or(ContractError::ProjectNotFound)?;
        if project.owner != bidder {
            return Err(ContractError::Unauthorized);
        }
        if project.archived {
            return Err(ContractError::AlreadyArchived);
        }

        let config =
            Self::get_featured_tier_config(env, tier).ok_or(ContractError::PremiumTierNotConfigured)?;
        let min_bid = Self::quote_premium_featured(env, tier, duration)?;
        if bid_amount < min_bid {
            return Err(ContractError::PremiumBidTooLow);
        }

        if Self::get_active_campaign_for_project(env, project_id).is_some() {
            return Err(ContractError::ProjectAlreadyPremiumFeatured);
        }

        let mut slots = Self::prune_expired(env, tier);
        if slots.len() >= config.max_slots {
            return Err(ContractError::PremiumTierFull);
        }

        Self::collect_bid(env, &bidder, bid_amount)?;

        let campaign_id: u64 = env
            .storage()
            .persistent()
            .get(&PremiumFeaturedKey::NextCampaignId)
            .unwrap_or(1);
        let now = env.ledger().timestamp();
        let campaign = PremiumFeaturedCampaign {
            id: campaign_id,
            project_id,
            bidder,
            tier,
            bid_amount,
            starts_at: now,
            ends_at: now
                .checked_add(duration)
                .ok_or(ContractError::ArithmeticOverflow)?,
        };

        env.storage()
            .persistent()
            .set(&PremiumFeaturedKey::Campaign(campaign_id), &campaign);
        env.storage().persistent().set(
            &PremiumFeaturedKey::NextCampaignId,
            &campaign_id
                .checked_add(1)
                .ok_or(ContractError::ArithmeticOverflow)?,
        );

        slots.push_back(campaign_id);
        env.storage()
            .persistent()
            .set(&PremiumFeaturedKey::TierCampaigns(tier), &slots);

        let mut history = Self::get_project_campaign_ids(env, project_id);
        history.push_back(campaign_id);
        env.storage()
            .persistent()
            .set(&PremiumFeaturedKey::ProjectCampaigns(project_id), &history);

        let mut stats = Self::get_premium_featured_stats(env);
        stats.successful_campaigns = stats.successful_campaigns.saturating_add(1);
        stats.total_bid_amount = stats.total_bid_amount.saturating_add(bid_amount);
        if bid_amount > stats.highest_bid {
            stats.highest_bid = bid_amount;
        }
        env.storage()
            .persistent()
            .set(&PremiumFeaturedKey::Stats, &stats);

        publish_premium_featured_bid_event(env, &campaign);
        Ok(campaign_id)
    }

    /// Active premium featured campaigns, optionally filtered to one tier.
    ///
    /// Ordered Platinum → Premium → Standard, then by bid amount (highest
    /// first), then by campaign ID (earliest first).
    pub fn get_premium_featured_projects(
        env: &Env,
        tier: Option<FeaturedTier>,
        start_index: u32,
        limit: u32,
    ) -> Vec<PremiumFeaturedCampaign> {
        let tiers = match tier {
            Some(t) => Vec::from_array(env, [t]),
            None => Vec::from_array(
                env,
                [
                    FeaturedTier::Platinum,
                    FeaturedTier::Premium,
                    FeaturedTier::Standard,
                ],
            ),
        };

        let now = env.ledger().timestamp();
        let mut active = Vec::new(env);
        for t in tiers.iter() {
            let mut tier_campaigns: Vec<PremiumFeaturedCampaign> = Vec::new(env);
            for id in Self::get_tier_campaign_ids(env, t).iter() {
                if let Some(c) = Self::get_premium_campaign(env, id) {
                    if c.ends_at > now {
                        tier_campaigns.push_back(c);
                    }
                }
            }
            Utils::bubble_sort_by(&mut tier_campaigns, |a, b| {
                a.bid_amount < b.bid_amount || (a.bid_amount == b.bid_amount && a.id > b.id)
            });
            active.append(&tier_campaigns);
        }

        paginate(env, &active, start_index, limit)
    }

    pub fn get_premium_campaign(env: &Env, campaign_id: u64) -> Option<PremiumFeaturedCampaign> {
        env.storage()
            .persistent()
            .get(&PremiumFeaturedKey::Campaign(campaign_id))
    }

    /// Every campaign a project has won (active and expired), oldest first.
    pub fn get_project_premium_campaigns(env: &Env, project_id: u64) -> Vec<PremiumFeaturedCampaign> {
        let mut result = Vec::new(env);
        for id in Self::get_project_campaign_ids(env, project_id).iter() {
            if let Some(c) = Self::get_premium_campaign(env, id) {
                result.push_back(c);
            }
        }
        result
    }

    pub fn get_premium_featured_stats(env: &Env) -> PremiumFeaturedStats {
        env.storage()
            .persistent()
            .get(&PremiumFeaturedKey::Stats)
            .unwrap_or_default()
    }

    fn get_active_campaign_for_project(
        env: &Env,
        project_id: u64,
    ) -> Option<PremiumFeaturedCampaign> {
        let now = env.ledger().timestamp();
        let ids = Self::get_project_campaign_ids(env, project_id);
        // Campaigns are appended in time order, so only the latest can be active.
        let last = ids.last()?;
        Self::get_premium_campaign(env, last).filter(|c| c.ends_at > now)
    }

    /// Drop expired campaigns from a tier's slot list, persist and return it.
    fn prune_expired(env: &Env, tier: FeaturedTier) -> Vec<u64> {
        let now = env.ledger().timestamp();
        let ids = Self::get_tier_campaign_ids(env, tier);
        let mut active = Vec::new(env);
        for id in ids.iter() {
            if let Some(c) = Self::get_premium_campaign(env, id) {
                if c.ends_at > now {
                    active.push_back(id);
                }
            }
        }
        if active.len() != ids.len() {
            env.storage()
                .persistent()
                .set(&PremiumFeaturedKey::TierCampaigns(tier), &active);
        }
        active
    }

    /// Transfer the bid to the treasury in the configured fee token.
    fn collect_bid(env: &Env, bidder: &Address, amount: u128) -> Result<(), ContractError> {
        if amount == 0 {
            return Ok(());
        }
        if amount > i128::MAX as u128 {
            return Err(ContractError::InvalidProjectData);
        }
        let token = FeeManager::get_fee_config(env)?
            .token
            .ok_or(ContractError::FeeConfigNotSet)?;
        let treasury: Address = env
            .storage()
            .persistent()
            .get(&StorageKey::Treasury)
            .ok_or(ContractError::TreasuryNotSet)?;
        soroban_sdk::token::Client::new(env, &token).transfer(bidder, &treasury, &(amount as i128));
        Ok(())
    }

    fn get_tier_campaign_ids(env: &Env, tier: FeaturedTier) -> Vec<u64> {
        env.storage()
            .persistent()
            .get(&PremiumFeaturedKey::TierCampaigns(tier))
            .unwrap_or(Vec::new(env))
    }

    fn get_project_campaign_ids(env: &Env, project_id: u64) -> Vec<u64> {
        env.storage()
            .persistent()
            .get(&PremiumFeaturedKey::ProjectCampaigns(project_id))
            .unwrap_or(Vec::new(env))
    }
}
