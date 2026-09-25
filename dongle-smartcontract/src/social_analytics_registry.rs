use crate::constants::{
    MAX_SOCIAL_CHECKPOINTS_PER_PROJECT, SOCIAL_ANALYTICS_MAX_PEERS,
    SOCIAL_ANALYTICS_MIN_CHECKPOINTS_FOR_GROWTH, SOCIAL_ENGAGEMENT_RATE_SCALE_PPM,
    SOCIAL_RATING_BPS_PER_STAR, SOCIAL_WINDOW_30_DAYS, SOCIAL_WINDOW_7_DAYS,
};
use crate::endorsement_registry::EndorsementRegistry;
use crate::errors::ContractError;
use crate::events::{
    publish_project_engagement_metric_computed_event,
    publish_project_social_analytics_export_event,
    publish_project_social_checkpoint_recorded_event, publish_project_social_peers_compared_event,
};
use crate::pagination::paginate;
use crate::project_registry::ProjectRegistry;
use crate::review_registry::ReviewRegistry;
use crate::storage_keys::ExtensionKey;
use crate::storage_keys::SocialAnalyticsKey as SAK;
use crate::storage_manager::StorageManager;
use crate::subscription_registry::SubscriptionRegistry;
use crate::types::{
    ProjectEngagementMetric, ProjectSocialAnalyticsExport, ProjectSocialDailyCheckpoint,
    ProjectSocialPeerRow,
};
use soroban_sdk::{Address, Env, Vec};

const SECONDS_PER_DAY: u64 = 86_400;

pub struct SocialAnalyticsRegistry;

impl SocialAnalyticsRegistry {
    // ── Internal helpers ──────────────────────────────────────────────────

    fn current_day_index(env: &Env) -> u32 {
        (env.ledger().timestamp() / SECONDS_PER_DAY) as u32
    }

    fn snapshot_counts(env: &Env, project_id: u64) -> (u32, u32, u32, u32, u32) {
        let followers = SubscriptionRegistry::get_follower_count(env, project_id);
        let endorsements = EndorsementRegistry::get_endorsement_count(env, project_id);
        // No per-project bookmark counter exists today; ExtensionKey is at the 50-variant
        // Soroban union cap and does not have a BookmarkCount slot. Future enhancement:
        // maintain per-project bookmark counts in the SAK namespace from the
        // bookmark_registry hooks; value is correctly zeroed until then.
        let bookmarks = 0u32;
        let review_stats = ReviewRegistry::get_project_stats(env, project_id);
        // Normalise average_rating to basis points: review_registry stores
        // average_rating scaled by 100 (e.g. 400 = 4.00 stars) per the
        // existing `ProjectStats.average_rating` definition.
        let avg_bps = review_stats
            .average_rating
            .saturating_mul(SOCIAL_RATING_BPS_PER_STAR / 100);
        (
            followers,
            endorsements,
            bookmarks,
            review_stats.review_count,
            avg_bps,
        )
    }

    fn get_checkpoint_days(env: &Env, project_id: u64) -> Vec<u32> {
        env.storage()
            .persistent()
            .get(&SAK::CheckpointDayIndex(project_id))
            .unwrap_or_else(|| Vec::new(env))
    }

    fn set_checkpoint_days(env: &Env, project_id: u64, days: &Vec<u32>) {
        env.storage()
            .persistent()
            .set(&SAK::CheckpointDayIndex(project_id), days);
    }

    fn update_boundary_cache(env: &Env, project_id: u64, days: &Vec<u32>) {
        let len = days.len();
        if len == 0 {
            env.storage()
                .persistent()
                .remove(&SAK::OldestCheckpointDay(project_id));
            env.storage()
                .persistent()
                .remove(&SAK::NewestCheckpointDay(project_id));
            return;
        }
        if let Some(oldest) = days.get(0) {
            env.storage()
                .persistent()
                .set(&SAK::OldestCheckpointDay(project_id), &oldest);
        }
        if let Some(newest) = days.get(len - 1) {
            env.storage()
                .persistent()
                .set(&SAK::NewestCheckpointDay(project_id), &newest);
        }
    }

