//! Analytics export for BI and compliance reporting (closes #934).
//!
//! `export_analytics_data` aggregates registry activity inside a timestamp
//! window and returns both the structured metrics and a rendered JSON or CSV
//! payload for direct ingestion by BI tools.
//!
//! ## Scanning
//!
//! Metrics are computed by reading projects directly, so one call scans at
//! most `MAX_ANALYTICS_SCAN_SIZE` project IDs. `AnalyticsExport::next_project_id`
//! is set when more projects remain; pass it back as
//! `AnalyticsFilter::start_project_id` and sum the pages off-chain. Each page
//! is consistent with on-chain state at the ledger it was read in.
//!
//! ## Metric definitions (all inside `[start, end]`, inclusive)
//!
//! - `projects_created`: projects whose `created_at` falls in the window.
//! - `reviews_created`: reviews (hidden ones included) whose `created_at`
//!   falls in the window.
//! - `verification_requests`: verification records whose `requested_at`
//!   falls in the window.
//! - `verifications_approved` / `verifications_rejected`: records decided in
//!   the window with final status `Verified` / `Rejected`.
//! - `verification_rate_bps`: approved ÷ (approved + rejected), in basis
//!   points; zero when nothing was decided.
//! - `verification_fee_revenue`: `fee_amount` of requests in the window.
//! - `registration_fee_revenue`: the latest registration fee payment of each
//!   distinct project owner, when paid in the window. Only the latest payment
//!   per address is stored on-chain, so earlier payments by the same owner
//!   are not counted.

use crate::constants::MAX_ANALYTICS_SCAN_SIZE;
use crate::errors::ContractError;
use crate::fee_manager::FeeManager;
use crate::project_registry::ProjectRegistry;
use crate::state_export::StateExport;
use crate::storage_keys::StorageKey;
use crate::types::{
    AnalyticsExport, AnalyticsFilter, AnalyticsMetrics, ExportFormat, Review, VerificationStatus,
};
use crate::verification_registry::VerificationRegistry;
use alloc::format;
use soroban_sdk::{Address, Env, String, Vec};

pub struct AnalyticsExportRegistry;

impl AnalyticsExportRegistry {
    pub fn export_analytics_data(
        env: &Env,
        start: u64,
        end: u64,
        format: ExportFormat,
        filter: AnalyticsFilter,
    ) -> Result<AnalyticsExport, ContractError> {
        if start > end {
            return Err(ContractError::InvalidInput);
        }

        let (metrics, next_project_id) = Self::compute_metrics(env, start, end, &filter);
        let payload = match format {
            ExportFormat::Json => Self::render_json(env, start, end, &metrics),
            ExportFormat::Csv => Self::render_csv(env, start, end, &metrics),
        };

        Ok(AnalyticsExport {
            format,
            start,
            end,
            filter,
            metrics,
            next_project_id,
            generated_at: env.ledger().timestamp(),
            payload,
        })
    }

    /// Filter used by the unfiltered entry point: every project, first page.
    pub fn default_filter() -> AnalyticsFilter {
        AnalyticsFilter {
            category: None,
            verified_only: false,
            start_project_id: 1,
            limit: 0,
        }
    }

