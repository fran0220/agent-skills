#!/usr/bin/env bash
set -euo pipefail

# Usage:
#   GATEWAY_URL=https://asset.origingame.dev \
#   ADMIN_PASSWORD=... \
#   LLM_PROXY_KEY=... OPENAI_API_KEY=... GOOGLE_API_KEY=... \
#   JIMENG_GATEWAY_KEY=... ELEVENLABS_API_KEY=... TRIPO3D_API_KEY=... \
#   ./scripts/seed-providers.sh
#
# Notes:
# - Admin user must already exist (register once via API).
# - This script logs in to fetch a JWT token, then seeds credentials/providers via HTTP API.

GATEWAY_URL="${GATEWAY_URL:-http://localhost:6700}"
ADMIN_USERNAME="${ADMIN_USERNAME:-admin}"
TOKEN="${TOKEN:-}"

LLM_PROXY_BASE_URL="${LLM_PROXY_BASE_URL:-http://67.230.182.59:8317}"
JIMENG_BASE_URL="${JIMENG_BASE_URL:-http://185.200.65.233:5100}"

require_cmd() {
  local cmd="$1"
  if ! command -v "$cmd" >/dev/null 2>&1; then
    echo "[seed] Missing required command: ${cmd}" >&2
    exit 1
  fi
}

require_var() {
  local name="$1"
  if [[ -z "${!name:-}" ]]; then
    echo "[seed] Missing required env var: ${name}" >&2
    exit 1
  fi
}

extract_token() {
  python3 -c 'import json,sys
try:
    payload = json.load(sys.stdin)
except Exception:
    print("")
    raise SystemExit(0)
print(payload.get("data", {}).get("token", ""))'
}

request_json() {
  local method="$1"
  local path="$2"
  local body="$3"

  local response
  response="$(curl -sS -w $'\n%{http_code}' \
    -X "${method}" "${GATEWAY_URL}${path}" \
    -H "Authorization: Bearer ${TOKEN}" \
    -H "Content-Type: application/json" \
    --data "${body}")"

  local status_code="${response##*$'\n'}"
  local json_body="${response%$'\n'*}"

  if [[ "${status_code}" == "409" ]]; then
    echo "${json_body}"
    return 10
  fi

  if [[ "${status_code}" -ge 400 ]]; then
    echo "[seed] Request failed: ${method} ${path} (HTTP ${status_code})" >&2
    echo "${json_body}" >&2
    return 1
  fi

  local ok
  ok="$(printf '%s' "${json_body}" | python3 -c 'import json,sys
try:
    payload = json.load(sys.stdin)
except Exception:
    print("false")
    raise SystemExit(0)
print("true" if payload.get("ok") is True else "false")')"

  if [[ "${ok}" != "true" ]]; then
    echo "[seed] API returned non-ok payload for ${method} ${path}" >&2
    echo "${json_body}" >&2
    return 1
  fi

  echo "${json_body}"
}

set_credential() {
  local key="$1"
  local value="$2"
  local description="$3"
  local payload
  payload="$(python3 - "$key" "$value" "$description" <<'PY'
import json
import sys

key = sys.argv[1]
value = sys.argv[2]
description = sys.argv[3]

print(json.dumps({
    "key": key,
    "value": value,
    "description": description,
}))
PY
)"

  request_json "PUT" "/api/credentials" "${payload}" >/dev/null
  echo "[seed] Credential set: ${key}"
}

create_provider() {
  local id="$1"
  local payload="$2"

  if request_json "POST" "/api/providers" "${payload}" >/dev/null; then
    echo "[seed] Provider created: ${id}"
    return
  fi

  local rc=$?
  if [[ $rc -eq 10 ]]; then
    echo "[seed] Provider already exists, skipped: ${id}"
    return
  fi
  exit $rc
}

