//! Address endorsement graph and multi-hop reputation chains (#917).
//!
//! Users endorse other users (`endorse_address`), forming a directed graph
//! where an edge `a → b` means "a vouches for b".  On top of the graph:
//!
//! * `get_endorsement_path(from, to)` finds the shortest transitive chain.
//! * `get_vouchers(target, max_depth)` answers "who can vouch for this
//!   person?" by walking incoming edges.
//! * `detect_endorsement_cycle(start)` returns a cycle through `start`, if
//!   one exists.
//!
//! ## Chain weight
//!
//! A chain starts at `MAX_BASIS_POINTS` (a direct endorsement).  Every hop
//! beyond the first multiplies the weight by `ENDORSEMENT_HOP_DECAY_BPS`, and
//! every intermediary multiplies it by its reputation factor:
//! `min(MAX_BASIS_POINTS, ENDORSEMENT_BASE_REPUTATION_BPS +
//! ENDORSEMENT_REPUTATION_STEP_BPS * endorser_count)`.
//!
//! ## Bounded traversal
//!
//! All traversals are breadth-first with a visited set, so cycles never cause
//! repeated work, and are capped at `MAX_ENDORSEMENT_CHAIN_DEPTH` hops and
//! `MAX_ENDORSEMENT_GRAPH_NODES` visited addresses.

use crate::constants::{
    ENDORSEMENT_BASE_REPUTATION_BPS, ENDORSEMENT_HOP_DECAY_BPS, ENDORSEMENT_REPUTATION_STEP_BPS,
    MAX_ADDRESS_ENDORSEMENTS_PER_USER, MAX_BASIS_POINTS, MAX_ENDORSEMENT_CHAIN_DEPTH,
    MAX_ENDORSEMENT_GRAPH_NODES,
};
use crate::errors::ContractError;
use crate::events::{publish_address_endorsed_event, publish_address_endorsement_revoked_event};
use crate::storage_keys::EndorsementGraphKey;
use crate::types::{EndorsementChain, EndorsementVoucher};
use crate::utils::Utils;
use soroban_sdk::{Address, Env, Map, Vec};

pub struct EndorsementGraph;

impl EndorsementGraph {
    // ── Edges ─────────────────────────────────────────────────────────────

    /// `endorser` vouches for `endorsee`.
    pub fn endorse_address(
        env: &Env,
        endorser: Address,
        endorsee: Address,
    ) -> Result<(), ContractError> {
        endorser.require_auth();
        if endorser == endorsee {
            return Err(ContractError::SelfEndorsement);
        }

        let mut outgoing = Self::outgoing(env, &endorser);
        if outgoing.contains(&endorsee) {
            return Err(ContractError::AddressAlreadyEndorsed);
        }
        let mut incoming = Self::incoming(env, &endorsee);
        if outgoing.len() >= MAX_ADDRESS_ENDORSEMENTS_PER_USER
            || incoming.len() >= MAX_ADDRESS_ENDORSEMENTS_PER_USER
        {
            return Err(ContractError::MaxAddressEndorsementsExceeded);
        }

        outgoing.push_back(endorsee.clone());
        incoming.push_back(endorser.clone());
        env.storage()
            .persistent()
            .set(&EndorsementGraphKey::Outgoing(endorser.clone()), &outgoing);
        env.storage()
            .persistent()
            .set(&EndorsementGraphKey::Incoming(endorsee.clone()), &incoming);

        publish_address_endorsed_event(env, endorser, endorsee);
        Ok(())
    }