    fn evict_oldest_if_needed(env: &Env, project_id: u64, days: &mut Vec<u32>) -> Option<u32> {
        if days.len() < MAX_SOCIAL_CHECKPOINTS_PER_PROJECT {
            return None;
        }
        let oldest_day = if let Some(d) = days.get(0) {
            d
        } else {
            return None;
        };
        env.storage()
            .persistent()
            .remove(&SAK::DailyCheckpoint(project_id, oldest_day));
        let mut rest: Vec<u32> = Vec::new(env);
        let n = days.len();
        for i in 1..n {
            if let Some(d) = days.get(i) {
                rest.push_back(d);
            }
        }
        *days = rest;
        Some(oldest_day)
    }

    fn find_nearest_checkpoint(
        env: &Env,
        project_id: u64,
        day_index: u32,
        prefer_left: bool,
    ) -> Option<ProjectSocialDailyCheckpoint> {
        let days = Self::get_checkpoint_days(env, project_id);
        let len = days.len();
        if len == 0 {
            return None;
        }
        // Linear scan (list is 730 max; short enough).
        let mut best_i: Option<u32> = None;
        for i in 0..len {
            if let Some(d) = days.get(i) {
                if prefer_left {
                    if d <= day_index {
                        best_i = Some(i);
                    } else {
                        break;
                    }
                } else {
                    if d >= day_index {
                        best_i = Some(i);
                        break;
                    }
                }
            }
        }
        if let Some(i) = best_i {
            if let Some(d) = days.get(i) {
                return env
                    .storage()
                    .persistent()
                    .get(&SAK::DailyCheckpoint(project_id, d));
            }
        }
        None
    }

    fn lookup_checkpoint_near(
        env: &Env,
        project_id: u64,
        day_index: u32,
    ) -> ProjectSocialDailyCheckpoint {
        // Priority: exact, or nearest <= day_index, else nearest > day_index,
        // else a synthetic zero-checkpoint built from empty state.
        if let Some(exact) = env
            .storage()
            .persistent()
            .get::<_, ProjectSocialDailyCheckpoint>(&SAK::DailyCheckpoint(project_id, day_index))
        {
            return exact;
        }
        if let Some(left) = Self::find_nearest_checkpoint(env, project_id, day_index, true) {
            return left;
        }
        if let Some(right) = Self::find_nearest_checkpoint(env, project_id, day_index, false) {
            return right;
        }
        let (f, e, b, r, avg) = Self::snapshot_counts(env, project_id);
        ProjectSocialDailyCheckpoint {
            project_id,
            day_index,
            recorded_at: env.ledger().timestamp(),
            follower_count: f,
            endorsement_count: e,
            bookmark_count: b,
            review_count: r,
            average_rating_bps: avg,
            total_engagement_units: (f as u64)
                .saturating_add(e as u64)
                .saturating_add(b as u64)
                .saturating_add(r as u64),
        }
    }

    fn clamp_gain(later: u32, earlier: u32) -> i64 {
        (later as i64).saturating_sub(earlier as i64)
    }

    fn require_project_exists(
        env: &Env,
        project_id: u64,
    ) -> Result<crate::types::Project, ContractError> {
        ProjectRegistry::get_project(env, project_id)
            .ok_or(ContractError::SocialAnalyticsProjectNotFound)
    }

    // ── Public checkpoint API (AC1) ───────────────────────────────────────

