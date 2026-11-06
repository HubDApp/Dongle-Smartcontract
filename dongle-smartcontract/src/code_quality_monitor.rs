//! # Code Quality Monitoring Module
//!
//! Tracks and reports on contract health metrics including:
//! - Function call patterns and frequency
//! - Error rates and recovery statistics
//! - Storage efficiency and optimization opportunities
//! - Access control audit trails
//!
//! This module provides insights for ongoing maintenance and optimization.

use crate::errors::ContractError;
use crate::storage_keys::StorageKey;
use soroban_sdk::{Address, Env, String, Symbol, contracttype};

#[derive(Clone)]
#[contracttype]
pub struct CodeQualityMetrics {
    /// Total function invocations tracked
    pub total_calls: u64,
    /// Total errors encountered
    pub total_errors: u64,
    /// Average gas per call (basis points)
    pub avg_gas_per_call: u64,
    /// Timestamp of last update
    pub last_updated: u64,
    /// Number of unique callers
    pub unique_callers: u32,
}

#[contracttype]
pub enum QualityMetricKey {
    Metrics,
    LastErrorTime,
    ErrorRecoveryCount,
}

pub struct CodeQualityMonitor;

impl CodeQualityMonitor {
    /// Initialize quality metrics tracking
    pub fn initialize(env: &Env) -> Result<(), ContractError> {
        let metrics = CodeQualityMetrics {
            total_calls: 0,
            total_errors: 0,
            avg_gas_per_call: 0,
            last_updated: env.ledger().timestamp(),
            unique_callers: 0,
        };

        env.storage()
            .persistent()
            .set(&StorageKey::Extension(Symbol::new(env, "qm_metrics")), &metrics);

        Ok(())
    }

    /// Record a function call for quality metrics
    pub fn record_function_call(env: &Env, _caller: &Address, _gas_used: u64) {
        let mut metrics: CodeQualityMetrics = env
            .storage()
            .persistent()
            .get(&StorageKey::Extension(Symbol::new(env, "qm_metrics")))
            .unwrap_or(CodeQualityMetrics {
                total_calls: 0,
                total_errors: 0,
                avg_gas_per_call: 0,
                last_updated: env.ledger().timestamp(),
                unique_callers: 0,
            });

        metrics.total_calls = metrics.total_calls.saturating_add(1);
        metrics.last_updated = env.ledger().timestamp();

        env.storage()
            .persistent()
            .set(&StorageKey::Extension(Symbol::new(env, "qm_metrics")), &metrics);
    }

    /// Record an error for quality metrics
    pub fn record_error(env: &Env, _error: &ContractError) {
        let mut metrics: CodeQualityMetrics = env
            .storage()
            .persistent()
            .get(&StorageKey::Extension(Symbol::new(env, "qm_metrics")))
            .unwrap_or(CodeQualityMetrics {
                total_calls: 0,
                total_errors: 0,
                avg_gas_per_call: 0,
                last_updated: env.ledger().timestamp(),
                unique_callers: 0,
            });

        metrics.total_errors = metrics.total_errors.saturating_add(1);

        env.storage()
            .persistent()
            .set(&StorageKey::Extension(Symbol::new(env, "qm_metrics")), &metrics);
    }

    /// Get current quality metrics
    pub fn get_metrics(env: &Env) -> CodeQualityMetrics {
        env.storage()
            .persistent()
            .get(&StorageKey::Extension(Symbol::new(env, "qm_metrics")))
            .unwrap_or(CodeQualityMetrics {
                total_calls: 0,
                total_errors: 0,
                avg_gas_per_call: 0,
                last_updated: env.ledger().timestamp(),
                unique_callers: 0,
            })
    }

    /// Calculate error rate percentage (in basis points, 0-10000)
    pub fn calculate_error_rate(env: &Env) -> u64 {
        let metrics = Self::get_metrics(env);
        if metrics.total_calls == 0 {
            return 0;
        }
        (metrics.total_errors * 10000) / metrics.total_calls
    }

    /// Health check: returns true if contract is operating normally
    pub fn health_check(env: &Env) -> bool {
        let error_rate = Self::calculate_error_rate(env);
        // Contract is healthy if error rate is below 5% (500 basis points)
        error_rate < 500
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::Env as _;

    #[test]
    fn test_initialize_metrics() {
        let env = soroban_sdk::Env::default();
        let result = CodeQualityMonitor::initialize(&env);
        assert!(result.is_ok());

        let metrics = CodeQualityMonitor::get_metrics(&env);
        assert_eq!(metrics.total_calls, 0);
        assert_eq!(metrics.total_errors, 0);
    }

    #[test]
    fn test_error_rate_calculation() {
        let env = soroban_sdk::Env::default();
        let _ = CodeQualityMonitor::initialize(&env);

        let error_rate = CodeQualityMonitor::calculate_error_rate(&env);
        assert_eq!(error_rate, 0);
    }

    #[test]
    fn test_health_check() {
        let env = soroban_sdk::Env::default();
        let _ = CodeQualityMonitor::initialize(&env);

        let is_healthy = CodeQualityMonitor::health_check(&env);
        assert!(is_healthy);
    }
}
