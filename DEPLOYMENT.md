# Dongle Smart Contract Deployment Documentation

This repository uses a structured deployment manifest, [`deployments.json`](file:///home/chidubem/ProjectFolder/DripProjects/Dongle-Smartcontract/deployments.json), to track and record smart contract deployments to Stellar networks (such as Testnet and Mainnet).

---

## Manifest Location & Schema

- **Manifest File:** [`deployments.json`](file:///home/chidubem/ProjectFolder/DripProjects/Dongle-Smartcontract/deployments.json)
- **JSON Schema:** [`deployments.schema.json`](file:///home/chidubem/ProjectFolder/DripProjects/Dongle-Smartcontract/deployments.schema.json)

The manifest is organized by network environment:
- `testnet`: Array of deployments on Stellar Testnet.
- `mainnet`: Array of deployments on Stellar Mainnet.

### Fields per Deployment Entry

Every entry in the deployment lists must contain:

| Field Name | Type | Description | Format / Pattern | Required |
|---|---|---|---|---|
| `contract_id` | String | The deployed contract address on Stellar | 56 alphanumeric characters starting with `C` | Yes |
| `wasm_hash` | String | Hexadecimal hash of the built contract WASM file | 64 hexadecimal characters | Yes |
| `git_commit` | String | Git commit hash (SHA-1 or short form) that produced this WASM | 7-40 hex characters | Yes |
| `git_tag` | String | Optional semantic version tag (e.g., `v0.1.0`) for release tracking | Alphanumeric, dots, underscores, hyphens | No |
| `deployer` | String | The public key / contract address that performed the deploy | 56 alphanumeric characters starting with `G` or `C` | Yes |
| `timestamp` | String | The date and time the deployment was executed | ISO 8601 / RFC 3339 format (e.g., `YYYY-MM-DDTHH:MM:SSZ`) | Yes |

---

## How to Update the Manifest After Deploys

Whenever you deploy a new version of the contract or initialize a new instance, update `deployments.json` by adding a new record to the corresponding network array.

### Automated Deployment (Recommended)

For automated deployment with built-in manifest update capability, use the deployment script:

```bash
# Set deployer identity
export DEPLOYER_IDENTITY=my-key-name
export NETWORK=testnet

# Run deployment script
bash scripts/deploy_testnet.sh
```

The script performs:
1. Network configuration and identity setup
2. Contract build (cargo + soroban)
3. WASM optimization (soroban contract optimize)
4. Contract deployment
5. Outputs: Contract ID and WASM hash for manifest

### Manual Deployment Steps

#### 1. Build and Optimize Contract

```bash
# Navigate to contract directory
cd dongle-smartcontract

# Build WASM release
cargo build --target wasm32-unknown-unknown --release

# Optimize WASM using soroban CLI
soroban contract optimize \
  --wasm target/wasm32-unknown-unknown/release/dongle_contract.wasm \
  --wasm-out target/wasm32-unknown-unknown/release/dongle_contract.optimized.wasm
```

**Note:** CI pipeline uses `stellar contract optimize` (with wasm-opt features). Both `soroban` and `stellar` commands produce equivalent optimized WASM.

#### 2. Deploy Contract

```bash
soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/dongle_contract.optimized.wasm \
  --source my-identity \
  --network testnet
```

**Output:** Note the `Contract ID` from the deployment output.

#### 3. Retrieve Deployment Information

Extract the following information:

- **Contract ID:** From soroban deploy output or via:
  ```bash
  soroban contract inspect --id <contract-id> --network testnet
  ```

- **WASM Hash:** Compute SHA-256 of the optimized WASM:
  ```bash
  # macOS/Linux
  shasum -a 256 target/wasm32-unknown-unknown/release/dongle_contract.optimized.wasm
  
  # Windows (PowerShell)
  (Get-FileHash -Algorithm SHA256 target/wasm32-unknown-unknown/release/dongle_contract.optimized.wasm).Hash.ToLower()
  ```

- **Git Commit:** Get current commit hash:
  ```bash
  git rev-parse HEAD           # Full SHA-1 (40 chars)
  git rev-parse --short HEAD   # Short hash (7 chars)
  ```

- **Git Tag (optional):** If this deployment corresponds to a release:
  ```bash
  git describe --tags          # e.g., v0.1.0
  ```

- **Deployer Account:** Your Soroban key public address:
  ```bash
  soroban keys address my-identity
  ```

- **Timestamp:** Current UTC time in ISO 8601 format (e.g., `2026-06-24T12:00:00Z`)

#### 4. Edit `deployments.json`

Append the entry to the end of the appropriate network array:

**Example:**
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

**Required fields:** `contract_id`, `wasm_hash`, `git_commit`, `deployer`, `timestamp`  
**Optional fields:** `git_tag`

#### 5. Validate the Changes Locally

Always run the validation script locally to ensure there are no syntax errors or invalid formats:

```bash
python3 scripts/validate_deployments.py
```

Ensure it exits with `✓ Deployment manifest validation passed successfully!`.

---

## Verifying Deployment on Network

After adding an entry to `deployments.json`, verify the contract exists on the network:

```bash
soroban contract inspect \
  --id CCWUXOTO2RJK5QRPNDP2K2OTW247V7OVDJ3Z4R3V47N4TZOQCXJ42N73 \
  --network testnet
```

---

## Post-Deploy Governance Operations

After initialize, privileged admin changes (adding/removing admins, raising the approval threshold, fee updates, verification decisions) can be executed directly **only while** `get_admin_approval_threshold` is `1`. Once the threshold is greater than `1`, those mutations must go through the multi-sig proposal workflow: `create_proposal` → `approve_proposal` → `execute_proposal`.

Use [`scripts/invoke.sh`](scripts/invoke.sh). Each call is signed by `DEPLOYER_IDENTITY` (the admin whose key is used for that step). Export `CONTRACT_ID` or keep `.contract_id` from deploy.

### 1. Bootstrap additional admins (threshold still 1)

```bash
export NETWORK=testnet
export DEPLOYER_IDENTITY=alice          # current admin identity

# Add co-admins while single-admin mode still allows direct add_admin
./scripts/invoke.sh add_admin "$(soroban keys address bob)"
./scripts/invoke.sh add_admin "$(soroban keys address carol)"

# Require two approvals before a proposal can execute
./scripts/invoke.sh set_admin_approval_threshold 2
./scripts/invoke.sh get_admin_approval_threshold
```

Direct `add_admin` / `remove_admin` now returns `Unauthorized`. Use proposals instead.

### 2. Create a proposal

Example: propose adding a fourth admin. The proposer is counted as the first approval.

```bash
export DEPLOYER_IDENTITY=alice
./scripts/invoke.sh create_proposal add_admin "$(soroban keys address dave)"
```

The invoke prints the new `proposal_id` (for example `1`). Inspect it:

```bash
./scripts/invoke.sh get_proposal 1
```

Other payloads supported by the helper:

```bash
./scripts/invoke.sh create_proposal remove_admin <admin_address>
./scripts/invoke.sh create_proposal set_threshold 3
./scripts/invoke.sh create_proposal set_fee none 1000 500 <treasury_address>
./scripts/invoke.sh create_proposal approve_verification 1
./scripts/invoke.sh create_proposal reject_verification 1
./scripts/invoke.sh create_proposal revoke_verification 1 "policy violation"
```

### 3. Collect approvals

A second distinct admin must approve until `approvals.len() >= threshold`. The original proposer cannot approve twice.

```bash
export DEPLOYER_IDENTITY=bob
./scripts/invoke.sh approve_proposal 1

./scripts/invoke.sh get_proposal 1
# status should be Approved when the threshold is met
```

### 4. Execute the proposal

Any admin (including one who has not approved) can execute once the threshold is met. Execution applies the payload (here, adding `dave` as admin) and marks the proposal `Executed`.

```bash
export DEPLOYER_IDENTITY=carol
./scripts/invoke.sh execute_proposal 1

./scripts/invoke.sh get_proposal 1
./scripts/invoke.sh is_admin "$(soroban keys address dave)"
```

Re-executing an already executed proposal fails with `InvalidStatus`.

### Threshold 1 shortcut

If the threshold is still `1`, `create_proposal` is auto-approved (the proposer's vote meets the threshold). Skip `approve_proposal` and call `execute_proposal` immediately.

For operational key rotation with this flow, see [docs/ADMIN_ROTATION_PLAYBOOK.md](docs/ADMIN_ROTATION_PLAYBOOK.md).

---

## CI/CD Validation

To prevent invalid, broken, or undocumented deployments from entering the `main` branch, the CI/CD pipeline runs `scripts/validate_deployments.py` on every push and pull request. If the script fails, the CI check will fail, blocking merges.

### CI Pipeline Summary

1. **validate-manifest:** Checks JSON schema compliance and format
2. **fmt:** Rust code formatting
3. **clippy:** Linting with wasm32 target
4. **test:** Full test suite
5. **build:** WASM release build
6. **optimize:** WASM optimization and artifact upload (GitHub Actions)

The optimize job uses `stellar contract optimize` (with wasm-opt) and uploads the optimized artifact to GitHub Actions. WASM hash computation is not automated in CI; extract it manually from build artifacts or compute locally.