    fn compute_metrics(
        env: &Env,
        start: u64,
        end: u64,
        filter: &AnalyticsFilter,
    ) -> (AnalyticsMetrics, Option<u64>) {
        let in_window = |ts: u64| ts >= start && ts <= end;

        let first = if filter.start_project_id == 0 {
            1
        } else {
            filter.start_project_id
        };
        let limit = if filter.limit == 0 || filter.limit > MAX_ANALYTICS_SCAN_SIZE {
            MAX_ANALYTICS_SCAN_SIZE
        } else {
            filter.limit
        };
        let project_count = StateExport::project_count(env);
        let last = core::cmp::min(first.saturating_add(limit as u64 - 1), project_count);

        let mut m = AnalyticsMetrics {
            projects_scanned: 0,
            projects_matched: 0,
            projects_created: 0,
            reviews_created: 0,
            verification_requests: 0,
            verifications_approved: 0,
            verifications_rejected: 0,
            verification_rate_bps: 0,
            verification_fee_revenue: 0,
            registration_fee_revenue: 0,
            total_fee_revenue: 0,
        };
        let mut counted_owners: Vec<Address> = Vec::new(env);

        let mut id = first;
        while id <= last {
            let Some(project) = ProjectRegistry::get_project(env, id) else {
                id += 1;
                continue;
            };
            m.projects_scanned += 1;
            id += 1;

            if let Some(category) = &filter.category {
                if &project.category != category {
                    continue;
                }
            }
            if filter.verified_only && project.verification_status != VerificationStatus::Verified
            {
                continue;
            }
            m.projects_matched += 1;

            if in_window(project.created_at) {
                m.projects_created += 1;
            }

            let reviewers: Vec<Address> = env
                .storage()
                .persistent()
                .get(&StorageKey::ProjectReviews(project.id))
                .unwrap_or_else(|| Vec::new(env));
            for reviewer in reviewers.iter() {
                if let Some(review) = env
                    .storage()
                    .persistent()
                    .get::<_, Review>(&StorageKey::Review(project.id, reviewer))
                {
                    if in_window(review.created_at) {
                        m.reviews_created += 1;
                    }
                }
            }

            for record in VerificationRegistry::get_verification_history(env, project.id).iter() {
                if in_window(record.requested_at) {
                    m.verification_requests += 1;
                    m.verification_fee_revenue =
                        m.verification_fee_revenue.saturating_add(record.fee_amount);
                }
                if record.decided_at > 0 && in_window(record.decided_at) {
                    match record.status {
                        VerificationStatus::Verified => m.verifications_approved += 1,
                        VerificationStatus::Rejected => m.verifications_rejected += 1,
                        _ => {}
                    }
                }
            }

            if !counted_owners.contains(&project.owner) {
                counted_owners.push_back(project.owner.clone());
                if let Some(payment) =
                    FeeManager::get_registration_fee_payment_details(env, &project.owner)
                {
                    if in_window(payment.paid_at) {
                        m.registration_fee_revenue =
                            m.registration_fee_revenue.saturating_add(payment.amount);
                    }
                }
            }
        }

        let decided = m.verifications_approved + m.verifications_rejected;
        if decided > 0 {
            m.verification_rate_bps =
                ((m.verifications_approved as u64 * 10_000) / decided as u64) as u32;
        }
        m.total_fee_revenue = m
            .verification_fee_revenue
            .saturating_add(m.registration_fee_revenue);

        let next = if last < project_count {
            Some(last + 1)
        } else {
            None
        };
        (m, next)
    }

    fn render_json(env: &Env, start: u64, end: u64, m: &AnalyticsMetrics) -> String {
        let body = format!(
            "{{\"start\":{},\"end\":{},\"projects_scanned\":{},\"projects_matched\":{},\
             \"projects_created\":{},\"reviews_created\":{},\"verification_requests\":{},\
             \"verifications_approved\":{},\"verifications_rejected\":{},\
             \"verification_rate_bps\":{},\"verification_fee_revenue\":\"{}\",\
             \"registration_fee_revenue\":\"{}\",\"total_fee_revenue\":\"{}\"}}",
            start,
            end,
            m.projects_scanned,
            m.projects_matched,
            m.projects_created,
            m.reviews_created,
            m.verification_requests,
            m.verifications_approved,
            m.verifications_rejected,
            m.verification_rate_bps,
            m.verification_fee_revenue,
            m.registration_fee_revenue,
            m.total_fee_revenue,
        );
        String::from_str(env, &body)
    }

    fn render_csv(env: &Env, start: u64, end: u64, m: &AnalyticsMetrics) -> String {
        let body = format!(
            "start,end,projects_scanned,projects_matched,projects_created,reviews_created,\
             verification_requests,verifications_approved,verifications_rejected,\
             verification_rate_bps,verification_fee_revenue,registration_fee_revenue,\
             total_fee_revenue\n{},{},{},{},{},{},{},{},{},{},{},{},{}\n",
            start,
            end,
            m.projects_scanned,
            m.projects_matched,
            m.projects_created,
            m.reviews_created,
            m.verification_requests,
            m.verifications_approved,
            m.verifications_rejected,
            m.verification_rate_bps,
            m.verification_fee_revenue,
            m.registration_fee_revenue,
            m.total_fee_revenue,
        );
        String::from_str(env, &body)
    }
}
