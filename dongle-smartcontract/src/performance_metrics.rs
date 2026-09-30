//! Verification performance metrics tracking.
//!
//! Records and aggregates approval times, appeal rates, reversal rates, and
//! per-admin statistics in monthly windows. Percentile estimates are derived
//! from seven approval-time histogram buckets. Trend data spans up to 24 months.
//!
//! ## Acceptance criteria
//!
//! | Requirement | Implementation |
//! |-------------|----------------|
//! | Track approval time | `record_approval` — stores elapsed seconds; updates histogram & running sum |
//! | Track appeal rate | `record_appeal` — increments global + per-admin appeal counter |
//! | Track reversal rate | `record_reversal` — increments global + per-admin reversal counter |
//! | Calculate percentiles | `compute_percentile` — bucket-based P50 / P90 approximation |
//! | Calculate trends | `get_trend` — one `VerificationTrendPoint` per tracked month |
//! | Compare across admins | `get_monthly_report` — includes `AdminVerificationPerformance` per admin |
//! | Monthly performance reports | `get_monthly_report` / `get_monthly_report_by_month_num` |
//!
//! ## Storage layout
//!
//! All keys live in [`crate::storage_keys::PerformanceKey`].
//!
//! | Key | Type | Description |
//! |-----|------|-------------|
//! | `GlobalPerformance(month_num)` | `VerificationPerformanceSnapshot` | Aggregated global metrics per month |
//! | `AdminPerformance(addr, month_num)` | `AdminVerificationPerformance` | Per-admin metrics per month |
//! | `TrackedMonths` | `Vec<u32>` | Ordered list of month numbers seen (oldest first, capped at 24) |
//! | `MonthAdmins(month_num)` | `Vec<Address>` | Admins that acted in a given month |
//!
//! Month numbers use the compact `YYYYMM` encoding (e.g. `202609` for September 2026).
//!
//! ## Histogram buckets
//!
//! ```text
//! b0  < 1 h          (0 – 3 600 s)
//! b1  1 h – 6 h      (3 600 – 21 600 s)
//! b2  6 h – 1 d      (21 600 – 86 400 s)
//! b3  1 d – 3 d      (86 400 – 259 200 s)
//! b4  3 d – 7 d      (259 200 – 604 800 s)
//! b5  7 d – 30 d     (604 800 – 2 592 000 s)
//! b6  >= 30 d
//! ```

use crate::storage_keys::PerformanceKey;
use crate::types::{
    AdminVerificationPerformance, VerificationPerformanceReport, VerificationPerformanceSnapshot,
    VerificationTrendPoint,
};
use soroban_sdk::{Address, Env, Vec};

// ── Histogram configuration ──────────────────────────────────────────────────

/// Bucket upper bounds (exclusive) in seconds.
const BUCKET_BOUNDS: [u64; 6] = [
    3_600,      // b0 upper: 1 h
    21_600,     // b1 upper: 6 h
    86_400,     // b2 upper: 1 day
    259_200,    // b3 upper: 3 days
    604_800,    // b4 upper: 7 days
    2_592_000,  // b5 upper: 30 days
];

/// Representative midpoint for each bucket (used as percentile estimate).
const BUCKET_MIDPOINTS: [u64; 7] = [
    1_800,      // b0: ~30 min
    12_600,     // b1: ~3 h 30 min
    54_000,     // b2: ~15 h
    172_800,    // b3: ~2 days
    432_000,    // b4: ~5 days
    1_598_400,  // b5: ~18.5 days
    5_184_000,  // b6: ~60 days (lower-bound estimate for >= 30 d)
];

/// Maximum number of months retained in the trend list.
const MAX_TREND_MONTHS: u32 = 24;

// ── Public struct ─────────────────────────────────────────────────────────────

pub struct PerformanceMetrics;

impl PerformanceMetrics {
    // ── Calendar helpers ─────────────────────────────────────────────────────

    /// Convert a Unix timestamp to a compact `YYYYMM` month number.
    ///
    /// Uses the proleptic Gregorian civil-calendar algorithm by Howard Hinnant.
    pub fn timestamp_to_month_num(ts: u64) -> u32 {
        let z = (ts / 86_400) as i64 + 719_468;
        let era = (if z >= 0 { z } else { z - 146_096 }) / 146_097;
        let doe = (z - era * 146_097) as u64;
        let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
        let y = yoe as i64 + era * 400;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let m = if mp < 10 { mp + 3 } else { mp - 9 };
        let y_adj = if m <= 2 { y + 1 } else { y };
        (y_adj as u32) * 100 + m as u32
    }

