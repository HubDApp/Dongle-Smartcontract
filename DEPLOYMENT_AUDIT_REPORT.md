# Deployment Audit Report: WASM Build Integrity & Documentation Reconciliation

**Date:** July 24, 2026  
**Branch:** `audit/deployment-wasm-integrity`  
**Status:** AUDIT IN PROGRESS

---

## Executive Summary

This audit validates deployment entries in `deployments.json` against buildable source commits and reconciles deployment documentation with current build/deploy scripts and CI pipeline. **Critical Finding:** The current main branch (commit c9975ea) **cannot build** due to a syntax error in `dongle-smartcontract/src/utils.rs`. The single testnet deployment entry must be traced to identify if it was built from a working commit or from a broken state.

---

## Part 1: Deployment Entries Audit

### Current Deployment Records

**File:** `deployments.json`

```json
{
  "testnet": [
    {
      "contract_id": "CCWUXOTO2RJK5QRPNDP2K2OTW247V7OVDJ3Z4R3V47N4TZOQCXJ42N73",
      "wasm_hash": "a4b5c6d7e8f901a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d7e8f901a2b3c4d5e6f7",
      "deployer": "GDAMR3SOTO2RJK5QRPNDP2K2OTW247V7OVDJ3Z4R3V47N4TZOQCXJ42N",
      "timestamp": "2026-06-24T12:00:00Z"
    }
  ],
  "mainnet": []
}
```

### Audit Findings

#### 1. Testnet Deployment Entry

| Field | Value | Status |
|-------|-------|--------|
| `contract_id` | `CCWUXOTO2RJK5QRPNDP2K2OTW247V7OVDJ3Z4R3V47N4TZOQCXJ42N73` | ✓ Valid format (56 chars, starts with C) |
| `wasm_hash` | `a4b5c6d7e8f901a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d7e8f901a2b3c4d5e6f7` | ✓ Valid hex (64 chars) |
| `deployer` | `GDAMR3SOTO2RJK5QRPNDP2K2OTW247V7OVDJ3Z4R3V47N4TZOQCXJ42N` | ✓ Valid format (56 chars, starts with G) |
| `timestamp` | `2026-06-24T12:00:00Z` | ✓ Valid ISO 8601 format |

**Schema Validation:** ✓ **PASS** — Entry passes all schema requirements as defined in `deployments.schema.json`

#### 2. Missing Build Provenance

**⚠ CRITICAL ISSUE:** The deployment entry **lacks Git commit/tag reference**. Unable to verify which source commit produced this WASM.

**Observations:**
- Deployment timestamp: `2026-06-24T12:00:00Z` (June 24, 2026, 12:00 UTC)
- Git commits around that time (in chronological order):
  - `74fe63d` (2026-06-24 14:28:56) — "Merge pull request #268"
  - `e069ec8` (2026-06-24 13:08:24) — "feat: implement structured deployment manifest tracking with automated CI validation"
  - Earlier commits: 2026-06-24 13:08:19, 11:30:21, 10:36:34, 09:54:59, 09:31:08, 06:33:41, 06:26:44

**Note:** Git log shows deployment-related commits *after* the timestamp (13:08 UTC vs 12:00 UTC), suggesting the deployment may have been manual or CI-generated.

#### 3. Current Build Status

**Current HEAD (c9975ea)** — `2026-06-28 09:37:48` — **CANNOT BUILD**

```
error: this file contains an unclosed delimiter
   --> dongle-smartcontract\src\utils.rs:365:3
```

**Root Cause:** File `dongle-smartcontract/src/utils.rs` has malformed impl block structure. A free function `is_maintainer()` was inserted before the first `impl Utils` block closes, creating unclosed delimiter error.

**Implication:** The testnet deployment hash cannot be re-verified from current source. No WASM artifact in the repository to validate hash match.

---

## Part 2: Deployment Instructions vs. Implementation

### 2.1 DEPLOYMENT.md Analysis

**File:** `DEPLOYMENT.md`

#### Documented Steps:

1. **Retrieve Deployment Info**
   - Contract ID from `soroban contract deploy` output
   - WASM hash via `soroban contract install` or build output
   - Deployer account public key
   - UTC timestamp

2. **Edit deployments.json**
   - Append entry to appropriate network array
   - Example format provided

3. **Validate Locally**
   - Run `python3 scripts/validate_deployments.py`

#### Status: ⚠ **INCOMPLETE**

**Missing Information:**
- No mention of how to obtain WASM hash after deployment (only mentions pre-deployment retrieval)
- No reference to `scripts/deploy_testnet.sh` — a complete automated script exists but is not mentioned
- No reference to CI automation — CI runs deployments validation automatically
- No guidance on git commit tracking for audit trail

---

### 2.2 scripts/deploy_testnet.sh vs DEPLOYMENT.md

**File:** `scripts/deploy_testnet.sh`

#### Script Capabilities:

```bash
# Automated workflow:
1. Load .env file for DEPLOYER_IDENTITY, NETWORK, RPC_URL, PASSPHRASE
2. Configure soroban network
3. Generate/fund deployer identity if needed
4. Build: cargo build + soroban contract build
5. Optimize: soroban contract optimize
6. Deploy: soroban contract deploy
7. Output: Contract ID saved to .contract_id file
```

#### Issues:

1. **DEPLOYMENT.md doesn't mention this script** — User must manually perform all steps
2. **Script doesn't automatically update deployments.json** — Manual step still required
3. **Script doesn't compute WASM hash** — Hash must be retrieved separately
4. **No Git integration** — Commit/tag not recorded with deployment

#### Recommendation:

Update `DEPLOYMENT.md` Step 1 to reference the automated script:

```markdown
For automated deployment, run:
  bash scripts/deploy_testnet.sh

This script handles build, optimization, and deployment. 
After successful deployment, retrieve the WASM hash and follow Step 2.
```

