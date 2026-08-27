# Changelog

All notable changes to the **Dongle Smart Contract** are documented in this file.

The format is based on [Keep a Changelog 1.1.0](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## How to read this file

- Every release has its own `## [MAJOR.MINOR.PATCH] - YYYY-MM-DD` heading, newest first.
- Changes are grouped under the Keep a Changelog categories: `Added`, `Changed`,
  `Deprecated`, `Removed`, `Fixed`, `Security`.
- Entries marked **BREAKING** change an on-chain interface (function signature,
  storage key, event topic, or error code) and require operator action before or
  during upgrade. See [`DEPLOYMENT.md`](DEPLOYMENT.md) for the upgrade procedure.
- Unreleased work lands under `## [Unreleased]` and is folded into the next
  version at release time.

## How to add an entry

Contributors must add a bullet to `## [Unreleased]` in the same pull request as
their change. Keep entries user-facing, one line each, and reference the issue or
PR number where available:

```markdown
## [Unreleased]

### Added

- New `get_config` view returning public contract configuration (#202).
```

Run `python3 scripts/validate_changelog.py` locally before pushing; CI runs the
same check. See [`docs/CONTRIBUTING.md`](docs/CONTRIBUTING.md#6-changelog-entries)
for the full policy.

---

## [Unreleased]

### Added

- Tag validation now rejects duplicate values (case-insensitive after ASCII
  lowercase normalization) with `InvalidTags` (#526).
- **Governance: threshold-downgrade supermajority rule.** A
  `ProposalPayload::SetThreshold` proposal that would *lower* the current
  approval threshold now requires strictly more approvals than the proposed new
  threshold before it can execute. This prevents a colluding group of exactly
  `new_threshold` admins from using the proposal path to silently dismantle the
  multi-sig quorum. Raises new error `ThresholdDowngradeRequiresSupermajority`
  (code 74) when the guard is violated. Threshold *increases* and no-ops are
  unaffected and still require only the live threshold.
- **Integration test: full verification-fee payment lifecycle**
  (`src/tests/fee_lifecycle.rs`). Nine tests covering: pre-payment rejection,
  `pay_fee` sets flag and records details, token balances correct, flag cleared
  after `request_verification`, second request without re-payment rejected with
  `InsufficientFee`, re-payment restores the flag, payment-details audit record
  retained after consumption, treasury balance accounting.
- **Architecture documentation** (`docs/ARCHITECTURE.md`). A new contributor
  reference covering: four-layer ASCII module map, Mermaid dependency graph for
  all 20+ modules, two annotated Mermaid sequence diagrams (`request_verification`
  happy path and multi-sig proposal lifecycle), complete storage-key tables for
  `StorageKey` and `ExtensionKey`, event taxonomy table, and a full module
  reference. Linked from `README.md` Quick Links and Documentation sections.

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
- Pinned the Rust toolchain (1.85.0) in CI workflows for reproducible builds
  (#514).
- Timelocked admin proposals now verify the proposal payload hash before
  execution.
- **Governance: `set_admin_approval_threshold` documentation clarified.** The
  function is intentionally blocked (returns `Unauthorized`) once the threshold
  exceeds 1. All threshold changes while multi-sig is active — including
  lowering — must go through `create_proposal` / `execute_proposal` and are
  subject to the supermajority rule described above.
- **Git hygiene: removed stale snapshot files from index.** Six Soroban test
  environment snapshots under `dongle-smartcontract/test_snapshots/` were
  tracked despite the `test_snapshots/` ignore rule in
  `dongle-smartcontract/.gitignore`. They were untracked via
  `git rm --cached` (the files remain on disk for any local snapshot test
  runner). The root `.gitignore` now also carries an explicit
  `dongle-smartcontract/test_snapshots/` entry so the rule is honoured
  regardless of which directory git is invoked from.

### Removed

- Committed test output text files (`test_output.txt`, `test_final.txt`,
  `test_output_latest.txt`) from the `dongle-smartcontract/` directory (#503).

### Fixed

- Documented previously undocumented verification events in
  `docs/EVENTS_SCHEMA.md` (#508).
- Applied `cargo fmt --all` across the workspace, clearing the pre-existing
  `rustfmt` drift in 18 source files that was failing the CI `Formatting` job
  and blocking the `Build Contract` and `Optimize WASM` jobs. Formatting only —
  no logic, signature, storage key, event or error code was changed.
- CI `Optimize WASM` job: install the Stellar CLI from its prebuilt release
  binary (pinned to 27.1.0) instead of `cargo install --locked stellar-cli
  --features opt`. The published crate now requires a rustc newer than the
  toolchain pinned in `rust-toolchain.toml` (1.85.0), so building it from
  source failed immediately; the prebuilt binary already bundles `wasm-opt`
  and installs in seconds.
- `scripts/optimize_wasm.sh` now selects an optimizer at runtime
  (`stellar contract optimize`, then `wasm-opt -Oz`), verifies the output file
  was produced, and exits with actionable install instructions when no
  optimizer is available.

## [0.6.0] - 2026-08-01

_Dispute flow, pagination and registry hygiene._

### Added

- Duplicate project dispute flow (#151).
- Possible-next-state coverage for the verification state machine.
- Project changelog CID feature, letting projects publish their own release
  notes off-chain (#157).

### Changed

- **BREAKING:** Renamed offset pagination cursor parameters to `start_index`
  across all paginated views; callers passing the old parameter name must be
  updated (#351).
- **BREAKING:** Unified claim status handling across both claim workflows, so a
  single `ClaimStatus` enum is now returned by both paths (#357).
- Consolidated registry helpers into `remove_item_from_vec` and removed the
  unused `FeeRefundRecord` scaffolding.

### Fixed

- Repaired corruption in `utils.rs` and restored the missing
  `validate_report_reason_cid` function (#408).
- Timelock and endorsement operations now return `ContractError` values instead
  of raw panics.

## [0.5.0] - 2026-07-01

_Verification expiry, anti-Sybil controls and safety hardening._

### Added

- Verification expiry implementation (#131).
- Anti-Sybil review constraints: reviewer eligibility age, endorsement
  requirements and review fees.
- Emergency stop / contract pause with an admin pause toggle.
- `get_config` view for public configuration read access (#202).
- `MIN_STRING_LEN` validation constants.
- Admin console contract configuration view.

### Changed

- **BREAKING:** Replaced raw panics with `ContractError` variants and added 22
  new error codes (61-82); integrators must map the new codes. See
  [`docs/ERROR_CODES.md`](docs/ERROR_CODES.md).
- Collapsed roughly 20 near-identical `extend_*_ttl` storage helpers into a
  single generic helper (#338).
- Consolidated CID validators and their error handling.
- Extended `.gitignore` coverage for test output files and cleaned up the
  Makefile (#384).

### Security

- Emergency pause blocks state-mutating entry points while active, limiting
  blast radius during incident response.

## [0.4.0] - 2026-06-01

_Fee handling, endorsements and deployment readiness._

### Added

- Comprehensive fee payment error handling and token transfer failure tests.
- Fee refund flow, fee expiry, configuration history and SLA tracking.
- Native asset fee guardrails and a token-only fee policy.
- Historical verification records and an admin multisig approval threshold.
- Metadata freeze policy for verified projects.
- Project endorsements.
- Bug bounty metadata and a `bounty_url` field on projects.
- Optional project license field.
- Verification evidence updates while a request is pending.
- Fee boundary validation, review tombstones, review sorting and a review
  update cooldown.
- Fee payment payer getter, reserved names, verification assignment and logo
  asset guidelines.
- WASM size optimization, invariant tests and property-based pagination tests.
- Weighted rating calculation.

### Changed

- Expanded Soroban deployment and schema documentation.
- Co-located example and schema JSON files in `docs/`.

## [0.3.0] - 2026-04-01

_TTL management, archival and review moderation._

### Added

- TTL (time-to-live) management for data persistence.
- Admin action logging.
- Project archive and reactivate feature.
- Project slugs (URL-friendly stable identifiers).
- Review moderation (report / hide) feature.

### Changed

- Reworked the verification evidence schema and update flows.

## [0.2.0] - 2026-02-01

_Extended feature set._

### Added

- Admin role management and access control system.
- Project verification system with automated fee handling and admin controls.
- Review timestamps plus review security and auditability improvements.
- Strict project name validation and normalization policy.
- Auth-matrix test coverage (#215).
- Region metadata, integrity hash, owner review block and event snapshot tests.
- Verified contract claims and sorting options.
- Project metadata CID schema and the admin rotation playbook.

## [0.1.0] - 2025-09-25

_Initial contract._

### Added

- Initial Rust / Soroban smart contract structure.
- Core Dongle modules: project, review, verification and fee management, with
  associated types, events and error handling.
- On-chain review events and tests.
- Dynamic ratings: type definitions, rating calculation module, `add_review`,
  `update_review` and `delete_review` with aggregate recalculation.
- Category enumeration and unique sequential project IDs.
- Project registration with duplicate-prevention metadata validation.
- Project listing and retrieval by id and by owner.
- Emitted `ProjectRegistered` event.
- Project review submission with IPFS CID.
- Review soft deletion with rating recalculation.

[Unreleased]: https://github.com/felladaniel36-hash/Dongle-Smartcontract/compare/v0.6.0...HEAD
[0.6.0]: https://github.com/felladaniel36-hash/Dongle-Smartcontract/compare/v0.5.0...v0.6.0
[0.5.0]: https://github.com/felladaniel36-hash/Dongle-Smartcontract/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/felladaniel36-hash/Dongle-Smartcontract/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/felladaniel36-hash/Dongle-Smartcontract/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/felladaniel36-hash/Dongle-Smartcontract/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/felladaniel36-hash/Dongle-Smartcontract/releases/tag/v0.1.0
