# Deployment Audit Checklist

This checklist ensures all deployment records in `deployments.json` are valid, traceable, and correspond to buildable source code.

---

## Pre-Deployment Audit

Run this before recording a new deployment in `deployments.json`.

### Source Code Verification

- [ ] Current commit builds without errors
  ```bash
  cargo build -p dongle-contract --target wasm32-unknown-unknown --release
  ```
  
- [ ] Current commit passes all tests
  ```bash
  cargo test -p dongle-contract
  ```
  
- [ ] Current commit passes linting
  ```bash
  cargo clippy -p dongle-contract --target wasm32-unknown-unknown -- -D warnings
  ```

- [ ] Git commit hash is recorded
  ```bash
  git rev-parse HEAD  # Record this value
  ```

- [ ] Git tag exists (optional, for releases)
  ```bash
  git describe --tags  # e.g., v0.1.0
  ```

### WASM Build Verification

- [ ] WASM optimized successfully
  ```bash
  soroban contract optimize \
    --wasm target/wasm32-unknown-unknown/release/dongle_contract.wasm \
    --wasm-out target/wasm32-unknown-unknown/release/dongle_contract.optimized.wasm
  ```

- [ ] WASM hash computed correctly
  ```bash
  # macOS/Linux
  shasum -a 256 target/wasm32-unknown-unknown/release/dongle_contract.optimized.wasm
  
  # Windows (PowerShell)
  (Get-FileHash -Algorithm SHA256 target/wasm32-unknown-unknown/release/dongle_contract.optimized.wasm).Hash.ToLower()
  ```
  Record this value (64-char hex string)

### Deployment Verification

- [ ] Contract deployed successfully to target network
  ```bash
  soroban contract deploy \
    --wasm target/wasm32-unknown-unknown/release/dongle_contract.optimized.wasm \
    --source <identity> \
    --network <network>
  ```
  Record contract ID from output

- [ ] Deployed contract exists and is callable on network
  ```bash
  soroban contract inspect --id <contract-id> --network <network>
  ```

- [ ] Deployer address obtained
  ```bash
  soroban keys address <identity>
  ```
  Record address (56 chars starting with G or C)

- [ ] Deployment timestamp recorded (UTC ISO 8601 format)
  Example: `2026-06-24T12:00:00Z`

### Deployment Entry Structure

- [ ] Contract ID format: 56 characters starting with `C`
- [ ] WASM hash format: 64 hexadecimal characters
- [ ] Git commit format: 7-40 hexadecimal characters
- [ ] Git tag format (if provided): Valid semver or tag format
- [ ] Deployer format: 56 characters starting with `G` or `C`
- [ ] Timestamp format: ISO 8601 / RFC 3339 (e.g., `2026-06-24T12:00:00Z`)

---

## Post-Deployment Audit

Run this after adding a new entry to `deployments.json`.

### Manifest Validation

- [ ] JSON syntax is valid
  ```bash
  python3 scripts/validate_deployments.py
  ```
  Must output: `✓ Deployment manifest validation passed successfully!`

- [ ] Required fields present: `contract_id`, `wasm_hash`, `git_commit`, `deployer`, `timestamp`

- [ ] No typos in field names

- [ ] No extra/unknown fields (except optional `git_tag`)

### Network Cross-Check

- [ ] Deployment exists on correct network
  ```bash
  soroban contract inspect --id <contract-id> --network <network>
  ```

- [ ] Multiple deployments of same contract ID are documented with timestamps
  (Warn if same contract ID appears in different commits)

### Git Audit

- [ ] Commit hash exists in repository
  ```bash
  git log --oneline | grep <commit-hash>
  ```

- [ ] Commit is reachable from main branch
  ```bash
  git merge-base --is-ancestor <commit-hash> main && echo "Ancestor of main"
  ```

- [ ] If `git_tag` is provided, tag exists and references the commit
  ```bash
  git tag -l | grep <tag-name>
  git rev-list -n 1 <tag-name> | grep <commit-hash>
  ```

- [ ] Commit message and context align with deployment
  ```bash
  git show <commit-hash> --stat
  ```

### Documentation

- [ ] Entry is appended (not inserted) to the network array
  (Maintains chronological order)

- [ ] No duplicate entries for same contract ID + network + timestamp

- [ ] If fixing a broken entry, old entry is documented with reason
  (e.g., comment explaining why entry was corrected)

---

## Audit Failure Resolution

If validation fails, follow these steps:

### Invalid Format

- **Issue:** JSON validation fails
- **Resolution:**
  1. Check field formats against schema in `deployments.schema.json`
  2. Use validator error messages to identify field
  3. Correct format and re-run `python3 scripts/validate_deployments.py`

### Commit Not Found

- **Issue:** Git commit hash doesn't exist or is typo
- **Resolution:**
  1. Verify correct commit hash: `git rev-parse HEAD`
  2. Correct typo in deployments.json
  3. Ensure commit is pushed to repository

### Network Verification Failed

- **Issue:** `soroban contract inspect` reports contract not found
- **Resolution:**
  1. Verify contract ID is correct (not copy-paste error)
  2. Verify network is correct (testnet vs mainnet)
  3. Check network connectivity
  4. If contract was purged or network reset, document with timestamp

### Source Code Doesn't Build

- **Issue:** Specified commit cannot compile
- **Resolution:**
  1. **DO NOT DEPLOY** from broken source
  2. Identify fix commit that restores buildability
  3. Rebuild WASM from fixed commit
  4. Record fixed deployment with new commit hash
  5. Update deployments.json with corrected entry

---

## Periodic Audit (Monthly)

For each existing deployment entry:

- [ ] Contract still exists on network
  ```bash
  soroban contract inspect --id <contract-id> --network <network>
  ```

- [ ] Commit still exists in repository
  ```bash
  git log --oneline | grep <commit-hash>
  ```

- [ ] WASM is reproducible from commit (spot check)
  ```bash
  git checkout <commit-hash>
  cargo build -p dongle-contract --target wasm32-unknown-unknown --release
  soroban contract optimize --wasm target/wasm32-unknown-unknown/release/dongle_contract.wasm
  # Compare hash with recorded wasm_hash
  ```

- [ ] No newer fixes to same contract on same network
  (If yes, document why older version remains active)

---

## Example: Valid Deployment Entry

```json
{
  "contract_id": "CCWUXOTO2RJK5QRPNDP2K2OTW247V7OVDJ3Z4R3V47N4TZOQCXJ42N73",
  "wasm_hash": "a4b5c6d7e8f901a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d7e8f901a2b3c4d5e6f7",
  "git_commit": "c9975ea",
  "git_tag": "v0.1.0",
  "deployer": "GDAMR3SOTO2RJK5QRPNDP2K2OTW247V7OVDJ3Z4R3V47N4TZOQCXJ42N",
  "timestamp": "2026-06-24T12:00:00Z"
}
```

**Verification:**
- Schema check: ✓ All required fields present, correct formats
- Git check: ✓ `c9975ea` exists, has tag `v0.1.0`
- Network check: ✓ Contract exists on testnet
- Build check: ✓ Commit can compile, WASM hash matches