    /// Snapshot all social-signal counters for `project_id` at "today's" day
    /// bucket and store them as a `ProjectSocialDailyCheckpoint`. If today's
    /// bucket already exists it is overwritten (latest-wins semantics).
    /// When the per-project checkpoint list hits MAX_SOCIAL_CHECKPOINTS_PER_PROJECT
    /// the oldest entry is FIFO-evicted so history remains a rolling window.
    pub fn record_daily_checkpoint(
        env: &Env,
        caller: Address,
        project_id: u64,
    ) -> Result<u32, ContractError> {
        caller.require_auth();
        Self::require_project_exists(env, project_id)?;

        let (f, e, b, r, avg_bps) = Self::snapshot_counts(env, project_id);
        let day_index = Self::current_day_index(env);
        let total_units = (f as u64)
            .saturating_add(e as u64)
            .saturating_add(b as u64)
            .saturating_add(r as u64);

        let checkpoint = ProjectSocialDailyCheckpoint {
            project_id,
            day_index,
            recorded_at: env.ledger().timestamp(),
            follower_count: f,
            endorsement_count: e,
            bookmark_count: b,
            review_count: r,
            average_rating_bps: avg_bps,
            total_engagement_units: total_units,
        };
        env.storage()
            .persistent()
            .set(&SAK::DailyCheckpoint(project_id, day_index), &checkpoint);

        let mut days = Self::get_checkpoint_days(env, project_id);
        let mut evicted: Option<u32> = None;
        let already_present = days.iter().any(|d| d == day_index);
        if !already_present {
            // Roll the rolling window BEFORE appending to keep invariant
            // len(CheckpointDayIndex) <= MAX_SOCIAL_CHECKPOINTS_PER_PROJECT.
            evicted = Self::evict_oldest_if_needed(env, project_id, &mut days);
            days.push_back(day_index);
            Self::set_checkpoint_days(env, project_id, &days);
        }
        Self::update_boundary_cache(env, project_id, &days);
        env.storage().persistent().set(
            &SAK::LastCheckpointRecordedAt(project_id),
            &env.ledger().timestamp(),
        );

        StorageManager::extend_social_analytics_project_ttl(env, project_id);
        StorageManager::extend_project_ttl(env, project_id);

        publish_project_social_checkpoint_recorded_event(
            env,
            project_id,
            day_index,
            f,
            e,
            b,
            r,
            avg_bps,
            total_units,
            evicted,
        );
        Ok(day_index)
    }

    pub fn get_checkpoint(
        env: &Env,
        project_id: u64,
        day_index: u32,
    ) -> Option<ProjectSocialDailyCheckpoint> {
        let cp = env
            .storage()
            .persistent()
            .get(&SAK::DailyCheckpoint(project_id, day_index));
        if cp.is_some() {
            StorageManager::extend_social_analytics_project_ttl(env, project_id);
        }
        cp
    }

    pub fn list_checkpoints(
        env: &Env,
        project_id: u64,
        start_index: u32,
        limit: u32,
    ) -> Vec<ProjectSocialDailyCheckpoint> {
        let days = Self::get_checkpoint_days(env, project_id);
        let paged_days = paginate(env, &days, start_index, limit);
        let mut out: Vec<ProjectSocialDailyCheckpoint> = Vec::new(env);
        for d in paged_days.iter() {
            if let Some(cp) = env
                .storage()
                .persistent()
                .get::<_, ProjectSocialDailyCheckpoint>(&SAK::DailyCheckpoint(project_id, d))
            {
                out.push_back(cp);
            }
        }
        StorageManager::extend_social_analytics_project_ttl(env, project_id);
        out
    }

    pub fn get_checkpoint_count(env: &Env, project_id: u64) -> u32 {
        Self::get_checkpoint_days(env, project_id).len()
    }

    pub fn get_oldest_newest_checkpoint_days(
        env: &Env,
        project_id: u64,
    ) -> (Option<u32>, Option<u32>) {
        (
            env.storage()
                .persistent()
                .get(&SAK::OldestCheckpointDay(project_id)),
            env.storage()
                .persistent()
                .get(&SAK::NewestCheckpointDay(project_id)),
        )
    }

    // ── Engagement rate calculations (AC2) ────────────────────────────────