---

### 2.3 CI Optimize Job vs DEPLOYMENT.md

**File:** `.github/workflows/ci.yml` — `optimize` job

#### CI Pipeline Flow:

```yaml
Jobs: validate-manifest → fmt → clippy → test → build → optimize

optimize job:
  - Installs: cargo install --locked stellar-cli --features opt
  - Builds: cargo build (again)
  - Runs: bash scripts/optimize_wasm.sh
  - Output: Uploads dongle_contract_optimized.wasm as artifact
```

#### scripts/optimize_wasm.sh Details:

```bash
Input:  target/wasm32-unknown-unknown/release/dongle_contract.wasm
Output: target/wasm32-unknown-unknown/release/dongle_contract.optimized.wasm

Computes: File size reduction and reports metrics
Uses: stellar contract optimize --wasm <input> --wasm-out <output>
```

#### Issues:

1. **DEPLOYMENT.md doesn't mention CI artifacts** — No guidance on using optimized WASM from CI
2. **DEPLOYMENT.md uses `soroban` CLI only** — CI uses `stellar` CLI with `--features opt`
3. **Mismatch in optimization approach:**
   - DEPLOYMENT.md: "Optimize contract WASM..." (vague)
   - scripts/deploy_testnet.sh: `soroban contract optimize`
   - CI: `stellar contract optimize` with explicit output flag
4. **No mention of wasm hash computation in CI** — Artifact uploaded but hash not extracted

#### Recommendation:

Update `DEPLOYMENT.md` to clarify optimization:

```markdown
Optimization is performed during:
- Manual deployment: soroban contract optimize
- CI pipeline: stellar contract optimize (with wasm-opt features)

Both produce equivalent optimized WASM. Hash must be computed separately.
```

---

## Part 3: Validation Script Audit

**File:** `scripts/validate_deployments.py`

### Validation Coverage

| Check | Status | Comment |
|-------|--------|---------|
| JSON structure | ✓ Implements | Validates root is object, required keys present |
| Field formats | ✓ Implements | Regex patterns for contract_id, wasm_hash, deployer, timestamp |
| Array structure | ✓ Implements | Validates testnet/mainnet are arrays |
| No extra fields | ✓ Implements | Rejects unknown properties |
| **Git commit tracking** | ✗ Missing | No validation of commit/tag references |
| **WASM hash verification** | ✗ Missing | Doesn't validate hash against actual build |
| **Deployer account validation** | ✗ Missing | No check against Stellar network |
| **Contract existence** | ✗ Missing | No Soroban network verification |

### Limitations

Current validator is **schema-only** — verifies format, not authenticity. Cannot confirm:
- Whether deployed contract actually exists on network
- Whether WASM hash corresponds to any built artifact
- Whether deployer account is valid
- Whether deployment happened before/after timestamp

**This is acceptable** — Validator is designed as format gate, not integrity gate. Full verification requires Soroban/Stellar network access.

---

## Part 4: Recommendations

### Immediate Actions (Blocking)

1. **Fix utils.rs syntax error** — Resolve unclosed delimiter to restore build capability
2. **Verify testnet deployment** — Confirm deployment exists on Stellar testnet:
   ```bash
   soroban contract inspect \
     --id CCWUXOTO2RJK5QRPNDP2K2OTW247V7OVDJ3Z4R3V47N4TZOQCXJ42N73 \
     --network testnet
   ```

### Short Term (Audit Trail)

3. **Add commit tracking to deployments.json schema:**
   ```json
   {
     "contract_id": "...",
     "wasm_hash": "...",
     "git_commit": "c9975ea",          // NEW
     "git_tag": "v0.1.0",               // NEW (optional)
     "deployer": "...",
     "timestamp": "..."
   }
   ```

4. **Update DEPLOYMENT.md:**
   - Add `git_commit` field capture instructions
   - Reference `scripts/deploy_testnet.sh` as primary method
   - Clarify optimization differences between manual and CI
   - Add Soroban inspect command for verification

5. **Enhance scripts/deploy_testnet.sh:**
   - Capture and print WASM hash after optimization
   - Auto-append deployment entry to deployments.json
   - Record git commit/tag in entry
   - Run validation before commit

### Long Term (CI Integration)

6. **Extend CI optimize job:**
   - Extract WASM hash after optimization
   - Create deployment.json entry template
   - Generate audit trail artifact

7. **Create deployment verification job:**
   - Run `soroban contract inspect` on network
   - Confirm contract exists at expected address
   - Validate WASM hash against artifact

---

## Summary Table: Documentation vs Implementation

| Component | DEPLOYMENT.md | deploy_testnet.sh | CI optimize | Status |
|-----------|---------------|-------------------|-------------|--------|
| Build | Mentioned vaguely | ✓ Implemented | ✓ Implemented | ⚠ Vague |
| Optimize | Mentioned vaguely | ✓ `soroban` CLI | ✓ `stellar` CLI | ⚠ Inconsistent |
| Deploy | Described step 1 | ✓ Implemented | Not in optimize job | ⚠ Incomplete |
| WASM hash retrieval | Described (pre-deploy) | Not automated | Not provided | ✗ Problematic |
| deployments.json update | Described manual | Not automated | Not implemented | ✗ Manual only |
| Git tracking | Not mentioned | Not implemented | Not implemented | ✗ Missing |
| Validation | Mentioned | ✓ Runs script | ✓ Runs script | ✓ Complete |

---

## Next Steps

Await approval to:
1. Fix utils.rs build error
2. Verify testnet deployment exists
3. Update deployments.json schema to include git_commit
4. Update DEPLOYMENT.md with complete instruction set
5. Enhance scripts/deploy_testnet.sh with automation
