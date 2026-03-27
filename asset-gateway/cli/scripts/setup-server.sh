#!/usr/bin/env bash
set -euo pipefail

# Usage (run on Oracle host): ./scripts/setup-server.sh
# 1) Prepare /opt/asset-gateway directories
# 2) Run PostgreSQL in Docker
# 3) Generate and write env file (if missing keys)
# 4) Install and enable systemd service

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

INSTALL_DIR="${INSTALL_DIR:-/opt/asset-gateway}"
DATA_DIR="${INSTALL_DIR}/data"
PGDATA_DIR="${INSTALL_DIR}/pgdata"
ENV_FILE="${INSTALL_DIR}/.env"
PG_PASSWORD_FILE="${INSTALL_DIR}/.pg_password"

PG_CONTAINER="${PG_CONTAINER:-asset-gw-postgres}"
PG_USER="${PG_USER:-assetgw}"
PG_DB="${PG_DB:-asset_gateway}"
PG_PORT="${PG_PORT:-5432}"

SERVICE_NAME="${SERVICE_NAME:-asset-gateway}"
SERVICE_FILE="/etc/systemd/system/${SERVICE_NAME}.service"
SERVICE_SOURCE="${SCRIPT_DIR}/asset-gateway.service"

require_cmd() {
  local cmd="$1"
  if ! command -v "$cmd" >/dev/null 2>&1; then
    echo "[setup] Missing required command: ${cmd}" >&2
    exit 1
  fi
}

ensure_env_key() {
  local key="$1"
  local value="$2"
  if grep -q "^${key}=" "${ENV_FILE}" 2>/dev/null; then
    return
  fi
  printf '%s=%s\n' "$key" "$value" >>"${ENV_FILE}"
}

main() {
  require_cmd sudo
  require_cmd docker
  require_cmd openssl

  echo "[setup] Preparing directories under ${INSTALL_DIR}"
  sudo mkdir -p "${INSTALL_DIR}" "${DATA_DIR}" "${PGDATA_DIR}"
  sudo chown -R "$(id -un):$(id -gn)" "${INSTALL_DIR}"

  if [[ ! -f "${PG_PASSWORD_FILE}" ]]; then
    openssl rand -hex 18 >"${PG_PASSWORD_FILE}"
    chmod 600 "${PG_PASSWORD_FILE}"
  fi
  local pg_password
  pg_password="$(cat "${PG_PASSWORD_FILE}")"

  echo "[setup] Ensuring docker service is active"
  sudo systemctl enable --now docker

  if ! sudo docker ps -a --format '{{.Names}}' | grep -q "^${PG_CONTAINER}$"; then
    echo "[setup] Creating PostgreSQL container: ${PG_CONTAINER}"
    sudo docker run -d \
      --name "${PG_CONTAINER}" \
      -e "POSTGRES_USER=${PG_USER}" \
      -e "POSTGRES_PASSWORD=${pg_password}" \
      -e "POSTGRES_DB=${PG_DB}" \
      -p "127.0.0.1:${PG_PORT}:5432" \
      -v "${PGDATA_DIR}:/var/lib/postgresql/data" \
      --restart unless-stopped \
      postgres:16-alpine >/dev/null
  else
    echo "[setup] PostgreSQL container already exists, starting it"
    sudo docker start "${PG_CONTAINER}" >/dev/null || true
  fi

  echo "[setup] Waiting for PostgreSQL readiness"
  local retries=0
  until sudo docker exec "${PG_CONTAINER}" pg_isready -U "${PG_USER}" -d "${PG_DB}" >/dev/null 2>&1; do
    retries=$((retries + 1))
    if [[ ${retries} -gt 30 ]]; then
      echo "[setup] PostgreSQL did not become ready in time" >&2
      exit 1
    fi
    sleep 1
  done

  if [[ ! -f "${ENV_FILE}" ]]; then
    install -m 600 /dev/null "${ENV_FILE}"
  fi

  local jwt_secret
  local vault_key
  jwt_secret="$(openssl rand -hex 32)"
  vault_key="$(openssl rand -hex 16)"

  ensure_env_key "ASSET_GATEWAY_DATABASE_URL" "postgres://${PG_USER}:${pg_password}@localhost:${PG_PORT}/${PG_DB}"
  ensure_env_key "ASSET_GATEWAY_JWT_SECRET" "${jwt_secret}"
  ensure_env_key "ASSET_GATEWAY_VAULT_KEY" "${vault_key}"
  ensure_env_key "ASSET_GATEWAY_DATA_DIR" "${DATA_DIR}"

  if [[ ! -f "${SERVICE_SOURCE}" ]]; then
    echo "[setup] Service template not found: ${SERVICE_SOURCE}" >&2
    exit 1
  fi

  echo "[setup] Installing systemd service: ${SERVICE_FILE}"
  sudo install -m 0644 "${SERVICE_SOURCE}" "${SERVICE_FILE}"
  sudo systemctl daemon-reload
  sudo systemctl enable "${SERVICE_NAME}"

  if [[ -x "${INSTALL_DIR}/asset-gateway" ]]; then
    echo "[setup] Restarting ${SERVICE_NAME}"
    sudo systemctl restart "${SERVICE_NAME}"
  else
    echo "[setup] Binary not found at ${INSTALL_DIR}/asset-gateway, skipping restart"
  fi

  echo "[setup] Done"
  echo "[setup] Env file: ${ENV_FILE}"
  echo "[setup] PostgreSQL container: ${PG_CONTAINER}"
}

main "$@"
