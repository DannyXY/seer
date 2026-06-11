#!/usr/bin/env bash
set -euo pipefail

BASE_URL="${SEER_API_URL:-http://localhost:10000}"
REQUIRE_SAFE_READY="${REQUIRE_SAFE_READY:-0}"
REQUIRE_PROTOCOL_READY="${REQUIRE_PROTOCOL_READY:-${REQUIRE_LENDLE_READY:-0}}"
RUN_LENDLE_EVAL="${RUN_LENDLE_EVAL:-0}"

require_command() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "missing required command: $1" >&2
    exit 1
  fi
}

fetch_json() {
  local path="$1"
  local output="$2"
  curl -fsS "${BASE_URL}${path}" -o "$output"
}

json_value() {
  local file="$1"
  local query="$2"
  if command -v jq >/dev/null 2>&1; then
    jq -r "$query" "$file"
  else
    echo ""
  fi
}

require_ready() {
  local file="$1"
  local path="$2"
  local label="$3"
  local required="$4"
  if [ "$required" != "1" ]; then
    return 0
  fi
  if ! command -v jq >/dev/null 2>&1; then
    echo "jq is required when ${label} readiness is enforced" >&2
    exit 1
  fi
  local ready
  ready="$(jq -r "${path}.ready" "$file")"
  if [ "$ready" != "true" ]; then
    echo "${label} readiness failed; missing:" >&2
    jq "${path}.missing" "$file" >&2
    exit 1
  fi
}

maybe_run_lendle_eval() {
  if [ "$RUN_LENDLE_EVAL" != "1" ]; then
    return 0
  fi
  if [ -z "${SEER_AUTH_TOKEN:-}" ] || [ -z "${SEER_WALLET_ADDRESS:-}" ]; then
    echo "RUN_LENDLE_EVAL=1 requires SEER_AUTH_TOKEN and SEER_WALLET_ADDRESS" >&2
    exit 1
  fi
  local owner="${SEER_OWNER_ADDRESS:-$SEER_WALLET_ADDRESS}"
  local payload
  payload="$(mktemp)"
  cat >"$payload" <<JSON
{
  "wallet_address": "${SEER_WALLET_ADDRESS}",
  "raw_intent": "Supply 1 USDC into Lendle now",
  "owner_address": "${owner}"
}
JSON
  echo "== Lendle allowance/simulation preview =="
  curl -fsS \
    -X POST "${BASE_URL}/api/agent/evaluate-intent-with-allowance" \
    -H "authorization: Bearer ${SEER_AUTH_TOKEN}" \
    -H "content-type: application/json" \
    --data-binary "@${payload}"
  echo
  rm -f "$payload"
}

require_command curl

readiness="$(mktemp)"
execution_readiness="$(mktemp)"
trap 'rm -f "$readiness" "$execution_readiness"' EXIT

echo "== Health =="
fetch_json "/api/health" /dev/stdout
echo

echo "== Contract Readiness =="
fetch_json "/api/contracts/readiness" "$readiness"
if command -v jq >/dev/null 2>&1; then
  jq '{expected_chain_id, observed_chain_id, ready_for_user_operation_relay, live_validation}' "$readiness"
else
  cat "$readiness"
  echo
fi

require_ready "$readiness" ".live_validation.safe_user_operation" "Safe user operation" "$REQUIRE_SAFE_READY"
require_ready "$readiness" ".live_validation.protocol_swaps" "Protocol execution" "$REQUIRE_PROTOCOL_READY"

echo "== Execution Destination Readiness =="
fetch_json "/api/contracts/execution-readiness" "$execution_readiness"
if command -v jq >/dev/null 2>&1; then
  jq '{chain_id, configured_token_symbols, protocols}' "$execution_readiness"
else
  cat "$execution_readiness"
  echo
fi

maybe_run_lendle_eval

echo "live validation smoke completed"
