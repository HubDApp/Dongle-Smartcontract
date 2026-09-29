//! Tests for verification performance metrics (#perf).
//!
//! Covers all acceptance criteria:
//! - Approval time tracking (average, P50, P90)
//! - Appeal rate tracking
//! - Reversal rate tracking
//! - Percentile computation correctness
//! - Multi-admin comparison
//! - Multi-month trend data
//! - Monthly performance report assembly

use crate::performance_metrics::PerformanceMetrics;
use soroban_sdk::{testutils::Ledger as _, Env};

// ── Helpers ──────────────────────────────────────────────────────────────────

fn setup_env() -> Env {
    Env::default()
}

/// Advance the ledger timestamp by `delta` seconds.
fn advance(env: &Env, delta: u64) {
    env.ledger().with_mut(|li| li.timestamp += delta);
}

/// Set the ledger timestamp to an absolute value.
fn set_ts(env: &Env, ts: u64) {
    env.ledger().with_mut(|li| li.timestamp = ts);
}

/// Make a dummy Address for use in tests.
fn admin(env: &Env, seed: u32) -> soroban_sdk::Address {
    soroban_sdk::Address::generate(env)
}

// ── timestamp_to_month_num ────────────────────────────────────────────────────

#[test]
fn test_timestamp_to_month_num_known_dates() {
    // 2026-09-26 00:00:00 UTC  →  202609
    assert_eq!(PerformanceMetrics::timestamp_to_month_num(1790352000), 202609);
    // 2024-01-01 00:00:00 UTC  →  202401
    assert_eq!(PerformanceMetrics::timestamp_to_month_num(1704067200), 202401);
    // 2024-12-31 23:59:59 UTC  →  202412
    assert_eq!(PerformanceMetrics::timestamp_to_month_num(1735689599), 202412);
    // 2000-03-01 00:00:00 UTC  →  200003
    assert_eq!(PerformanceMetrics::timestamp_to_month_num(951868800), 200003);
}

#[test]
fn test_timestamp_to_month_num_zero_epoch() {
    // 1970-01-01 00:00:00 UTC  →  197001
    assert_eq!(PerformanceMetrics::timestamp_to_month_num(0), 197001);
}

// ── month_num_to_string ───────────────────────────────────────────────────────

#[test]
fn test_month_num_to_string() {
    let env = setup_env();
    let s = PerformanceMetrics::month_num_to_string(&env, 202609);
    assert_eq!(s, soroban_sdk::String::from_str(&env, "2026-09"));
}

#[test]
fn test_month_num_to_string_jan() {
    let env = setup_env();
    let s = PerformanceMetrics::month_num_to_string(&env, 202401);
    assert_eq!(s, soroban_sdk::String::from_str(&env, "2024-01"));
}

// ── compute_percentile ────────────────────────────────────────────────────────

#[test]
fn test_percentile_empty_histogram() {
    assert_eq!(PerformanceMetrics::compute_percentile(&[0; 7], 50), 0);
    assert_eq!(PerformanceMetrics::compute_percentile(&[0; 7], 90), 0);
}

#[test]
fn test_percentile_all_in_first_bucket() {
    // 10 approvals all under 1 h → P50 = P90 = 1_800 s (~30 min)
    let h = [10u32, 0, 0, 0, 0, 0, 0];
    assert_eq!(PerformanceMetrics::compute_percentile(&h, 50), 1_800);
    assert_eq!(PerformanceMetrics::compute_percentile(&h, 90), 1_800);
}

#[test]
fn test_percentile_split_across_buckets() {
    // 5 in b0 (< 1 h), 5 in b1 (1–6 h): 10 total
    // P50 target = ceil(10 * 50 / 100) = 5 → cumulative at b0 (5) >= 5 → midpoint b0 = 1_800
    // P90 target = ceil(10 * 90 / 100) = 9 → cumulative at b1 (10) >= 9 → midpoint b1 = 12_600
    let h = [5u32, 5, 0, 0, 0, 0, 0];
    assert_eq!(PerformanceMetrics::compute_percentile(&h, 50), 1_800);
    assert_eq!(PerformanceMetrics::compute_percentile(&h, 90), 12_600);
}

#[test]
fn test_percentile_spread_all_buckets() {
    // 1 sample in each bucket (7 total)
    // P50 target = ceil(7 * 50 / 100) = 4 → b3 midpoint = 172_800
    let h = [1u32; 7];
    assert_eq!(PerformanceMetrics::compute_percentile(&h, 50), 172_800);
    // P90 target = ceil(7 * 90 / 100) = 7 → b6 midpoint = 5_184_000
    assert_eq!(PerformanceMetrics::compute_percentile(&h, 90), 5_184_000);
}