    /// Remove the `endorser → endorsee` edge.
    pub fn revoke_address_endorsement(
        env: &Env,
        endorser: Address,
        endorsee: Address,
    ) -> Result<(), ContractError> {
        endorser.require_auth();

        let outgoing = Self::outgoing(env, &endorser);
        if !outgoing.contains(&endorsee) {
            return Err(ContractError::AddressNotEndorsed);
        }
        let incoming = Self::incoming(env, &endorsee);

        env.storage().persistent().set(
            &EndorsementGraphKey::Outgoing(endorser.clone()),
            &Utils::remove_item_from_vec(env, &outgoing, &endorsee),
        );
        env.storage().persistent().set(
            &EndorsementGraphKey::Incoming(endorsee.clone()),
            &Utils::remove_item_from_vec(env, &incoming, &endorser),
        );

        publish_address_endorsement_revoked_event(env, endorser, endorsee);
        Ok(())
    }

    pub fn has_endorsed_address(env: &Env, endorser: &Address, endorsee: &Address) -> bool {
        Self::outgoing(env, endorser).contains(endorsee)
    }

    /// Addresses `user` endorses.
    pub fn get_address_endorsements(env: &Env, user: Address) -> Vec<Address> {
        Self::outgoing(env, &user)
    }

    /// Addresses endorsing `user`.
    pub fn get_address_endorsers(env: &Env, user: Address) -> Vec<Address> {
        Self::incoming(env, &user)
    }

    /// Reputation of an address, measured as its direct endorser count.
    pub fn get_endorsement_reputation(env: &Env, user: Address) -> u32 {
        Self::incoming(env, &user).len()
    }

    // ── Chain analysis ────────────────────────────────────────────────────

    /// Shortest endorsement chain from `from` to `to`, if one exists within
    /// `MAX_ENDORSEMENT_CHAIN_DEPTH` hops.
    pub fn get_endorsement_path(env: &Env, from: Address, to: Address) -> Option<EndorsementChain> {
        if from == to {
            return None;
        }
        let path = Self::shortest_path(env, &from, &to)?;
        Some(Self::build_chain(env, from, to, path))
    }

    /// Every address that can vouch for `target` through a chain of at most
    /// `max_depth` hops (0 or anything above the cap means the cap), nearest
    /// first.
    pub fn get_vouchers(env: &Env, target: Address, max_depth: u32) -> Vec<EndorsementVoucher> {
        let max_depth = Self::effective_depth(max_depth);

        // `toward[v]` is the next address on v's shortest chain to `target`.
        let mut toward: Map<Address, Address> = Map::new(env);
        let mut depth: Map<Address, u32> = Map::new(env);
        let mut queue: Vec<Address> = Vec::new(env);
        let mut order: Vec<Address> = Vec::new(env);
        depth.set(target.clone(), 0);
        queue.push_back(target.clone());

        while let Some(node) = queue.pop_front() {
            let node_depth = depth.get(node.clone()).unwrap_or(0);
            if node_depth >= max_depth {
                continue;
            }
            for endorser in Self::incoming(env, &node).iter() {
                if depth.contains_key(endorser.clone()) {
                    continue;
                }
                if depth.len() >= MAX_ENDORSEMENT_GRAPH_NODES {
                    break;
                }
                depth.set(endorser.clone(), node_depth + 1);
                toward.set(endorser.clone(), node.clone());
                order.push_back(endorser.clone());
                queue.push_back(endorser);
            }
        }

        let mut vouchers = Vec::new(env);
        for voucher in order.iter() {
            let mut path = Vec::new(env);
            let mut cur = voucher.clone();
            path.push_back(cur.clone());
            while cur != target {
                cur = toward.get(cur).expect("voucher chain is connected");
                path.push_back(cur.clone());
            }
            let hops = path.len() - 1;
            vouchers.push_back(EndorsementVoucher {
                voucher,
                hops,
                weight: Self::chain_weight(env, &path),
            });
        }
        vouchers
    }

    /// A shortest endorsement cycle through `start` (`start` first and last),
    /// or `None` if `start` is not on a cycle within the traversal limits.
    pub fn detect_endorsement_cycle(env: &Env, start: Address) -> Option<Vec<Address>> {
        Self::shortest_path(env, &start, &start)
    }

    pub fn has_endorsement_cycle(env: &Env, start: Address) -> bool {
        Self::detect_endorsement_cycle(env, start).is_some()
    }

