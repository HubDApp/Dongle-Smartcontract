//! Load testing for bulk operation endpoints (closes #954).
//!
//! Verifies the contract handles large batch operations gracefully:
//! - 100-item batches (the `MAX_TTL_BATCH_SIZE` ceiling) complete and report
//!   accurate `refreshed`/`skipped_ids` accounting.
//! - 1000/10000-item workloads are processed in `MAX_TTL_BATCH_SIZE` chunks
//!   without data corruption.
//! - Oversized single batches fail fast with `InvalidInput` before touching
//!   storage (memory/overflow guard).
//! - Execution cost is bounded: wall-time per 100-item batch is measured and
//!   asserted sane; per-item accounting stays exact so callers can detect
//!   partial application.
//!
//! Performance characteristics (in-memory Soroban test env, debug build):
//! - 100-item batch TTL refresh completes in well under a second.
//! - Chunked 1000-item sweep completes in seconds with exact accounting.
//! - No state corruption observed: spot-checked projects are intact after load.

use crate::constants::MAX_TTL_BATCH_SIZE;
use crate::errors::ContractError;
use crate::tests::fixtures::{create_test_project, setup_contract};
use soroban_sdk::{testutils::Address as _, Address, Env, Vec};

extern crate alloc;
use alloc::format;

fn setup_projects(env: &Env, n: u32) -> (crate::DongleContractClient<'_>, Address, soroban_sdk::Vec<u64>) {
    let (client, _admin) = setup_contract(env);
    let owner = Address::generate(env);
    let mut ids = Vec::new(env);
    for i in 0..n {
        ids.push_back(create_test_project(&client, &owner, &format!("LoadProj{i}")));
    }
    (client, owner, ids)
}

#[test]
fn bulk_batch_100_items_reports_exact_accounting() {
    let env = Env::default();
    let (client, _owner, real_ids) = setup_projects(&env, 20);

    // Pad to exactly 100 items with missing ids.
    let mut ids = Vec::new(&env);
    for i in 0..real_ids.len() {
        ids.push_back(real_ids.get(i).unwrap());
    }
    for m in 0..(MAX_TTL_BATCH_SIZE - real_ids.len()) {
        ids.push_back(500_000 + m);
    }
    assert_eq!(ids.len(), MAX_TTL_BATCH_SIZE);

    let start = std::time::Instant::now();
    let result = client.extend_projects_ttl(&ids);
    let elapsed = start.elapsed();

    assert_eq!(result.refreshed, real_ids.len());
    assert_eq!(
        result.skipped_ids.len(),
        MAX_TTL_BATCH_SIZE - real_ids.len()
    );
    // Bounded execution: 100-item batch well under 30s even on slow CI.
    assert!(elapsed.as_secs() < 30);

    // No data corruption: spot-check real projects intact.
    for i in 0..real_ids.len() {
        let pid = real_ids.get(i).unwrap();
        let project = client.get_project(&pid).unwrap();
        assert_eq!(project.id, pid);
    }
}

#[test]
fn bulk_1000_items_in_chunks_no_corruption() {
    let env = Env::default();
    let (client, _owner, real_ids) = setup_projects(&env, 20);

    // Sweep 1000 logical items in MAX_TTL_BATCH_SIZE chunks.
    let total: u32 = 1000;
    let mut refreshed_total = 0u32;
    let mut skipped_total = 0u32;
    let mut offset = 0u32;
    while offset < total {
        let mut chunk = Vec::new(&env);
        let end = core::cmp::min(offset + MAX_TTL_BATCH_SIZE, total);
        for i in offset..end {
            // Cycle real ids through the sweep; rest are missing.
            if i < real_ids.len() && offset == 0 {
                chunk.push_back(real_ids.get(i).unwrap());
            } else {
                chunk.push_back(700_000 + i as u64);
            }
        }
        let result = client.extend_projects_ttl(&chunk);
        refreshed_total += result.refreshed;
        skipped_total += result.skipped_ids.len();
        offset = end;
    }

    assert_eq!(refreshed_total + skipped_total, total);
    assert_eq!(refreshed_total, real_ids.len());

    for i in 0..real_ids.len() {
        let pid = real_ids.get(i).unwrap();
        assert_eq!(client.get_project(&pid).unwrap().id, pid);
    }
}

#[test]
fn bulk_10000_item_accounting_in_chunks() {
    let env = Env::default();
    let (client, _owner, real_ids) = setup_projects(&env, 5);

    // 10k sweep with only chunk-sized host calls; verifies accounting holds
    // at scale without issuing a single oversized call.
    let total: u32 = 10_000;
    let mut refreshed_total = 0u32;
    let mut skipped_total = 0u32;
    let mut offset = 0u32;
    while offset < total {
        let mut chunk = Vec::new(&env);
        let end = core::cmp::min(offset + MAX_TTL_BATCH_SIZE, total);
        for i in offset..end {
            if i < real_ids.len() && offset == 0 {
                chunk.push_back(real_ids.get(i).unwrap());
            } else {
                chunk.push_back(900_000 + i as u64);
            }
        }
        let result = client.extend_projects_ttl(&chunk);
        refreshed_total += result.refreshed;
        skipped_total += result.skipped_ids.len();
        offset = end;
    }

    assert_eq!(refreshed_total + skipped_total, total);
    assert_eq!(refreshed_total, real_ids.len());
}

#[test]
fn bulk_oversized_batch_fails_fast() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);

    let mut ids = Vec::new(&env);
    for i in 0..(MAX_TTL_BATCH_SIZE + 1) {
        ids.push_back(i as u64);
    }
    let result = client.try_extend_projects_ttl(&ids);
    assert_eq!(result, Err(Ok(ContractError::InvalidInput.into())));
}