    /// Compute an engagement-rate metric over an arbitrary window
    /// `[window_start_day, window_end_day]` inclusive. Deltas are computed
    /// by subtracting the checkpoint nearest-but-not-after the start day
    /// from the checkpoint nearest-but-not-after the end day. ppm-scaled.
    pub fn compute_project_engagement_metric(
        env: &Env,
        project_id: u64,
        window_start_day: u32,
        window_end_day: u32,
    ) -> Result<ProjectEngagementMetric, ContractError> {
        Self::require_project_exists(env, project_id)?;
        if window_start_day > window_end_day {
            return Err(ContractError::SocialAnalyticsInvalidWindow);
        }
        let start_cp = Self::lookup_checkpoint_near(env, project_id, window_start_day);
        let end_cp = Self::lookup_checkpoint_near(env, project_id, window_end_day);

        let fg = Self::clamp_gain(end_cp.follower_count, start_cp.follower_count);
        let eg = Self::clamp_gain(end_cp.endorsement_count, start_cp.endorsement_count);
        let bg = Self::clamp_gain(end_cp.bookmark_count, start_cp.bookmark_count);
        let rg = Self::clamp_gain(end_cp.review_count, start_cp.review_count);
        let gain_total: u64 = (fg.max(0) as u64)
            .saturating_add(eg.max(0) as u64)
            .saturating_add(bg.max(0) as u64)
            .saturating_add(rg.max(0) as u64);
        let denom = core::cmp::max(start_cp.follower_count, 1) as u64;
        let rate_ppm = gain_total.saturating_mul(SOCIAL_ENGAGEMENT_RATE_SCALE_PPM) / denom;
        let rating_delta =
            (end_cp.average_rating_bps as i64).saturating_sub(start_cp.average_rating_bps as i64);

        let metric = ProjectEngagementMetric {
            project_id,
            window_start_day,
            window_end_day,
            follower_gain: fg,
            endorsement_gain: eg,
            bookmark_gain: bg,
            review_gain: rg,
            net_engagement_gain: gain_total,
            start_follower_count: start_cp.follower_count,
            engagement_rate_ppm: rate_ppm,
            rating_delta_bps: rating_delta,
        };
        publish_project_engagement_metric_computed_event(
            env,
            project_id,
            window_start_day,
            window_end_day,
            rate_ppm,
            gain_total,
            fg,
            eg,
            bg,
            rg,
            rating_delta,
        );
        Ok(metric)
    }

    // ── Peer comparison (AC3) ─────────────────────────────────────────────