// ── record_request ────────────────────────────────────────────────────────────

#[test]
fn test_record_request_increments_total() {
    let env = setup_env();
    // 2026-09-26 UTC
    set_ts(&env, 1790352000);

    PerformanceMetrics::record_request(&env, env.ledger().timestamp());
    PerformanceMetrics::record_request(&env, env.ledger().timestamp());

    let mn = PerformanceMetrics::timestamp_to_month_num(env.ledger().timestamp());
    let snap = PerformanceMetrics::get_global_snapshot(&env, mn);
    assert_eq!(snap.total_requests, 2);
    assert_eq!(snap.total_approved, 0);
    assert_eq!(snap.total_rejected, 0);
}

// ── record_approval ───────────────────────────────────────────────────────────

#[test]
fn test_record_approval_increments_and_tracks_time() {
    let env = setup_env();
    set_ts(&env, 1790352000); // 2026-09-26

    let a = admin(&env, 1);
    // Approval took 7_200 s (2 h) → bucket b1
    PerformanceMetrics::record_approval(&env, &a, 1790352000, 1790352000 + 7_200);

    let mn = PerformanceMetrics::timestamp_to_month_num(1790352000);
    let snap = PerformanceMetrics::get_global_snapshot(&env, mn);
    assert_eq!(snap.total_approved, 1);
    assert_eq!(snap.approval_time_count, 1);
    assert_eq!(snap.approval_time_sum_secs, 7_200);
    // b0 = 0, b1 = 1
    assert_eq!(snap.hist_b0, 0);
    assert_eq!(snap.hist_b1, 1);
    // P50 and P90 both land in b1
    assert_eq!(snap.p50_approval_time_secs, 12_600);
    assert_eq!(snap.p90_approval_time_secs, 12_600);

    // Per-admin record
    let aperf = PerformanceMetrics::get_admin_performance(&env, a, mn);
    assert_eq!(aperf.approvals, 1);
    assert_eq!(aperf.approval_time_sum_secs, 7_200);
    assert_eq!(snap.last_updated_at, 1790352000 + 7_200);
}

#[test]
fn test_record_multiple_approvals_average() {
    let env = setup_env();
    set_ts(&env, 1790352000);

    let a = admin(&env, 1);
    // 3_600 s (b0) + 7_200 s (b1) + 86_400 s (b2) = 97_200 s total, 3 samples
    PerformanceMetrics::record_approval(&env, &a, 1790352000, 1790352000 + 3_600);
    PerformanceMetrics::record_approval(&env, &a, 1790352000, 1790352000 + 7_200);
    PerformanceMetrics::record_approval(&env, &a, 1790352000, 1790352000 + 86_400);

    let mn = PerformanceMetrics::timestamp_to_month_num(1790352000);
    let snap = PerformanceMetrics::get_global_snapshot(&env, mn);
    assert_eq!(snap.total_approved, 3);
    assert_eq!(snap.approval_time_sum_secs, 97_200);
    assert_eq!(snap.hist_b0, 1);
    assert_eq!(snap.hist_b1, 1);
    assert_eq!(snap.hist_b2, 1);
}

// ── record_rejection ──────────────────────────────────────────────────────────

#[test]
fn test_record_rejection_increments() {
    let env = setup_env();
    set_ts(&env, 1790352000);

    let a = admin(&env, 1);
    PerformanceMetrics::record_rejection(&env, &a, env.ledger().timestamp());
    PerformanceMetrics::record_rejection(&env, &a, env.ledger().timestamp());

    let mn = PerformanceMetrics::timestamp_to_month_num(env.ledger().timestamp());
    let snap = PerformanceMetrics::get_global_snapshot(&env, mn);
    assert_eq!(snap.total_rejected, 2);

    let aperf = PerformanceMetrics::get_admin_performance(&env, a, mn);
    assert_eq!(aperf.rejections, 2);
}

// ── record_appeal ─────────────────────────────────────────────────────────────

#[test]
fn test_record_appeal_increments_against_admin() {
    let env = setup_env();
    set_ts(&env, 1790352000);

    let a = admin(&env, 1);
    PerformanceMetrics::record_rejection(&env, &a, env.ledger().timestamp());
    PerformanceMetrics::record_appeal(&env, &a, env.ledger().timestamp());

    let mn = PerformanceMetrics::timestamp_to_month_num(env.ledger().timestamp());
    let snap = PerformanceMetrics::get_global_snapshot(&env, mn);
    assert_eq!(snap.total_appeals, 1);

    let aperf = PerformanceMetrics::get_admin_performance(&env, a, mn);
    assert_eq!(aperf.appeals_against, 1);
}

