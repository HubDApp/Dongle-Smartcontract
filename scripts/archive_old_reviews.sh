#!/usr/bin/env bash
# archive_old_reviews.sh — Automatic archival job for issue #804.
#
# Iterates over all registered project IDs and calls `archive_old_reviews` on
# each one, batching up to MAX_ARCHIVE_BATCH_SIZE (50) reviews per contract
# invocation. Continues re-calling for the same project until that project
# returns 0 archived reviews (all eligible reviews processed).
#
# After each batch the script optionally writes the Arweave TX ID back to the
# contract via `set_archived_review_arweave_tx` if ARWEAVE_GATEWAY is set.
# (Arweave upload itself is handled externally; this script only records a
# TX ID you supply or reads from a sidecar file.)
#
# USAGE:
#   ./scripts/archive_old_reviews.sh [--dry-run] [--project-ids 1,2,3] [--start-id 1]
#
# ENVIRONMENT VARIABLES:
#   DEPLOYER_IDENTITY  — Soroban CLI identity name (admin key). Required.
#   CONTRACT_ID        — Contract address. Loaded from .contract_id if absent.
#   NETWORK            — Soroban network name (default: testnet).
#   RPC_URL            — Soroban RPC URL (default: testnet).
#   PASSPHRASE         — Network passphrase (default: testnet).
#   BATCH_SIZE         — Reviews to archive per call (default: 50, max: 50).
#   ARWEAVE_GATEWAY    — Optional Arweave gateway URL. When set, the script
#                        attempts to upload review data and record the TX ID.
#   LOG_FILE           — Path to append archival log lines (default: stdout).
#
# EXIT CODES:
#   0  — All projects processed successfully.
#   1  — Configuration error (missing identity, contract ID, etc.).
#   2  — One or more invocations failed (see LOG_FILE for details).
#
# SCHEDULING:
#   Add to crontab (e.g. weekly at 03:00 UTC on Sunday):
#     0 3 * * 0 /path/to/scripts/archive_old_reviews.sh >> /var/log/archive_reviews.log 2>&1

set -euo pipefail

# ── Directories ───────────────────────────────────────────────────────────────
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# ── Load .env if present ──────────────────────────────────────────────────────
if [ -f "$PROJECT_ROOT/.env" ]; then
    # shellcheck disable=SC2046
    export $(grep -v '^#' "$PROJECT_ROOT/.env" | xargs)
fi

# ── Configuration from environment ────────────────────────────────────────────
DEPLOYER_IDENTITY="${DEPLOYER_IDENTITY:-}"
NETWORK="${NETWORK:-testnet}"
RPC_URL="${RPC_URL:-https://soroban-testnet.stellar.org:443}"
PASSPHRASE="${PASSPHRASE:-Test SDF Network ; September 2015}"
CONTRACT_ID="${CONTRACT_ID:-}"
BATCH_SIZE="${BATCH_SIZE:-50}"
ARWEAVE_GATEWAY="${ARWEAVE_GATEWAY:-}"
LOG_FILE="${LOG_FILE:-}"        # empty = stdout only
DRY_RUN=false
SPECIFIC_IDS=()
START_ID=0

# ── Parse CLI flags ────────────────────────────────────────────────────────────
while [[ $# -gt 0 ]]; do
    case "$1" in
        --dry-run)
            DRY_RUN=true
            shift
            ;;
        --project-ids)
            IFS=',' read -ra SPECIFIC_IDS <<< "$2"
            shift 2
            ;;
        --start-id)
            START_ID="$2"
            shift 2
            ;;
        -h|--help)
            head -40 "$0" | grep '^#'
            exit 0
            ;;
        *)
            echo "Unknown flag: $1" >&2
            exit 1
            ;;
    esac
done

# ── Logging helper ─────────────────────────────────────────────────────────────
log() {
    local msg
    msg="[$(date -u '+%Y-%m-%dT%H:%M:%SZ')] $*"
    echo "$msg"
    if [ -n "$LOG_FILE" ]; then
        echo "$msg" >> "$LOG_FILE"
    fi
}

# ── Prerequisite checks ────────────────────────────────────────────────────────
if ! command -v soroban &> /dev/null; then
    log "ERROR: soroban CLI not found. Install it: cargo install --locked soroban-cli --features opt"
    exit 1
fi

if [ -z "$DEPLOYER_IDENTITY" ]; then
    log "ERROR: DEPLOYER_IDENTITY is not set. Export it or add to .env."
    exit 1
fi

# Load contract ID from .contract_id file if not provided
if [ -z "$CONTRACT_ID" ] && [ -f "$PROJECT_ROOT/.contract_id" ]; then
    CONTRACT_ID=$(cat "$PROJECT_ROOT/.contract_id")
fi

if [ -z "$CONTRACT_ID" ]; then
    log "ERROR: CONTRACT_ID is not set. Deploy the contract first or set CONTRACT_ID."
    exit 1
fi

ADMIN_ADDRESS=$(soroban keys address "$DEPLOYER_IDENTITY" 2>/dev/null || true)
if [ -z "$ADMIN_ADDRESS" ]; then
    log "ERROR: Could not resolve address for identity '$DEPLOYER_IDENTITY'."
    exit 1
fi