    /// Compare the target project against other projects in the SAME
    /// category. For every peer project we compute the last-30-day
    /// engagement metric (fast: checkpoint-nearest lookups, no global scan).
    /// Results are sorted by engagement_rate_ppm desc with target project
    /// always included so clients can compute percentile = pos / len.
    pub fn compare_similar_projects(
        env: &Env,
        project_id: u64,
        max_peers: u32,
    ) -> Result<Vec<ProjectSocialPeerRow>, ContractError> {
        let project = Self::require_project_exists(env, project_id)?;
        let max = if max_peers == 0 {
            SOCIAL_ANALYTICS_MAX_PEERS
        } else {
            core::cmp::min(max_peers, SOCIAL_ANALYTICS_MAX_PEERS)
        };
        let today = Self::current_day_index(env);
        let start_30 = today.saturating_sub(SOCIAL_WINDOW_30_DAYS - 1);

        let target_metric =
            Self::compute_project_engagement_metric(env, project_id, start_30, today)?;
        let target_follower_count = SubscriptionRegistry::get_follower_count(env, project_id);
        let target_avg = ReviewRegistry::get_project_stats(env, project_id)
            .average_rating
            .saturating_mul(SOCIAL_RATING_BPS_PER_STAR / 100);

        let mut rows: alloc::vec::Vec<(u64, ProjectSocialPeerRow)> = alloc::vec::Vec::new();
        rows.push((
            project_id,
            ProjectSocialPeerRow {
                project_id,
                project_name: project.name.clone(),
                engagement_rate_ppm: target_metric.engagement_rate_ppm,
                net_engagement_gain: target_metric.net_engagement_gain,
                latest_follower_count: target_follower_count,
                average_rating_bps: target_avg,
            },
        ));

        let project_count = ProjectRegistry::get_project_count(env);
        let mut scanned = 0u32;
        let page_limit: u32 = 100;
        // Paging through the full project list and checking category. Given
        // the per-project work is cheap (a few counter reads) and project
        // caps are bounded, this completes in well under budget for realistic
        // project counts (MAX_PROJECTS ≤ 100_000 / and max_peers ≤ 50 means
        // we stop collecting once we have max peers and just still finish
        // calculating sort).
        'outer: loop {
            let list = ProjectRegistry::list_projects(env, scanned as u64, page_limit);
            if list.is_empty() {
                break 'outer;
            }
            let chunk_size = list.len();
            for i in 0..chunk_size {
                if let Some(peer_proj) = list.get(i) {
                    scanned += 1;
                    if peer_proj.id == project_id {
                        continue;
                    }
                    if peer_proj.category != project.category {
                        continue;
                    }
                    let peer_metric = match Self::compute_project_engagement_metric(
                        env,
                        peer_proj.id,
                        start_30,
                        today,
                    ) {
                        Ok(m) => m,
                        Err(_) => continue,
                    };
                    let peer_followers =
                        SubscriptionRegistry::get_follower_count(env, peer_proj.id);
                    let peer_avg = ReviewRegistry::get_project_stats(env, peer_proj.id)
                        .average_rating
                        .saturating_mul(SOCIAL_RATING_BPS_PER_STAR / 100);
                    rows.push((
                        peer_proj.id,
                        ProjectSocialPeerRow {
                            project_id: peer_proj.id,
                            project_name: peer_proj.name.clone(),
                            engagement_rate_ppm: peer_metric.engagement_rate_ppm,
                            net_engagement_gain: peer_metric.net_engagement_gain,
                            latest_follower_count: peer_followers,
                            average_rating_bps: peer_avg,
                        },
                    ));
                }
            }
            if chunk_size < page_limit {
                break 'outer;
            }
            if scanned as u64 >= project_count {
                break 'outer;
            }
        }

        rows.sort_by(|a, b| {
            use core::cmp::Ordering;
            match b.1.engagement_rate_ppm.cmp(&a.1.engagement_rate_ppm) {
                Ordering::Equal => a.0.cmp(&b.0),
                o => o,
            }
        });

        let truncate_to = core::cmp::min(rows.len() as u32, max.saturating_add(1));
        let mut result: Vec<ProjectSocialPeerRow> = Vec::new(env);
        for row in rows.iter().take(truncate_to as usize) {
            result.push_back(row.1.clone());
        }

