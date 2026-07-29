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