    /// Build a human-readable `"YYYY-MM"` Soroban string from a month number.
    pub fn month_num_to_string(env: &Env, mn: u32) -> soroban_sdk::String {
        let year = mn / 100;
        let month = mn % 100;
        let bytes: [u8; 7] = [
            b'0' + ((year / 1_000) % 10) as u8,
            b'0' + ((year / 100) % 10) as u8,
            b'0' + ((year / 10) % 10) as u8,
            b'0' + (year % 10) as u8,
            b'-',
            b'0' + ((month / 10) % 10) as u8,
            b'0' + (month % 10) as u8,
        ];
        let s = core::str::from_utf8(&bytes).unwrap_or("0000-00");
        soroban_sdk::String::from_str(env, s)
    }

    fn current_month_num(env: &Env) -> u32 {
        Self::timestamp_to_month_num(env.ledger().timestamp())
    }

    // ── Storage helpers ──────────────────────────────────────────────────────

    fn load_global(env: &Env, month_num: u32) -> VerificationPerformanceSnapshot {
        env.storage()
            .persistent()
            .get(&PerformanceKey::GlobalPerformance(month_num))
            .unwrap_or_else(|| VerificationPerformanceSnapshot {
                month_key: Self::month_num_to_string(env, month_num),
                total_requests: 0,
                total_approved: 0,
                total_rejected: 0,
                total_appeals: 0,
                total_reversed: 0,
                approval_time_sum_secs: 0,
                approval_time_count: 0,
                hist_b0: 0,
                hist_b1: 0,
                hist_b2: 0,
                hist_b3: 0,
                hist_b4: 0,
                hist_b5: 0,
                hist_b6: 0,
                p50_approval_time_secs: 0,
                p90_approval_time_secs: 0,
                last_updated_at: 0,
            })
    }

    fn save_global(env: &Env, month_num: u32, snap: &VerificationPerformanceSnapshot) {
        env.storage()
            .persistent()
            .set(&PerformanceKey::GlobalPerformance(month_num), snap);
    }

    fn load_admin(env: &Env, admin: &Address, month_num: u32) -> AdminVerificationPerformance {
        env.storage()
            .persistent()
            .get(&PerformanceKey::AdminPerformance(admin.clone(), month_num))
            .unwrap_or_else(|| AdminVerificationPerformance {
                admin: admin.clone(),
                month_key: Self::month_num_to_string(env, month_num),
                approvals: 0,
                rejections: 0,
                appeals_against: 0,
                reversals: 0,
                approval_time_sum_secs: 0,
                approval_time_count: 0,
                last_action_at: 0,
            })
    }

    fn save_admin(
        env: &Env,
        admin: &Address,
        month_num: u32,
        perf: &AdminVerificationPerformance,
    ) {
        env.storage()
            .persistent()
            .set(&PerformanceKey::AdminPerformance(admin.clone(), month_num), perf);

        // Append admin to the month's roster if not already present.
        let mut admins: Vec<Address> = env
            .storage()
            .persistent()
            .get(&PerformanceKey::MonthAdmins(month_num))
            .unwrap_or_else(|| Vec::new(env));
        let already_present = admins.iter().any(|a| a == *admin);
        if !already_present {
            admins.push_back(admin.clone());
            env.storage()
                .persistent()
                .set(&PerformanceKey::MonthAdmins(month_num), &admins);
        }
    }

    /// Append `month_num` to the tracked-months list if not already present.
    /// Evicts the oldest entry when the 24-month cap is exceeded.
    fn track_month(env: &Env, month_num: u32) {
        let mut months: Vec<u32> = env
            .storage()
            .persistent()
            .get(&PerformanceKey::TrackedMonths)
            .unwrap_or_else(|| Vec::new(env));

        if months.iter().any(|m| m == month_num) {
            return;
        }

        months.push_back(month_num);

        while months.len() > MAX_TREND_MONTHS {
            let mut trimmed: Vec<u32> = Vec::new(env);
            for i in 1..months.len() {
                if let Some(v) = months.get(i) {
                    trimmed.push_back(v);
                }
            }
            months = trimmed;
        }

        env.storage()
            .persistent()
            .set(&PerformanceKey::TrackedMonths, &months);
    }

    // ── Histogram & percentile helpers ───────────────────────────────────────

    fn bucket_index(elapsed_secs: u64) -> usize {
        for (i, &bound) in BUCKET_BOUNDS.iter().enumerate() {
            if elapsed_secs < bound {
                return i;
            }
        }
        6
    }

