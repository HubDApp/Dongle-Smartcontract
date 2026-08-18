# Changelog

All notable changes to the Dongle Smart Contract are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- **Repository hygiene:** Consolidated repository-root documentation. Reference
  documentation now lives in `docs/` (`CONTRACT_INTERFACE.md`,
  `CONTRIBUTING.md`, `DATA_EXPORT_GUIDE.md`, `ERROR_CODES.md`,
  `EVENTS_SCHEMA.md`, `STORAGE_INDEXES.md`, `THREAT_MODEL.md`), and transient
  internal status/summary notes were removed from the root. The repository root
  now contains only `README.md`, `DEPLOYMENT.md`, and this `CHANGELOG.md`.
- Updated stale documentation links in `README.md`, `docs/`,
  `dongle-smartcontract/README.md`, and `bug-bounty/README.md` to point at the
  consolidated `docs/` paths.

## [2026-08-01] - Dispute Flow, Pagination & Registry Hygiene

### Added

- Duplicate project dispute flow (#151).
- Possible-next-state coverage for verification state machine.
- Project changelog CID feature (#157).

### Changed

- Renamed offset pagination cursor parameters to `start_index` (#351).
- Unified claim status handling across both claim workflows (#357).
- Consolidated registry helpers (`remove_item_from_vec`), removed unused
  `FeeRefundRecord` scaffolding, and repaired `utils.rs` corruption.
- CI fix: return contract errors instead of raw panics for timelock and
  endorsement operations.

## [2026-07-01] - Verification Expiry, Anti-Sybil & Safety Hardening

### Added

- Verification expiry implementation (#131).
- Anti-Sybil review constraints (reviewer eligibility age, endorsements,
  review fees).
- Emergency stop / contract pause with admin pause toggle.
- `get_config` view for public configuration read access.
- `MIN_STRING_LEN` validation constants.
- Admin console contract configuration view.

### Changed

- Replaced raw panics with `ContractError` variants; added 22 missing
  `ContractError` variants (61-82).
- Collapsed ~20 near-identical `extend_*_ttl` storage helpers into a single
  generic helper (#338).
- Consolidated CID validators and error handling.
- Extended `.gitignore` coverage for test output files; Makefile cleanup
  (#384).

## [2026-06-01] - Fee Handling, Endorsements & Deployment Readiness

### Added

- Comprehensive fee payment error handling and token transfer failure tests.
- Fee refund flow, expiry, config history, and SLA tracking.
- Native asset fee guardrails and token-only fee policy clarification.
- Historical verification records and admin multisig approval threshold.
- Metadata freeze policy for verified projects.
- Project endorsements.
- Bug bounty metadata and `bounty_url` on projects.
- Optional project license field.
- Verification evidence updates while pending.
- Fee boundary validation, review tombstones, sorting, and review update cooldown.
- Fee payment payer getter, reserved names, verification assignment, and logo
  asset guidelines.
- WASM optimization, invariant tests, and property-based pagination tests.
- Weighted rating calculation.

### Changed

- Soroban deployment and schema documentation.
- Example/schema JSON files co-located in `docs/`.

## [2026-04-01] - TTL Management, Archive & Review Moderation

### Added

- TTL (time-to-live) management for data persistence.
- Admin action logging.
- Project archive & reactivate feature.
- Project slug (URL-friendly stable identifiers).
- Review moderation (report/hide) feature.

### Changed

- Verification evidence schema and update flows.

## [2026-02-01] - Extended Feature Set

### Added

- Admin role management and access control system.
- Project verification system with automated fee handling and admin controls.
- Review timestamps and review security/auditability improvements.
- Strict project name validation and normalization policy.
- Auth-matrix test coverage (#215).
- Region metadata, integrity hash, owner review block, and event snapshot
  tests.
- Verified contract claims and sorting options.
- Project metadata CID schema and admin rotation playbook.

## [2025-09-25] - Initial Contract

### Added

- Initial Rust/Soroban smart contract structure.
- Core Dongle smart contract modules: project, review, verification, and fee
  management, with associated types, events, and error handling.
- On-chain review events and tests.
- Dynamic ratings: type definitions, rating calculation module, `add_review`,
  `update_review`, and `delete_review` with aggregate recalculation.
- Category enumeration and unique sequential project IDs.
- Project registration with duplicate-prevention metadata validation.
- Project listing and retrieval by id/owner.
- Emitted `ProjectRegistered` event.
- Project review submission with IPFS CID.
- Review soft deletion and rating recalculation.
