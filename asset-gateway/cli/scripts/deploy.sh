#!/usr/bin/env bash
set -euo pipefail

# Usage: ./scripts/deploy.sh
# 1) Cross-compile ARM64 release binary
# 2) Upload to Oracle host
# 3) Restart asset-gateway systemd service

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

TARGET="${TARGET:-aarch64-unknown-linux-gnu}"
BINARY_NAME="asset-gateway"
REMOTE_HOST="${REMOTE_HOST:-oracle}"
REMOTE_DIR="${REMOTE_DIR:-/opt/asset-gateway}"

build_binary() {
  if command -v cross >/dev/null 2>&1; then
    echo "[deploy] Building with cross for ${TARGET}"
    cross build --release --target "${TARGET}" --manifest-path "${PROJECT_ROOT}/Cargo.toml"
    return
  fi

  echo "[deploy] 'cross' not found, fallback to cargo build for ${TARGET}"
  cargo build --release --target "${TARGET}" --manifest-path "${PROJECT_ROOT}/Cargo.toml"
}

main() {
  build_binary

  local local_bin="${PROJECT_ROOT}/target/${TARGET}/release/${BINARY_NAME}"
  if [[ ! -f "${local_bin}" ]]; then
    echo "[deploy] Build succeeded but binary not found: ${local_bin}" >&2
    exit 1
  fi

  echo "[deploy] Preparing remote directory: ${REMOTE_HOST}:${REMOTE_DIR}"
  ssh "${REMOTE_HOST}" "sudo mkdir -p '${REMOTE_DIR}' && sudo chown \$(id -un):\$(id -gn) '${REMOTE_DIR}'"

  echo "[deploy] Uploading binary"
  scp "${local_bin}" "${REMOTE_HOST}:${REMOTE_DIR}/${BINARY_NAME}"

  echo "[deploy] Restarting systemd service"
  ssh "${REMOTE_HOST}" "sudo systemctl restart asset-gateway && sudo systemctl --no-pager --full status asset-gateway | sed -n '1,12p'"

  echo "[deploy] Done"
}

main "$@"