// ── record_reversal ───────────────────────────────────────────────────────────

#[test]
fn test_record_reversal_increments() {
    let env = setup_env();
    set_ts(&env, 1790352000);

    let a = admin(&env, 1);
    PerformanceMetrics::record_rejection(&env, &a, env.ledger().timestamp());
    PerformanceMetrics::record_appeal(&env, &a, env.ledger().timestamp());
    PerformanceMetrics::record_reversal(&env, &a, env.ledger().timestamp());

    let mn = PerformanceMetrics::timestamp_to_month_num(env.ledger().timestamp());
    let snap = PerformanceMetrics::get_global_snapshot(&env, mn);
    assert_eq!(snap.total_reversed, 1);

    let aperf = PerformanceMetrics::get_admin_performance(&env, a.clone(), mn);
    assert_eq!(aperf.reversals, 1);
    assert_eq!(aperf.rejections, 1);
    assert_eq!(aperf.appeals_against, 1);
}

// ── Multi-admin comparison ────────────────────────────────────────────────────

#[test]
fn test_multi_admin_comparison() {
    let env = setup_env();
    set_ts(&env, 1790352000);

    let a1 = admin(&env, 1);
    let a2 = admin(&env, 2);

    // Admin 1: 2 approvals, 1 rejection; admin 2: 1 approval, 2 rejections
    PerformanceMetrics::record_approval(&env, &a1, 1790352000, 1790352000 + 3_600);
    PerformanceMetrics::record_approval(&env, &a1, 1790352000, 1790352000 + 7_200);
    PerformanceMetrics::record_rejection(&env, &a1, env.ledger().timestamp());

    PerformanceMetrics::record_approval(&env, &a2, 1790352000, 1790352000 + 86_400);
    PerformanceMetrics::record_rejection(&env, &a2, env.ledger().timestamp());
    PerformanceMetrics::record_rejection(&env, &a2, env.ledger().timestamp());

    let mn = PerformanceMetrics::timestamp_to_month_num(1790352000);
    let p1 = PerformanceMetrics::get_admin_performance(&env, a1.clone(), mn);
    let p2 = PerformanceMetrics::get_admin_performance(&env, a2.clone(), mn);

    assert_eq!(p1.approvals, 2);
    assert_eq!(p1.rejections, 1);
    assert_eq!(p2.approvals, 1);
    assert_eq!(p2.rejections, 2);

    // Month admins list should contain both
    let month_admins = PerformanceMetrics::get_month_admins(&env, mn);
    assert!(month_admins.len() >= 2);
}

// ── Trend data ────────────────────────────────────────────────────────────────

#[test]
fn test_trend_single_month() {
    let env = setup_env();
    set_ts(&env, 1790352000); // 2026-09

    let a = admin(&env, 1);
    PerformanceMetrics::record_request(&env, env.ledger().timestamp());
    PerformanceMetrics::record_approval(&env, &a, 1790352000, 1790352000 + 3_600);

    let trend = PerformanceMetrics::get_trend(&env);
    assert_eq!(trend.len(), 1);
    let pt = trend.get(0).unwrap();
    assert_eq!(pt.total_requests, 1);
    assert_eq!(pt.total_approved, 1);
    assert_eq!(pt.total_rejected, 0);
    assert_eq!(pt.appeal_rate_bps, 0);
    assert_eq!(pt.reversal_rate_bps, 0);
    assert_eq!(pt.avg_approval_time_secs, 3_600);
}

#[test]
fn test_trend_two_months() {
    let env = setup_env();
    let a = admin(&env, 1);

    // Month 1: 2026-08 (August)
    set_ts(&env, 1787673600); // 2026-08-01 approx
    PerformanceMetrics::record_request(&env, env.ledger().timestamp());
    PerformanceMetrics::record_approval(&env, &a, env.ledger().timestamp(), env.ledger().timestamp() + 3_600);

    // Month 2: 2026-09 (September)
    set_ts(&env, 1790352000);
    PerformanceMetrics::record_request(&env, env.ledger().timestamp());
    PerformanceMetrics::record_approval(&env, &a, env.ledger().timestamp(), env.ledger().timestamp() + 7_200);
    PerformanceMetrics::record_rejection(&env, &a, env.ledger().timestamp());
    PerformanceMetrics::record_appeal(&env, &a, env.ledger().timestamp());

    let trend = PerformanceMetrics::get_trend(&env);
    assert_eq!(trend.len(), 2);

    let aug = trend.get(0).unwrap();
    let sep = trend.get(1).unwrap();

    // August
    assert_eq!(aug.total_requests, 1);
    assert_eq!(aug.total_approved, 1);
    assert_eq!(aug.appeal_rate_bps, 0);

    // September: 1 request, 1 appeal → appeal_rate = 1/1 * 10000 = 10000
    assert_eq!(sep.total_approved, 1);
    assert_eq!(sep.appeal_rate_bps, 10_000);
    assert_eq!(sep.reversal_rate_bps, 0); // no reversal
}