    /// Weight (bps) of a chain given as an ordered address path.
    pub fn chain_weight(env: &Env, path: &Vec<Address>) -> u32 {
        let len = path.len();
        if len < 2 {
            return 0;
        }
        let bps = MAX_BASIS_POINTS as u64;
        let mut weight = bps;
        for _ in 1..(len - 1) {
            weight = weight * ENDORSEMENT_HOP_DECAY_BPS as u64 / bps;
        }
        for i in 1..(len - 1) {
            let intermediary = path.get(i).expect("index within path");
            weight = weight * Self::reputation_factor(env, &intermediary) as u64 / bps;
        }
        weight as u32
    }

    // ── Internal helpers ──────────────────────────────────────────────────

    fn outgoing(env: &Env, user: &Address) -> Vec<Address> {
        env.storage()
            .persistent()
            .get(&EndorsementGraphKey::Outgoing(user.clone()))
            .unwrap_or_else(|| Vec::new(env))
    }

    fn incoming(env: &Env, user: &Address) -> Vec<Address> {
        env.storage()
            .persistent()
            .get(&EndorsementGraphKey::Incoming(user.clone()))
            .unwrap_or_else(|| Vec::new(env))
    }

    fn effective_depth(max_depth: u32) -> u32 {
        if max_depth == 0 || max_depth > MAX_ENDORSEMENT_CHAIN_DEPTH {
            MAX_ENDORSEMENT_CHAIN_DEPTH
        } else {
            max_depth
        }
    }

    fn reputation_factor(env: &Env, user: &Address) -> u32 {
        let endorsers = Self::incoming(env, user).len();
        ENDORSEMENT_BASE_REPUTATION_BPS
            .saturating_add(ENDORSEMENT_REPUTATION_STEP_BPS.saturating_mul(endorsers))
            .min(MAX_BASIS_POINTS)
    }

    fn build_chain(env: &Env, from: Address, to: Address, path: Vec<Address>) -> EndorsementChain {
        EndorsementChain {
            from,
            to,
            hops: path.len() - 1,
            weight: Self::chain_weight(env, &path),
            path,
        }
    }

    /// Breadth-first search along outgoing edges.  `from == to` searches for
    /// a cycle back to `from`.  Returns the path including both endpoints.
    fn shortest_path(env: &Env, from: &Address, to: &Address) -> Option<Vec<Address>> {
        let mut parent: Map<Address, Address> = Map::new(env);
        let mut depth: Map<Address, u32> = Map::new(env);
        let mut queue: Vec<Address> = Vec::new(env);
        depth.set(from.clone(), 0);
        queue.push_back(from.clone());

        while let Some(node) = queue.pop_front() {
            let node_depth = depth.get(node.clone()).unwrap_or(0);
            if node_depth >= MAX_ENDORSEMENT_CHAIN_DEPTH {
                continue;
            }
            for next in Self::outgoing(env, &node).iter() {
                if next == *to {
                    return Some(Self::reconstruct(env, &parent, from, node, next));
                }
                if depth.contains_key(next.clone()) {
                    continue;
                }
                if depth.len() >= MAX_ENDORSEMENT_GRAPH_NODES {
                    return None;
                }
                depth.set(next.clone(), node_depth + 1);
                parent.set(next.clone(), node.clone());
                queue.push_back(next);
            }
        }
        None
    }

    fn reconstruct(
        env: &Env,
        parent: &Map<Address, Address>,
        from: &Address,
        last: Address,
        to: Address,
    ) -> Vec<Address> {
        let mut reversed = Vec::new(env);
        reversed.push_back(to);
        let mut cur = last;
        while cur != *from {
            reversed.push_back(cur.clone());
            cur = parent.get(cur).expect("path parent recorded");
        }
        reversed.push_back(from.clone());

        let mut path = Vec::new(env);
        for i in (0..reversed.len()).rev() {
            path.push_back(reversed.get(i).expect("index within path"));
        }
        path
    }
}
