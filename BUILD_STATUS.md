# Build & Deployment Status

**Last Updated:** 2026-08-27  
**Status:** ⚠️ **NOT READY FOR DEPLOYMENT**

This document serves as the single, authoritative source for the project's current build, test, and deployment readiness status. It is updated whenever significant changes affect compilability or deployment viability.

## Current State

### Build Status

- **Rust Compilation:** 🟢 **PASSING** - `wasm32-unknown-unknown --release` compiles with no errors or warnings
- **Test Suite:** 🟡 **UNKNOWN** - Not verified as part of the build fix
- **WASM Target:** 🟢 **PASSING** - `dongle_contract.wasm` is produced at `target/wasm32-unknown-unknown/release/dongle_contract.wasm`
- **Documentation:** ✅ **UP-TO-DATE** - Deployment and interface docs are current

### Why "Ready" Documents Are Misleading

Multiple documents in the repo history may claim the project is "complete," "ready," or "final" (e.g., ALL_TASKS_COMPLETION_SUMMARY.md, FINAL_PROJECT_SUMMARY.md, etc.). **These are stale.** Compilation is now fixed, but the full test suite and deployment checklist have not been re-verified, so the project is still **not** ready for deployment.

**This BUILD_STATUS.md is the only document you should consult for current project readiness.**

## Build Issues

### Known Compilation Errors

**Resolved (issue #608).** The `wasm32-unknown-unknown --release` build failed with 23 compiler
errors caused by a bad merge that dropped pieces of several partially-landed features:

- New `ContractError` variants referenced but never declared (`MultiSigRequired`,
  `FeePaymentExpired`, `LinkedProjectNotFound`, `AlreadyMaintainerAdded`).
- `fee_manager.rs` missing the `FEE_PAYMENT_EXPIRY_SECONDS` import, the
  `get_fee_config_history` getter, and the `ExtensionKey` storage key for fee-config history.
- A duplicated `get_verifications_batch` contract method in `lib.rs`, plus a missing
  `get_verification_records_batch` on `VerificationRegistry`.
- A borrow/`contains_key` type mismatch in `admin_manager.rs`.
- `[profile.*]` tables in the non-root member crate (ignored by Cargo, emitted a warning).

All are fixed; the build now completes cleanly and produces `dongle_contract.wasm`.

### How to Check Current Build Status

```bash
cd dongle-smartcontract

# Check if compilation succeeds
cargo build --target wasm32-unknown-unknown --release

# Check tests
cargo test

# Verify no warnings or errors
cargo check
```

If any of these commands fail, the project is **NOT READY** for deployment.

## Deployment Readiness Checklist

Use this checklist to determine if deployment is viable:

- [x] ✅ Cargo build succeeds on `wasm32-unknown-unknown --release`
- [ ] ✅ All tests pass (`cargo test`)
- [x] ✅ No compiler warnings or errors
- [ ] ✅ Threat model reviewed (see [THREAT_MODEL.md](./docs/THREAT_MODEL.md))
- [ ] ✅ Admin rotation procedures verified (see [ADMIN_ROTATION_PLAYBOOK.md](./docs/ADMIN_ROTATION_PLAYBOOK.md))
- [ ] ✅ Deployment checklist completed (see [DEPLOYMENT_AUDIT_CHECKLIST.md](./docs/DEPLOYMENT_AUDIT_CHECKLIST.md))
- [ ] ✅ Contract interface is stable (no breaking changes in [CONTRACT_INTERFACE.md](./docs/CONTRACT_INTERFACE.md))
- [ ] ✅ Storage schema is finalized (review [STORAGE_SCHEMA.md](./docs/STORAGE_SCHEMA.md))

**Current Status:** 2/8 checks passing. **DO NOT DEPLOY.**

## What to Do Now

### For Build Fixes

1. Run the build and capture full error output:
   ```bash
   cargo build --target wasm32-unknown-unknown --release 2>&1 | tee build-output.log
   ```

2. Investigate root causes in the compiler output

3. Fix issues and update this document when build succeeds

4. Run full test suite to ensure no regressions

### For Contributors

- Do not create or append to completion documents
- Update **only this BUILD_STATUS.md** when status changes
- Run `cargo check` before committing
- Report any new build issues via pull request

## Previous Status Documents

The following documents may exist in the repo history but are **DEPRECATED** and should be **ARCHIVED or DELETED**:

- ALL_TASKS_COMPLETION_SUMMARY.md *(if present)*
- FINAL_PROJECT_SUMMARY.md *(if present)*
- FINAL_STATUS.md *(if present)*
- IMPLEMENTATION_COMPLETE.md *(if present)*
- IMPLEMENTATION_SUMMARY.md *(if present)*
- DEPLOYMENT_READY_SUMMARY.md *(if present)*
- FINAL_DEPLOYMENT_UPDATE.md *(if present)*
- FINAL_VERIFICATION.md *(if present)*
- PUSH_SUMMARY.md *(if present)*

These documents are misleading when the project does not compile. **Ignore them.** Consult only this BUILD_STATUS.md for authoritative status.

## Related Documentation

For deployment-related information, refer to:

- **[DEPLOYMENT.md](./DEPLOYMENT.md)** — Deployment manifest schema and deployment tracking
- **[DEPLOYMENT_AUDIT_CHECKLIST.md](./docs/DEPLOYMENT_AUDIT_CHECKLIST.md)** — Pre-deployment validation checklist
- **[INITIALIZATION_DEPLOYMENT_CHECKLIST.md](./docs/INITIALIZATION_DEPLOYMENT_CHECKLIST.md)** — Post-deployment initialization steps
- **[CONTRACT_INTERFACE.md](./docs/CONTRACT_INTERFACE.md)** — Contract API reference
- **[THREAT_MODEL.md](./docs/THREAT_MODEL.md)** — Security threat assessment
- **[ADMIN_ROTATION_PLAYBOOK.md](./docs/ADMIN_ROTATION_PLAYBOOK.md)** — Operational security procedures

## Questions?

- For build issues, check the [Contributing Guidelines](docs/CONTRIBUTING.md)
- For deployment procedures, see [DEPLOYMENT.md](DEPLOYMENT.md)
- For contract usage, see [dongle-smartcontract/README.md](dongle-smartcontract/README.md)