#[test]
fn test_trend_appeal_and_reversal_rate() {
    let env = setup_env();
    set_ts(&env, 1790352000);
    let a = admin(&env, 1);

    PerformanceMetrics::record_request(&env, env.ledger().timestamp());
    PerformanceMetrics::record_request(&env, env.ledger().timestamp());
    PerformanceMetrics::record_rejection(&env, &a, env.ledger().timestamp());
    // 2 appeals, 1 reversal
    PerformanceMetrics::record_appeal(&env, &a, env.ledger().timestamp());
    PerformanceMetrics::record_appeal(&env, &a, env.ledger().timestamp());
    PerformanceMetrics::record_reversal(&env, &a, env.ledger().timestamp());

    let trend = PerformanceMetrics::get_trend(&env);
    assert_eq!(trend.len(), 1);
    let pt = trend.get(0).unwrap();

    // appeal_rate = 2 / 2 * 10_000 = 10_000 bps
    assert_eq!(pt.appeal_rate_bps, 10_000);
    // reversal_rate = 1 / 2 * 10_000 = 5_000 bps
    assert_eq!(pt.reversal_rate_bps, 5_000);
}

// ── Monthly report assembly ───────────────────────────────────────────────────

#[test]
fn test_monthly_report_empty_month() {
    let env = setup_env();
    let report = PerformanceMetrics::get_monthly_report(&env, 202609);
    assert_eq!(report.total_requests, report.global.total_requests);
    assert_eq!(report.global.total_requests, 0);
    assert_eq!(report.admin_summaries.len(), 0);
    assert_eq!(report.trend.len(), 0);
}

#[test]
fn test_monthly_report_full() {
    let env = setup_env();
    set_ts(&env, 1790352000); // 2026-09-26

    let a1 = admin(&env, 1);
    let a2 = admin(&env, 2);

    // 2 requests
    PerformanceMetrics::record_request(&env, env.ledger().timestamp());
    PerformanceMetrics::record_request(&env, env.ledger().timestamp());
    // a1: 1 approval
    PerformanceMetrics::record_approval(&env, &a1, 1790352000, 1790352000 + 3_600);
    // a2: 1 rejection, 1 appeal, 1 reversal
    PerformanceMetrics::record_rejection(&env, &a2, env.ledger().timestamp());
    PerformanceMetrics::record_appeal(&env, &a2, env.ledger().timestamp());
    PerformanceMetrics::record_reversal(&env, &a2, env.ledger().timestamp());

    let mn = PerformanceMetrics::timestamp_to_month_num(1790352000);
    let report = PerformanceMetrics::get_monthly_report(&env, mn);

    assert_eq!(report.global.total_requests, 2);
    assert_eq!(report.global.total_approved, 1);
    assert_eq!(report.global.total_rejected, 1);
    assert_eq!(report.global.total_appeals, 1);
    assert_eq!(report.global.total_reversed, 1);
    // 2 distinct admins acted
    assert_eq!(report.admin_summaries.len(), 2);
    // trend contains this month
    assert_eq!(report.trend.len(), 1);
    assert_eq!(
        report.month_key,
        soroban_sdk::String::from_str(&env, "2026-09")
    );
}

// ── tracked_months cap ────────────────────────────────────────────────────────

#[test]
fn test_tracked_months_deduplication() {
    let env = setup_env();
    set_ts(&env, 1790352000);

    // Record 3 requests in the same month – should still be 1 tracked month.
    let ts = env.ledger().timestamp();
    PerformanceMetrics::record_request(&env, ts);
    PerformanceMetrics::record_request(&env, ts);
    PerformanceMetrics::record_request(&env, ts);

    let months = PerformanceMetrics::get_tracked_months(&env);
    assert_eq!(months.len(), 1);
}

#[test]
fn test_default_snapshot_has_zero_values() {
    let env = setup_env();
    let snap = PerformanceMetrics::get_global_snapshot(&env, 202609);
    assert_eq!(snap.total_requests, 0);
    assert_eq!(snap.total_approved, 0);
    assert_eq!(snap.p50_approval_time_secs, 0);
    assert_eq!(snap.p90_approval_time_secs, 0);
}