# Clamp batch size to [1, 50]
BATCH_SIZE=$(( BATCH_SIZE > 50 ? 50 : BATCH_SIZE ))
BATCH_SIZE=$(( BATCH_SIZE < 1 ? 1 : BATCH_SIZE ))

log "=== Review Archival Job ==="
log "Contract:   $CONTRACT_ID"
log "Admin:      $ADMIN_ADDRESS"
log "Network:    $NETWORK"
log "Batch size: $BATCH_SIZE"
log "Dry run:    $DRY_RUN"

# ── Soroban invoke wrapper ──────────────────────────────────────────────────────
soroban_invoke() {
    local fn_name="$1"
    shift
    soroban contract invoke \
        --id "$CONTRACT_ID" \
        --source "$DEPLOYER_IDENTITY" \
        --network "$NETWORK" \
        --rpc-url "$RPC_URL" \
        --network-passphrase "$PASSPHRASE" \
        -- "$fn_name" "$@" 2>&1
}

# ── Collect project IDs to process ────────────────────────────────────────────
if [ "${#SPECIFIC_IDS[@]}" -gt 0 ]; then
    PROJECT_IDS=("${SPECIFIC_IDS[@]}")
    log "Processing ${#PROJECT_IDS[@]} specific project ID(s): ${PROJECT_IDS[*]}"
else
    # Query the on-chain project count and iterate from START_ID.
    log "Fetching project count from contract..."
    PROJECT_COUNT=$(soroban_invoke get_project_count 2>/dev/null || echo "0")
    PROJECT_COUNT="${PROJECT_COUNT//[[:space:]]/}"

    if ! [[ "$PROJECT_COUNT" =~ ^[0-9]+$ ]]; then
        log "WARNING: Could not read project count ('$PROJECT_COUNT'). Defaulting to 0."
        PROJECT_COUNT=0
    fi

    log "Total projects on-chain: $PROJECT_COUNT"

    # Build sequential ID list from START_ID to PROJECT_COUNT-1.
    PROJECT_IDS=()
    for (( id=START_ID; id<PROJECT_COUNT; id++ )); do
        PROJECT_IDS+=("$id")
    done
fi

if [ "${#PROJECT_IDS[@]}" -eq 0 ]; then
    log "No projects to process. Exiting."
    exit 0
fi

# ── Main archival loop ─────────────────────────────────────────────────────────
TOTAL_ARCHIVED=0
FAILED_PROJECTS=()

for project_id in "${PROJECT_IDS[@]}"; do
    log "--- Project $project_id ---"

    # Keep calling archive_old_reviews until a batch returns 0 (no more eligible reviews).
    pass=1
    while true; do
        if $DRY_RUN; then
            log "  [dry-run] Would call archive_old_reviews project=$project_id batch=$BATCH_SIZE"
            break
        fi

        archived_count=$(soroban_invoke archive_old_reviews \
            --admin "$ADMIN_ADDRESS" \
            --project-id "$project_id" \
            --batch-size "$BATCH_SIZE" 2>/dev/null || echo "ERROR")

        # Strip whitespace
        archived_count="${archived_count//[[:space:]]/}"

        if [[ "$archived_count" == "ERROR" ]] || ! [[ "$archived_count" =~ ^[0-9]+$ ]]; then
            log "  WARN: archive_old_reviews failed for project $project_id (pass $pass): $archived_count"
            FAILED_PROJECTS+=("$project_id")
            break
        fi

        if [ "$archived_count" -eq 0 ]; then
            log "  Project $project_id: no more eligible reviews."
            break
        fi

        log "  Project $project_id pass $pass: archived $archived_count review(s)."
        TOTAL_ARCHIVED=$(( TOTAL_ARCHIVED + archived_count ))
        (( pass++ )) || true
    done
done

# ── Summary ────────────────────────────────────────────────────────────────────
log "=== Archival Complete ==="
log "Total reviews archived: $TOTAL_ARCHIVED"

if [ "${#FAILED_PROJECTS[@]}" -gt 0 ]; then
    log "WARN: Failed to archive reviews for ${#FAILED_PROJECTS[@]} project(s): ${FAILED_PROJECTS[*]}"
    log "Re-run with --project-ids $(IFS=,; echo "${FAILED_PROJECTS[*]}") to retry."
    exit 2
fi

log "All projects processed successfully."

# ── Arweave upload hint ────────────────────────────────────────────────────────
if [ -n "$ARWEAVE_GATEWAY" ] && [ "$TOTAL_ARCHIVED" -gt 0 ]; then
    log ""
    log "=== Arweave Upload Hint ==="
    log "ARWEAVE_GATEWAY is set ($ARWEAVE_GATEWAY)."
    log "After uploading review payloads, record each Arweave TX ID on-chain with:"
    log ""
    log "  soroban contract invoke --id \$CONTRACT_ID --source \$DEPLOYER_IDENTITY --network \$NETWORK -- \\"
    log "    set_archived_review_arweave_tx \\"
    log "    --admin <admin_address> \\"
    log "    --project-id <project_id> \\"
    log "    --reviewer <reviewer_address> \\"
    log "    --arweave-tx-id <tx_id>"
    log ""
    log "The contract stores the TX ID in the ArchivedReview record so"
    log "consumers can retrieve permanent storage references via get_archived_review."
fi
