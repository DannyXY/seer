#!/usr/bin/env bash
# Deploy all Seer contracts to Mantle Sepolia.
#
# Required env vars:
#   PRIVATE_KEY      — deployer private key (hex, with or without 0x prefix)
#   BACKEND_SIGNER   — address that signs on behalf of the backend (for SeerIdentitySBT + SeerIntentRegistry)
#   RESOLVER         — address allowed to create and resolve predictions (for SeerPredictionRegistry)
#
# Optional env vars:
#   RPC_URL          — defaults to https://rpc.sepolia.mantle.xyz

set -euo pipefail

RPC_URL="${RPC_URL:-https://rpc.sepolia.mantle.xyz}"
VERIFIER_URL="${VERIFIER_URL:-https://explorer.sepolia.mantle.xyz/api?}"

# ── Input validation ──────────────────────────────────────────────────────────
if [[ -z "${PRIVATE_KEY:-}" ]]; then
  echo "ERROR: PRIVATE_KEY is not set." >&2
  exit 1
fi
if [[ -z "${BACKEND_SIGNER:-}" ]]; then
  echo "ERROR: BACKEND_SIGNER is not set." >&2
  exit 1
fi
if [[ -z "${RESOLVER:-}" ]]; then
  echo "ERROR: RESOLVER is not set." >&2
  exit 1
fi

# Normalise key — forge create expects no leading 0x
PRIVATE_KEY="${PRIVATE_KEY#0x}"

echo "=== Seer contract deployment ==="
echo "RPC:            $RPC_URL"
echo "Backend signer: $BACKEND_SIGNER"
echo "Resolver:       $RESOLVER"
echo ""

# Helper: deploy a contract and return its address.
# Usage: deploy <contract-path>:<ContractName> [constructor args...]
deploy() {
  local contract="$1"; shift
  local output
  output=$(forge create \
    --rpc-url "$RPC_URL" \
    --private-key "$PRIVATE_KEY" \
    --broadcast \
    "$contract" \
    ${@:+"--constructor-args" "$@"} \
    2>&1)

  if ! grep -q "Deployed to:" <<< "$output"; then
    echo "DEPLOY FAILED for $contract:" >&2
    echo "$output" >&2
    exit 1
  fi

  grep "Deployed to:" <<< "$output" | awk '{print $3}'
}

# Helper: verify a deployed contract on Mantle Explorer (Blockscout).
# Usage: verify <address> <contract-path>:<ContractName> [constructor args (hex-encoded)]
verify() {
  local address="$1"
  local contract="$2"
  local constructor_args="${3:-}"
  echo "  Verifying $contract at $address..."
  local args=(
    --rpc-url "$RPC_URL"
    --verifier blockscout
    --verifier-url "$VERIFIER_URL"
    "$address"
    "$contract"
  )
  if [[ -n "$constructor_args" ]]; then
    args+=(--constructor-args "$constructor_args")
  fi
  if forge verify-contract "${args[@]}" 2>&1; then
    echo "  Verified."
  else
    echo "  WARNING: verification failed — you can retry manually later." >&2
  fi
}

# ── 1. SeerArenaPoints ────────────────────────────────────────────────────────
echo "Deploying SeerArenaPoints..."
ARENA_POINTS=$(deploy "contract/SeerArenaPoints.sol:SeerArenaPoints")
echo "  SeerArenaPoints: $ARENA_POINTS"
sleep 5
verify "$ARENA_POINTS" "contract/SeerArenaPoints.sol:SeerArenaPoints"

# ── 2. SeerIdentitySBT ───────────────────────────────────────────────────────
echo "Deploying SeerIdentitySBT..."
IDENTITY_SBT=$(deploy "contract/SeerIdentitySBT.sol:SeerIdentitySBT" "$BACKEND_SIGNER")
echo "  SeerIdentitySBT: $IDENTITY_SBT"
sleep 5
verify "$IDENTITY_SBT" "contract/SeerIdentitySBT.sol:SeerIdentitySBT" \
  "$(cast abi-encode 'constructor(address)' "$BACKEND_SIGNER")"

# ── 3. SeerIntentRegistry ────────────────────────────────────────────────────
echo "Deploying SeerIntentRegistry..."
INTENT_REGISTRY=$(deploy "contract/SeerIntentRegistry.sol:SeerIntentRegistry" "$BACKEND_SIGNER")
echo "  SeerIntentRegistry: $INTENT_REGISTRY"
sleep 5
verify "$INTENT_REGISTRY" "contract/SeerIntentRegistry.sol:SeerIntentRegistry" \
  "$(cast abi-encode 'constructor(address)' "$BACKEND_SIGNER")"

# ── 4. SeerPredictionRegistry ────────────────────────────────────────────────
echo "Deploying SeerPredictionRegistry..."
PREDICTION_REGISTRY=$(deploy "contract/SeerPredictionRegistry.sol:SeerPredictionRegistry" "$ARENA_POINTS" "$RESOLVER")
echo "  SeerPredictionRegistry: $PREDICTION_REGISTRY"
sleep 5
verify "$PREDICTION_REGISTRY" "contract/SeerPredictionRegistry.sol:SeerPredictionRegistry" \
  "$(cast abi-encode 'constructor(address,address)' "$ARENA_POINTS" "$RESOLVER")"

# ── 5. Wire SeerArenaPoints → SeerPredictionRegistry ─────────────────────────
echo "Wiring SeerArenaPoints.setArena($PREDICTION_REGISTRY)..."
cast send \
  --rpc-url "$RPC_URL" \
  --private-key "$PRIVATE_KEY" \
  --confirmations 1 \
  "$ARENA_POINTS" \
  "setArena(address)" \
  "$PREDICTION_REGISTRY"
echo "  Done."

# ── Summary ──────────────────────────────────────────────────────────────────
echo ""
echo "=== Deployment complete ==="
echo "SeerArenaPoints:       $ARENA_POINTS"
echo "SeerIdentitySBT:       $IDENTITY_SBT"
echo "SeerIntentRegistry:    $INTENT_REGISTRY"
echo "SeerPredictionRegistry: $PREDICTION_REGISTRY"
echo ""
echo "Add these to your .env / backend config:"
echo "  ARENA_POINTS_ADDRESS=$ARENA_POINTS"
echo "  IDENTITY_SBT_ADDRESS=$IDENTITY_SBT"
echo "  INTENT_REGISTRY_ADDRESS=$INTENT_REGISTRY"
echo "  PREDICTION_REGISTRY_ADDRESS=$PREDICTION_REGISTRY"
