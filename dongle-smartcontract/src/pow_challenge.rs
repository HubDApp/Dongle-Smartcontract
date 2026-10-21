//! Proof-of-work challenge for expensive operations (#946).
//!
//! Makes bulk / resource-heavy calls economically costly for spammers while
//! staying cheap for honest users.
//!
//! ## Algorithm
//!
//! - `issue_pow_challenge(op_cost, difficulty)` returns a pseudo-random
//!   32-byte challenge derived from ledger timestamp + sequence + `op_cost`.
//! - `difficulty` = number of leading zero **bits** required in
//!   `sha256(challenge || nonce)`. Difficulty scales with operation cost:
//!   `required_difficulty(op_cost) = BASE_DIFFICULTY + op_cost / COST_PER_BIT`
//!   capped at `MAX_DIFFICULTY` (see [`PowChallenge::difficulty_for_cost`]).
//! - Callers find a `nonce` (u64, big-endian bytes) whose digest meets the
//!   difficulty, then pass `(challenge, nonce)` to `verify_pow` before the
//!   expensive function executes.
//!
//! ## Difficulty levels
//!
//! | Level | Bits | Intended use |
//! |-------|------|--------------|
//! | 0–7   | trivial | tests, single cheap writes |
//! | 8–15  | easy | single bulk item |
//! | 16–19 | moderate | bulk batches (default `BASE_DIFFICULTY = 12`) |
//! | 20–24 | hard | very large batches; capped at 24 to keep verification cheap |
//!
//! ## Client library
//!
//! Off-chain callers use [`find_pow_nonce`] (also re-exported for integration
//! tests / scripts): brute-forces nonces from 0 until the digest meets
//! `difficulty`, then submits the `(challenge, nonce)` pair on-chain.

use soroban_sdk::{contracttype, Bytes, BytesN, Env};

use crate::errors::ContractError;

/// Base difficulty (leading-zero bits) for a zero-cost operation.
pub const BASE_DIFFICULTY: u32 = 12;
/// Extra difficulty bit per this much operation cost (e.g. items in batch).
pub const COST_PER_BIT: u32 = 25;
/// Hard cap so verification stays cheap and clients can't be priced out.
pub const MAX_DIFFICULTY: u32 = 24;
/// Challenges expire after this many ledgers (replay protection).
pub const CHALLENGE_TTL_LEDGERS: u32 = 100;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PowChallenge {
    pub challenge: BytesN<32>,
    pub difficulty: u32,
    pub issued_at_ledger: u32,
}

/// Scale difficulty with operation cost (e.g. batch size).
pub fn difficulty_for_cost(op_cost: u32) -> u32 {
    let scaled = BASE_DIFFICULTY.saturating_add(op_cost / COST_PER_BIT);
    core::cmp::min(scaled, MAX_DIFFICULTY)
}

/// Issue a PoW challenge for an operation of the given cost.
pub fn issue_pow_challenge(env: &Env, op_cost: u32) -> PowChallenge {
    let difficulty = difficulty_for_cost(op_cost);
    let seq = env.ledger().sequence();
    let ts = env.ledger().timestamp();
    let mut buf = [0u8; 32];
    buf[0..8].copy_from_slice(&ts.to_be_bytes());
    buf[8..12].copy_from_slice(&seq.to_be_bytes());
    buf[12..16].copy_from_slice(&op_cost.to_be_bytes());
    buf[16..24].copy_from_slice(&env.ledger().max_live_until_ledger().to_be_bytes());
    // Mix in previous bytes for uniqueness without host RNG.
    let mut x = 0x9e3779b97f4a7c15u64;
    for (i, b) in buf.iter_mut().enumerate().take(32).skip(24) {
        x ^= (i as u64).wrapping_mul(0xbf58476d1ce4e5b9);
        x ^= x >> 30;
        x = x.wrapping_mul(0xbf58476d1ce4e5b9);
        *b = (x & 0xff) as u8;
    }
    PowChallenge {
        challenge: BytesN::from_array(env, &buf),
        difficulty,
        issued_at_ledger: seq,
    }
}

/// Count leading zero bits of the sha256(challenge || nonce_be).
pub fn pow_hash_meets(env: &Env, challenge: &BytesN<32>, nonce: u64, difficulty: u32) -> bool {
    let mut input = Bytes::new(env);
    for b in challenge.to_array().iter() {
        input.push_back(*b);
    }
    for b in nonce.to_be_bytes().iter() {
        input.push_back(*b);
    }
    let digest = env.crypto().sha256(&input);
    let bytes = digest.to_array();
    let mut zeros = 0u32;
    for b in bytes.iter() {
        if *b == 0 {
            zeros += 8;
        } else {
            zeros += b.leading_zeros();
            break;
        }
        if zeros >= difficulty {
            break;
        }
    }
    zeros >= difficulty
}

/// Validate a PoW solution before executing a resource-heavy function.
/// Returns `InvalidInput` on stale challenges or insufficient work.
pub fn verify_pow(
    env: &Env,
    challenge: &PowChallenge,
    nonce: u64,
    op_cost: u32,
) -> Result<(), ContractError> {
    let required = difficulty_for_cost(op_cost);
    let difficulty = core::cmp::max(challenge.difficulty, required);
    if difficulty > MAX_DIFFICULTY {
        return Err(ContractError::InvalidInput);
    }
    if env.ledger().sequence() > challenge.issued_at_ledger.saturating_add(CHALLENGE_TTL_LEDGERS) {
        return Err(ContractError::InvalidInput);
    }
    if !pow_hash_meets(env, &challenge.challenge, nonce, difficulty) {
        return Err(ContractError::InvalidInput);
    }
    Ok(())
}

/// Client-side helper: brute-force a nonce meeting `difficulty`.
///
/// Intended for off-chain clients and integration tests. Iterates from 0;
/// with the default difficulty (~12 bits) this resolves in milliseconds.
pub fn find_pow_nonce(env: &Env, challenge: &BytesN<32>, difficulty: u32) -> u64 {
    let mut nonce = 0u64;
    loop {
        if pow_hash_meets(env, challenge, nonce, difficulty) {
            return nonce;
        }
        nonce = nonce.saturating_add(1);
        if nonce == u64::MAX {
            panic!("PoW solution not found");
        }
    }
}

pub struct PowGate;

impl PowGate {
    pub fn issue(env: &Env, op_cost: u32) -> PowChallenge {
        issue_pow_challenge(env, op_cost)
    }
    pub fn verify(
        env: &Env,
        challenge: &PowChallenge,
        nonce: u64,
        op_cost: u32,
    ) -> Result<(), ContractError> {
        verify_pow(env, challenge, nonce, op_cost)
    }
    pub fn difficulty_for(op_cost: u32) -> u32 {
        difficulty_for_cost(op_cost)
    }
}