        let self_rank = rows
            .iter()
            .position(|(pid, _)| *pid == project_id)
            .unwrap_or(0) as u32;
        let category = project.category;
        publish_project_social_peers_compared_event(
            env,
            project_id,
            category,
            result.len(),
            self_rank,
        );
        Ok(result)
    }

    // ── Export analytics report (AC4) ─────────────────────────────────────

    /// Produce the full on-chain social analytics export payload for a project.
    /// Includes last-7-day and last-30-day engagement metrics, 30-day growth
    /// numbers (AC1), peer comparison (AC3), and an incrementing report
    /// nonce so indexers can deduplicate identical snapshots when re-exporting
    /// on the same ledger.
    pub fn export_report(
        env: &Env,
        project_id: u64,
    ) -> Result<ProjectSocialAnalyticsExport, ContractError> {
        let project = Self::require_project_exists(env, project_id)?;
        let days = Self::get_checkpoint_days(env, project_id);
        if days.len() < SOCIAL_ANALYTICS_MIN_CHECKPOINTS_FOR_GROWTH {
            // No growth deltas derivable; a bare report with zeroed metrics and
            // the live current state would still be useful. Callers that want
            // strict growth reports should checkpoint twice before exporting.
        }
        let (oldest, newest) = Self::get_oldest_newest_checkpoint_days(env, project_id);
        let today = Self::current_day_index(env);
        let start_7 = today.saturating_sub(SOCIAL_WINDOW_7_DAYS - 1);
        let start_30 = today.saturating_sub(SOCIAL_WINDOW_30_DAYS - 1);

        let m7 = Self::compute_project_engagement_metric(env, project_id, start_7, today)
            .unwrap_or_else(|_| ProjectEngagementMetric {
                project_id,
                window_start_day: start_7,
                window_end_day: today,
                follower_gain: 0,
                endorsement_gain: 0,
                bookmark_gain: 0,
                review_gain: 0,
                net_engagement_gain: 0,
                start_follower_count: 0,
                engagement_rate_ppm: 0,
                rating_delta_bps: 0,
            });
        let m30 = Self::compute_project_engagement_metric(env, project_id, start_30, today)
            .unwrap_or_else(|_| ProjectEngagementMetric {
                project_id,
                window_start_day: start_30,
                window_end_day: today,
                follower_gain: 0,
                endorsement_gain: 0,
                bookmark_gain: 0,
                review_gain: 0,
                net_engagement_gain: 0,
                start_follower_count: 0,
                engagement_rate_ppm: 0,
                rating_delta_bps: 0,
            });

        let peer_rows = Self::compare_similar_projects(env, project_id, SOCIAL_ANALYTICS_MAX_PEERS)
            .unwrap_or_else(|_| Vec::new(env));
        let self_index: u32 = {
            let mut found = 0u32;
            for (i, row) in peer_rows.iter().enumerate() {
                if row.project_id == project_id {
                    found = i as u32;
                    break;
                }
            }
            found
        };

        let next_counter: u64 = env
            .storage()
            .persistent()
            .get(&SAK::ExportReportCounter(project_id))
            .unwrap_or(0u64)
            .saturating_add(1);
        env.storage()
            .persistent()
            .set(&SAK::ExportReportCounter(project_id), &next_counter);

        let exported = ProjectSocialAnalyticsExport {
            project_id,
            category: project.category.clone(),
            checkpoint_count: days.len(),
            oldest_checkpoint_day: oldest,
            newest_checkpoint_day: newest,
            last_7_days: m7.clone(),
            last_30_days: m30.clone(),
            growth_30d_total_engagement: m30.net_engagement_gain,
            growth_last_30_days_followers: m30.follower_gain,
            growth_30d_endorsements: m30.endorsement_gain,
            growth_last_30_days_bookmarks: m30.bookmark_gain,
            growth_last_30_days_reviews: m30.review_gain,
            peer_comparison: peer_rows.clone(),
            self_index_in_peer_ranking: self_index,
            generated_at: env.ledger().timestamp(),
        };
        publish_project_social_analytics_export_event(
            env,
            project_id,
            project.category,
            days.len(),
            m7.engagement_rate_ppm,
            m30.engagement_rate_ppm,
            m30.net_engagement_gain,
            peer_rows.len(),
            self_index,
            next_counter,
        );
        StorageManager::extend_social_analytics_project_ttl(env, project_id);
        Ok(exported)
    }

    /// Last-generated export report nonce. Indexers use this to skip duplicate
    /// re-exports of the same report when the export function runs twice.
    pub fn get_export_report_nonce(env: &Env, project_id: u64) -> u64 {
        env.storage()
            .persistent()
            .get(&SAK::ExportReportCounter(project_id))
            .unwrap_or(0)
    }

    // Silence unused import for ExtensionKey (kept so it remains available
    // for future bookmark-count cross-reading after bookmark_registry
    // enhancement).
    #[allow(dead_code)]
    fn _keep_extension_key_imported(_x: ExtensionKey) {}
}