    fn update_histogram_and_percentiles(
        snap: &mut VerificationPerformanceSnapshot,
        elapsed_secs: u64,
    ) {
        let bucket = Self::bucket_index(elapsed_secs);
        match bucket {
            0 => snap.hist_b0 = snap.hist_b0.saturating_add(1),
            1 => snap.hist_b1 = snap.hist_b1.saturating_add(1),
            2 => snap.hist_b2 = snap.hist_b2.saturating_add(1),
            3 => snap.hist_b3 = snap.hist_b3.saturating_add(1),
            4 => snap.hist_b4 = snap.hist_b4.saturating_add(1),
            5 => snap.hist_b5 = snap.hist_b5.saturating_add(1),
            _ => snap.hist_b6 = snap.hist_b6.saturating_add(1),
        }
        let h = [
            snap.hist_b0, snap.hist_b1, snap.hist_b2, snap.hist_b3,
            snap.hist_b4, snap.hist_b5, snap.hist_b6,
        ];
        snap.p50_approval_time_secs = Self::compute_percentile(&h, 50);
        snap.p90_approval_time_secs = Self::compute_percentile(&h, 90);
    }

    /// Estimate the approval-time value at `percentile` (0–100) using the
    /// histogram bucket counts.
    ///
    /// Returns the midpoint of the first bucket whose cumulative count meets
    /// or exceeds the target. Rates expressed in basis points:
    /// `10_000 bps == 100%`.
    pub fn compute_percentile(histogram: &[u32; 7], percentile: u32) -> u64 {
        let total: u32 = histogram.iter().sum();
        if total == 0 {
            return 0;
        }
        let target = ((total as u64) * (percentile as u64) + 99) / 100;
        let mut cumulative: u64 = 0;
        for (i, &count) in histogram.iter().enumerate() {
            cumulative += count as u64;
            if cumulative >= target {
                return BUCKET_MIDPOINTS[i];
            }
        }
        0
    }

    // ── Recording functions ───────────────────────────────────────────────────

    /// Record a new verification request.
    pub fn record_request(env: &Env, requested_at: u64) {
        let month_num = Self::current_month_num(env);
        Self::track_month(env, month_num);

        let mut snap = Self::load_global(env, month_num);
        snap.total_requests = snap.total_requests.saturating_add(1);
        snap.last_updated_at = requested_at;
        Self::save_global(env, month_num, &snap);
    }

    /// Record a successful approval.
    ///
    /// `requested_at` / `decided_at` are taken from `VerificationRecord` so
    /// elapsed time is independent of when this function is invoked.
    pub fn record_approval(
        env: &Env,
        admin: &Address,
        requested_at: u64,
        decided_at: u64,
    ) {
        let month_num = Self::current_month_num(env);
        Self::track_month(env, month_num);
        let elapsed = decided_at.saturating_sub(requested_at);

        let mut snap = Self::load_global(env, month_num);
        snap.total_approved = snap.total_approved.saturating_add(1);
        snap.approval_time_sum_secs = snap.approval_time_sum_secs.saturating_add(elapsed);
        snap.approval_time_count = snap.approval_time_count.saturating_add(1);
        snap.last_updated_at = decided_at;
        Self::update_histogram_and_percentiles(&mut snap, elapsed);
        Self::save_global(env, month_num, &snap);

        let mut aperf = Self::load_admin(env, admin, month_num);
        aperf.approvals = aperf.approvals.saturating_add(1);
        aperf.approval_time_sum_secs = aperf.approval_time_sum_secs.saturating_add(elapsed);
        aperf.approval_time_count = aperf.approval_time_count.saturating_add(1);
        aperf.last_action_at = decided_at;
        Self::save_admin(env, admin, month_num, &aperf);
    }

    /// Record a rejection.
    pub fn record_rejection(env: &Env, admin: &Address, decided_at: u64) {
        let month_num = Self::current_month_num(env);
        Self::track_month(env, month_num);

        let mut snap = Self::load_global(env, month_num);
        snap.total_rejected = snap.total_rejected.saturating_add(1);
        snap.last_updated_at = decided_at;
        Self::save_global(env, month_num, &snap);

        let mut aperf = Self::load_admin(env, admin, month_num);
        aperf.rejections = aperf.rejections.saturating_add(1);
        aperf.last_action_at = decided_at;
        Self::save_admin(env, admin, month_num, &aperf);
    }

