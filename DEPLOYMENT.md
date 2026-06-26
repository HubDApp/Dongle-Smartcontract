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

| Field Name | Type | Description | Format / Pattern |
|---|---|---|---|
| `contract_id` | String | The deployed contract address on Stellar | 56 alphanumeric characters starting with `C` |
| `wasm_hash` | String | Hexadecimal hash of the built contract WASM file | 64 hexadecimal characters |
| `deployer` | String | The public key / contract address that performed the deploy | 56 alphanumeric characters starting with `G` or `C` |
| `timestamp` | String | The date and time the deployment was executed | ISO 8601 / RFC 3339 format (e.g., `YYYY-MM-DDTHH:MM:SSZ`) |

---

## How to Update the Manifest After Deploys

Whenever you deploy a new version of the contract or initialize a new instance, update `deployments.json` by adding a new record to the corresponding network array.

### Step-by-Step Guide

#### 1. Retrieve Deployment Information

After executing the deployment command, gather the necessary info:

- **Contract ID:** Printed by the Soroban CLI output when running `soroban contract deploy`.
- **WASM Hash:** Can be retrieved using the Soroban CLI by printing or checking the hash of the built `.wasm` file:
  ```bash
  soroban contract install --wasm target/wasm32-unknown-unknown/release/dongle_contract.wasm --network <network> --source <identity>
  ```
  *(Note: This returns the WASM hash which is required prior to deployment. You can also get it from build output / target hash.)*
- **Deployer Account:** The Stellar public key corresponding to the source identity used for deployment (e.g., Alice's key: `GDAMR...`).
- **Timestamp:** The UTC timestamp of the deployment.

#### 2. Edit `deployments.json`

Append the entry to the end of the list for the appropriate network in `deployments.json`.

**Example:**
```json
{
  "contract_id": "CCWUXOTO2RJK5QRPNDP2K2OTW247V7OVDJ3Z4R3V47N4TZOQCXJ42N73",
  "wasm_hash": "a4b5c6d7e8f901a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d7e8f901a2b3c4d5e6f7",
  "deployer": "GDAMR3SOTO2RJK5QRPNDP2K2OTW247V7OVDJ3Z4R3V47N4TZOQCXJ42N",
  "timestamp": "2026-06-24T12:00:00Z"
}
```

#### 3. Validate the Changes Locally

Always run the validation script locally to ensure there are no syntax errors or invalid formats:

```bash
python3 scripts/validate_deployments.py
```

Ensure it exits with `✓ Deployment manifest validation passed successfully!`.

---

## CI/CD Validation

To prevent invalid, broken, or undocumented deployments from entering the `main` branch, the CI/CD pipeline runs `scripts/validate_deployments.py` on every push and pull request. If the script fails, the CI check will fail, blocking merges.
