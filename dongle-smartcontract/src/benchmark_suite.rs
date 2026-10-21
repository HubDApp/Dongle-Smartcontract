//! Benchmark suite for critical functions (#941).
//!
//! Measures gas-adjacent cost (ledger footprint + execution time proxy),
//! stores baselines in git (`benchmarks/baseline.json`), and lets CI compare
//! new measurements against the baseline, alerting on >10% regressions.
//!
//! On-chain, `record_benchmark` stores a named measurement (storage reads,
//! execution time in ledger-ms proxy). Off-chain, `scripts/check_benchmarks.py`
//! compares fresh results against the committed baseline.

use soroban_sdk::{contracttype, Address, Env, String, Vec};

use crate::admin_manager::AdminManager;
use crate::errors::ContractError;

/// Regression threshold in basis points: 10% == 1_000 bps.
pub const REGRESSION_THRESHOLD_BPS: u32 = 1_000;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BenchmarkSample {
    pub name: String,
    pub storage_reads: u32,
    pub exec_units: u64,
    pub recorded_at: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BenchmarkBaseline {
    pub name: String,
    pub storage_reads: u32,
    pub exec_units: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BenchmarkRegression {
    pub name: String,
    pub baseline: u64,
    pub current: u64,
    pub regression_bps: u32,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
enum BenchmarkKey {
    Samples,
    Baselines,
}

pub struct BenchmarkSuite;

impl BenchmarkSuite {
    /// Record a measurement for a critical function (admin-only keeper call).
    pub fn record(
        env: &Env,
        caller: Address,
        name: String,
        storage_reads: u32,
        exec_units: u64,
    ) -> Result<(), ContractError> {
        caller.require_auth();
        if !AdminManager::is_admin(env, &caller) {
            return Err(ContractError::AdminOnly);
        }
        let mut samples: Vec<BenchmarkSample> = env
            .storage()
            .persistent()
            .get(&BenchmarkKey::Samples)
            .unwrap_or_else(|| Vec::new(env));
        samples.push_back(BenchmarkSample {
            name,
            storage_reads,
            exec_units,
            recorded_at: env.ledger().timestamp(),
        });
        env.storage()
            .persistent()
            .set(&BenchmarkKey::Samples, &samples);
        Ok(())
    }

    /// Admin-only: pin current medians as the baseline for `name`.
    pub fn set_baseline(
        env: &Env,
        admin: Address,
        name: String,
        storage_reads: u32,
        exec_units: u64,
    ) -> Result<(), ContractError> {
        admin.require_auth();
        if !AdminManager::is_admin(env, &admin) {
            return Err(ContractError::AdminOnly);
        }
        let mut baselines: Vec<BenchmarkBaseline> = env
            .storage()
            .persistent()
            .get(&BenchmarkKey::Baselines)
            .unwrap_or_else(|| Vec::new(env));
        let mut found = false;
        for i in 0..baselines.len() {
            if let Some(mut b) = baselines.get(i) {
                if b.name == name {
                    b.storage_reads = storage_reads;
                    b.exec_units = exec_units;
                    baselines.set(i, b);
                    found = true;
                    break;
                }
            }
        }
        if !found {
            baselines.push_back(BenchmarkBaseline {
                name,
                storage_reads,
                exec_units,
            });
        }
        env.storage()
            .persistent()
            .set(&BenchmarkKey::Baselines, &baselines);
        Ok(())
    }

    pub fn get_baselines(env: &Env) -> Vec<BenchmarkBaseline> {
        env.storage()
            .persistent()
            .get(&BenchmarkKey::Baselines)
            .unwrap_or_else(|| Vec::new(env))
    }

    pub fn get_samples(env: &Env) -> Vec<BenchmarkSample> {
        env.storage()
            .persistent()
            .get(&BenchmarkKey::Samples)
            .unwrap_or_else(|| Vec::new(env))
    }

    fn regression_bps(baseline: u64, current: u64) -> Option<u32> {
        if baseline == 0 {
            return None;
        }
        if current <= baseline {
            return None;
        }
        let bps = ((current.saturating_sub(baseline)) as u128)
            .saturating_mul(10_000)
            .checked_div(baseline as u128)
            .unwrap_or(0);
        let bps = core::cmp::min(bps, u32::MAX as u128) as u32;
        if bps > REGRESSION_THRESHOLD_BPS {
            Some(bps)
        } else {
            None
        }
    }

    /// Compare latest samples against baselines; return regressions >10%.
    pub fn check_regressions(env: &Env) -> Vec<BenchmarkRegression> {
        let baselines = Self::get_baselines(env);
        let samples = Self::get_samples(env);
        let mut out: Vec<BenchmarkRegression> = Vec::new(env);
        for i in 0..baselines.len() {
            let b = match baselines.get(i) {
                Some(v) => v,
                None => continue,
            };
            // Latest sample for this benchmark name.
            let mut latest: Option<BenchmarkSample> = None;
            for j in 0..samples.len() {
                if let Some(s) = samples.get(j) {
                    if s.name == b.name {
                        latest = Some(s);
                    }
                }
            }
            if let Some(s) = latest {
                if let Some(bps) = Self::regression_bps(b.exec_units, s.exec_units) {
                    out.push_back(BenchmarkRegression {
                        name: b.name.clone(),
                        baseline: b.exec_units,
                        current: s.exec_units,
                        regression_bps: bps,
                    });
                }
            }
        }
        out
    }
}