    /// Record an appeal submission.
    ///
    /// `against_admin` is sourced from `VerificationRejectionState::rejected_by`.
    pub fn record_appeal(env: &Env, against_admin: &Address, submitted_at: u64) {
        let month_num = Self::current_month_num(env);
        Self::track_month(env, month_num);

        let mut snap = Self::load_global(env, month_num);
        snap.total_appeals = snap.total_appeals.saturating_add(1);
        snap.last_updated_at = submitted_at;
        Self::save_global(env, month_num, &snap);

        let mut aperf = Self::load_admin(env, against_admin, month_num);
        aperf.appeals_against = aperf.appeals_against.saturating_add(1);
        aperf.last_action_at = submitted_at;
        Self::save_admin(env, against_admin, month_num, &aperf);
    }

    /// Record a successful appeal (rejection reversed to Verified).
    ///
    /// `original_admin` is the admin whose rejection was overturned.
    pub fn record_reversal(env: &Env, original_admin: &Address, reviewed_at: u64) {
        let month_num = Self::current_month_num(env);
        Self::track_month(env, month_num);

        let mut snap = Self::load_global(env, month_num);
        snap.total_reversed = snap.total_reversed.saturating_add(1);
        snap.last_updated_at = reviewed_at;
        Self::save_global(env, month_num, &snap);

        let mut aperf = Self::load_admin(env, original_admin, month_num);
        aperf.reversals = aperf.reversals.saturating_add(1);
        aperf.last_action_at = reviewed_at;
        Self::save_admin(env, original_admin, month_num, &aperf);
    }

    // ── Query functions ───────────────────────────────────────────────────────

    /// Return the global performance snapshot for the given `YYYYMM` month number.
    pub fn get_global_snapshot(env: &Env, month_num: u32) -> VerificationPerformanceSnapshot {
        Self::load_global(env, month_num)
    }

    /// Return per-admin performance for `admin` in the given month.
    pub fn get_admin_performance(
        env: &Env,
        admin: Address,
        month_num: u32,
    ) -> AdminVerificationPerformance {
        Self::load_admin(env, &admin, month_num)
    }

    /// Return all admin addresses that acted in the given month.
    pub fn get_month_admins(env: &Env, month_num: u32) -> Vec<Address> {
        env.storage()
            .persistent()
            .get(&PerformanceKey::MonthAdmins(month_num))
            .unwrap_or_else(|| Vec::new(env))
    }

    /// Return the ordered list of tracked month numbers (oldest first).
    pub fn get_tracked_months(env: &Env) -> Vec<u32> {
        env.storage()
            .persistent()
            .get(&PerformanceKey::TrackedMonths)
            .unwrap_or_else(|| Vec::new(env))
    }

    /// Build trend data for every tracked month.
    pub fn get_trend(env: &Env) -> Vec<VerificationTrendPoint> {
        let months = Self::get_tracked_months(env);
        let mut trend: Vec<VerificationTrendPoint> = Vec::new(env);
        for i in 0..months.len() {
            let mn = match months.get(i) {
                Some(v) => v,
                None => continue,
            };
            let snap = Self::load_global(env, mn);
            let appeal_rate_bps = if snap.total_requests > 0 {
                ((snap.total_appeals as u64) * 10_000 / (snap.total_requests as u64)) as u32
            } else {
                0
            };
            let reversal_rate_bps = if snap.total_appeals > 0 {
                ((snap.total_reversed as u64) * 10_000 / (snap.total_appeals as u64)) as u32
            } else {
                0
            };
            let avg_approval_time_secs = if snap.approval_time_count > 0 {
                snap.approval_time_sum_secs / (snap.approval_time_count as u64)
            } else {
                0
            };
            trend.push_back(VerificationTrendPoint {
                month_key: snap.month_key,
                total_requests: snap.total_requests,
                total_approved: snap.total_approved,
                total_rejected: snap.total_rejected,
                appeal_rate_bps,
                reversal_rate_bps,
                avg_approval_time_secs,
            });
        }
        trend
    }

    /// Build a complete monthly performance report for the given month number.
    ///
    /// The report combines:
    /// - Global aggregate snapshot for the month
    /// - Per-admin performance for every admin active that month
    /// - Full trend series (up to 24 months)
    pub fn get_monthly_report(env: &Env, month_num: u32) -> VerificationPerformanceReport {
        let global = Self::load_global(env, month_num);
        let admin_addresses = Self::get_month_admins(env, month_num);
        let mut admin_summaries: Vec<AdminVerificationPerformance> = Vec::new(env);
        for i in 0..admin_addresses.len() {
            if let Some(addr) = admin_addresses.get(i) {
                admin_summaries.push_back(Self::load_admin(env, &addr, month_num));
            }
        }
        VerificationPerformanceReport {
            month_key: global.month_key.clone(),
            global,
            admin_summaries,
            trend: Self::get_trend(env),
        }
    }
}
