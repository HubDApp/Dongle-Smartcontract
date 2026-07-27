#!/usr/bin/env bash
set -euo pipefail

# Scripts directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Load environment variables if .env file exists
if [ -f "$PROJECT_ROOT/.env" ]; then
    export $(grep -v '^#' "$PROJECT_ROOT/.env" | xargs)
fi

# Configuration
DEPLOYER_IDENTITY="${DEPLOYER_IDENTITY:-}"
NETWORK="${NETWORK:-testnet}"
CONTRACT_ID="${CONTRACT_ID:-}"

# Load contract ID from file if not provided as env var
if [ -z "$CONTRACT_ID" ] && [ -f "$PROJECT_ROOT/.contract_id" ]; then
    CONTRACT_ID=$(cat "$PROJECT_ROOT/.contract_id")
fi

if [ -z "$CONTRACT_ID" ]; then
    echo "Error: CONTRACT_ID is not set. Please deploy the contract first or set CONTRACT_ID."
    exit 1
fi

if [ -z "$DEPLOYER_IDENTITY" ]; then
    echo "Error: DEPLOYER_IDENTITY is not set. Please set DEPLOYER_IDENTITY."
    exit 1
fi

# Print usage if no arguments
usage() {
    echo "Usage: $0 <command> [args...]"
    echo ""
    echo "Common Invocations:"
    echo "  $0 register <owner_address> <name> <slug> <description> <category>"
    echo "  $0 get_project <project_id>"
    echo "  $0 get_project_by_slug <slug>"
    echo "  $0 set_fee <token_address_or_none> <verification_fee> <registration_fee> <treasury_address>"
    echo "  $0 pay_fee <payer_address> <project_id> <token_address_or_none>"
    echo "  $0 request_verification <project_id> <requester_address> <evidence_cid>"
    echo ""
    exit 1
}

if [ $# -lt 1 ]; then
    usage
fi

CMD="$1"
shift

case "$CMD" in
    register)
        if [ $# -ne 5 ]; then
            echo "Error: register command requires exactly 5 arguments: owner, name, slug, description, category"
            exit 1
        fi
        OWNER="$1"
        NAME="$2"
        SLUG="$3"
        DESC="$4"
        CAT="$5"

        echo "Invoking register_project..."
        soroban contract invoke \
          --id "$CONTRACT_ID" \
          --source "$DEPLOYER_IDENTITY" \
          --network "$NETWORK" \
          -- register_project \
          --params "{\"owner\":\"$OWNER\",\"name\":\"$NAME\",\"slug\":\"$SLUG\",\"description\":\"$DESC\",\"category\":\"$CAT\",\"website\":null,\"logo_cid\":null,\"metadata_cid\":null,\"tags\":null,\"social_links\":null,\"launch_timestamp\":null}"
        ;;

    get_project)
        if [ $# -ne 1 ]; then
            echo "Error: get_project command requires exactly 1 argument: project_id"
            exit 1
        fi
        PROJECT_ID="$1"

        echo "Invoking get_project..."
        soroban contract invoke \
          --id "$CONTRACT_ID" \
          --source "$DEPLOYER_IDENTITY" \
          --network "$NETWORK" \
          -- get_project \
          --project_id "$PROJECT_ID"
        ;;

    get_project_by_slug)
        if [ $# -ne 1 ]; then
            echo "Error: get_project_by_slug command requires exactly 1 argument: slug"
            exit 1
        fi
        SLUG="$1"

        echo "Invoking get_project_by_slug..."
        soroban contract invoke \
          --id "$CONTRACT_ID" \
          --source "$DEPLOYER_IDENTITY" \
          --network "$NETWORK" \
          -- get_project_by_slug \
          --slug "$SLUG"
        ;;

    set_fee)
        if [ $# -ne 4 ]; then
            echo "Error: set_fee command requires exactly 4 arguments: token_address_or_none, verification_fee, registration_fee, treasury_address"
            exit 1
        fi
        TOKEN_ARG="$1"
        VER_FEE="$2"
        REG_FEE="$3"
        TREASURY="$4"

        # Format token address as JSON option
        if [ "$TOKEN_ARG" = "none" ] || [ "$TOKEN_ARG" = "None" ] || [ "$TOKEN_ARG" = "null" ]; then
            TOKEN="null"
        else
            TOKEN="\"$TOKEN_ARG\""
        fi

        echo "Invoking set_fee..."
        soroban contract invoke \
          --id "$CONTRACT_ID" \
          --source "$DEPLOYER_IDENTITY" \
          --network "$NETWORK" \
          -- set_fee \
          --admin "$(soroban keys address "$DEPLOYER_IDENTITY")" \
          --token "$TOKEN" \
          --verification_fee "$VER_FEE" \
          --registration_fee "$REG_FEE" \
          --treasury "$TREASURY"
        ;;

    pay_fee)
        if [ $# -ne 3 ]; then
            echo "Error: pay_fee command requires exactly 3 arguments: payer_address, project_id, token_address_or_none"
            exit 1
        fi
        PAYER="$1"
        PROJECT_ID="$2"
        TOKEN_ARG="$3"

        if [ "$TOKEN_ARG" = "none" ] || [ "$TOKEN_ARG" = "None" ] || [ "$TOKEN_ARG" = "null" ]; then
            TOKEN="null"
        else
            TOKEN="\"$TOKEN_ARG\""
        fi

        echo "Invoking pay_fee..."
        soroban contract invoke \
          --id "$CONTRACT_ID" \
          --source "$DEPLOYER_IDENTITY" \
          --network "$NETWORK" \
          -- pay_fee \
          --payer "$PAYER" \
          --project_id "$PROJECT_ID" \
          --token "$TOKEN"
        ;;

    request_verification)
        if [ $# -ne 3 ]; then
            echo "Error: request_verification command requires exactly 3 arguments: project_id, requester_address, evidence_cid"
            exit 1
        fi
        PROJECT_ID="$1"
        REQUESTER="$2"
        EVIDENCE="$3"

        echo "Invoking request_verification..."
        soroban contract invoke \
          --id "$CONTRACT_ID" \
          --source "$DEPLOYER_IDENTITY" \
          --network "$NETWORK" \
          -- request_verification \
          --project_id "$PROJECT_ID" \
          --requester "$REQUESTER" \
          --evidence_cid "$EVIDENCE"
        ;;

    *)
        echo "Unknown command: $CMD"
        usage
        ;;
esac
