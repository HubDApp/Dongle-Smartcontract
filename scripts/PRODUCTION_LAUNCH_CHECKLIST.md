# Production Launch Checklist

This checklist must be completed before deploying the Dongle smart contract to Stellar Mainnet. Sign off each item with the reviewer's initials and date.

---

## 1. Code Quality & Testing

- [ ] All unit tests pass: `cargo test --workspace`
- [ ] Property-based tests pass (rating math invariants verified)
- [ ] WASM build succeeds without warnings: `cargo build --target wasm32-unknown-unknown --release`
- [ ] No `panic!`, `unwrap()`, or `expect()` calls remain in production code paths
- [ ] All public functions return `Result<_, ContractError>` or safe types (no silent failures)
- [ ] Contract compiles with `overflow-checks = true` (confirmed in `Cargo.toml` `[profile.release]`)
- [ ] No unused imports or dead code that could mask bugs (`cargo clippy`)

---

## 2. Security Review

- [ ] All state-modifying functions require auth (`caller.require_auth()` called before writes)
- [ ] All admin-only functions verify `AdminManager::is_admin` before executing
- [ ] Fee collection paths cannot be bypassed (fee config validated before any transfer)
- [ ] No integer overflow possible in rating math (all arithmetic uses saturating ops or checked ops)
- [ ] `DependencyRef` external_contract addresses are validated (56-char Strkey, 'C' prefix, base32)
- [ ] No reentrancy risk in fee payment flows (Soroban's single-threaded execution model reviewed)
- [ ] Threat model (`THREAT_MODEL.md`) reviewed and all listed mitigations are implemented
- [ ] Access control matrix reviewed: admin / owner / public actions are correctly scoped
- [ ] Timelock minimum delay (`TIMELOCK_MIN_DELAY = 86400s`) enforced for all scheduled actions

---

## 3. Contract Initialization & Configuration

- [ ] `initialize(admin)` will be called exactly once post-deploy (idempotency guard verified)
- [ ] Initial admin address is a multisig or hardware-wallet-controlled key, NOT a hot wallet
- [ ] Fee configuration (`set_fee`) will be set before accepting any verification requests
- [ ] Treasury address is confirmed and under secure key management
- [ ] Minimum project age (`set_min_project_age`) configured if early-review abuse is a concern
- [ ] Verification duration (`set_verification_duration`) set to appropriate validity window

---

## 4. Deployment Artifacts

- [ ] Release WASM built with `opt-level = "z"`, `lto = true`, `strip = "symbols"` (size-optimized)
- [ ] WASM file size is within Stellar's contract upload limits
- [ ] WASM hash recorded in `deployments.json` with correct format (64-char hex)
- [ ] `deployments.json` validated: `python3 scripts/validate_deployments.py` exits successfully
- [ ] Contract ID (starts with `C`, 56 chars) recorded in `deployments.json`
- [ ] Deployer public key recorded (starts with `G`, 56 chars)
- [ ] Deployment timestamp recorded in ISO 8601 format

---

## 5. Testnet Validation

- [ ] Full deployment executed on Testnet using `scripts/deploy_testnet.sh`
- [ ] `initialize` called on Testnet deployment
- [ ] End-to-end workflow tested on Testnet:
  - [ ] Project registration (with and without fees)
  - [ ] Review submission, update, and deletion
  - [ ] Verification request → approval flow
  - [ ] Verification renewal flow
  - [ ] Admin action log entries created correctly
  - [ ] Collection create / add / remove project
  - [ ] Dispute open and resolution
  - [ ] External contract dependencies (add, remove, duplicate rejection)
- [ ] Rating math validated on Testnet: average updates correctly after add/update/delete
- [ ] Fee payment flow validated on Testnet (if fees enabled)
- [ ] Timelock scheduling and execution validated on Testnet

---

## 6. Data Schema & Events

- [ ] Event schema (`EVENTS_SCHEMA.md`) matches emitted events in production code
- [ ] Review CID schema (`review-cid.schema.json`) validated against example (`review-cid.example.json`)
- [ ] Data export guide (`DATA_EXPORT_GUIDE.md`) tested against Testnet data
- [ ] Indexers confirmed to be handling all event types in `EVENTS_SCHEMA.md`

---

## 7. Operational Readiness

- [ ] On-call runbook created for common issues (verification stuck, fee config drift)
- [ ] TTL extension strategy defined (who extends TTLs and how often for critical data)
- [ ] Monitoring alert configured for transaction failures on the contract address
- [ ] Admin key rotation procedure documented and tested on Testnet
- [ ] Multisig approval quorum for admin proposals (`set_admin_approval_threshold`) confirmed
- [ ] Emergency response plan: how to pause operations if a critical bug is found post-launch

---

## 8. Final Sign-off

| Item | Reviewer | Date | Notes |
|------|----------|------|-------|
| Code review complete | | | |
| Security review complete | | | |
| Testnet validation complete | | | |
| Deployment artifacts verified | | | |
| Operational readiness confirmed | | | |
| **GO / NO-GO decision** | | | |

---

*After a successful Mainnet deployment, update `deployments.json` and tag the commit with the contract version.*