main() {
  require_cmd curl
  require_cmd python3
  require_cmd asset-gateway

  require_var LLM_PROXY_KEY
  require_var OPENAI_API_KEY
  require_var GOOGLE_API_KEY
  require_var JIMENG_GATEWAY_KEY
  require_var ELEVENLABS_API_KEY
  require_var TRIPO3D_API_KEY

  if [[ -z "${TOKEN}" ]]; then
    require_var ADMIN_PASSWORD
    echo "[seed] Logging in as ${ADMIN_USERNAME}"
    local login_json
    login_json="$(asset-gateway --gateway-url "${GATEWAY_URL}" auth login --username "${ADMIN_USERNAME}" --password "${ADMIN_PASSWORD}")"
    TOKEN="$(printf '%s' "${login_json}" | extract_token)"
    if [[ -z "${TOKEN}" ]]; then
      echo "[seed] Failed to read token from auth login response" >&2
      echo "${login_json}" >&2
      exit 1
    fi
  fi

  echo "[seed] Seeding encrypted credentials"
  set_credential "LLM_PROXY_KEY" "${LLM_PROXY_KEY}" "LLM proxy API key"
  set_credential "OPENAI_API_KEY" "${OPENAI_API_KEY}" "OpenAI API key for image generation"
  set_credential "GOOGLE_API_KEY" "${GOOGLE_API_KEY}" "Google API key for Gemini image"
  set_credential "JIMENG_API_KEY" "${JIMENG_GATEWAY_KEY}" "Jimeng gateway API key"
  set_credential "ELEVENLABS_API_KEY" "${ELEVENLABS_API_KEY}" "ElevenLabs API key"
  set_credential "TRIPO3D_API_KEY" "${TRIPO3D_API_KEY}" "Tripo3D API key"

  echo "[seed] Seeding providers"
  create_provider "llm_proxy" "$(cat <<JSON
{
  \"id\": \"llm_proxy\",
  \"display_name\": \"LLM Proxy (Claude/GPT/Gemini/Grok/GLM)\",
  \"adapter\": \"llm_proxy\",
  \"asset_types\": [\"text\"],
  \"config\": {\"base_url\": \"${LLM_PROXY_BASE_URL}\", \"default_model\": \"claude-sonnet-4-6\"},
  \"priority\": 10,
  \"enabled\": true
}
JSON
)"

  create_provider "gpt_image" "$(cat <<JSON
{
  \"id\": \"gpt_image\",
  \"display_name\": \"GPT Image (OpenAI via LLM Proxy)\",
  \"adapter\": \"gpt_image\",
  \"asset_types\": [\"image\"],
  \"config\": {\"base_url\": \"${LLM_PROXY_BASE_URL}\"},
  \"priority\": 5,
  \"enabled\": true
}
JSON
)"

  create_provider "gemini_image" "$(cat <<JSON
{
  \"id\": \"gemini_image\",
  \"display_name\": \"Gemini Flash Image (Google via LLM Proxy)\",
  \"adapter\": \"gemini_image\",
  \"asset_types\": [\"image\"],
  \"config\": {\"base_url\": \"${LLM_PROXY_BASE_URL}\"},
  \"priority\": 8,
  \"enabled\": true
}
JSON
)"

  create_provider "jimeng" "$(cat <<JSON
{
  \"id\": \"jimeng\",
  \"display_name\": \"Jimeng (Image + Seedance Video)\",
  \"adapter\": \"jimeng\",
  \"asset_types\": [\"image\", \"video\"],
  \"config\": {\"base_url\": \"${JIMENG_BASE_URL}\"},
  \"priority\": 3,
  \"enabled\": true
}
JSON
)"

  create_provider "elevenlabs" "$(cat <<JSON
{
  \"id\": \"elevenlabs\",
  \"display_name\": \"ElevenLabs (Audio BGM/SFX)\",
  \"adapter\": \"elevenlabs\",
  \"asset_types\": [\"audio\"],
  \"config\": {},
  \"priority\": 10,
  \"enabled\": true
}
JSON
)"

  create_provider "tripo3d" "$(cat <<JSON
{
  \"id\": \"tripo3d\",
  \"display_name\": \"Tripo3D (Image-to-GLB)\",
  \"adapter\": \"tripo3d\",
  \"asset_types\": [\"model3d\"],
  \"config\": {},
  \"priority\": 10,
  \"enabled\": true
}
JSON
)"

  echo "[seed] Done"
  echo "[seed] Gateway URL: ${GATEWAY_URL}"
}

main "$@"
