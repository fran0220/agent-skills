#!/usr/bin/env bash
set -euo pipefail

# Usage: ./scripts/deploy.sh
#
# Primary deploy path: push to main → GitHub Actions CI builds & deploys automatically.
# This script is a manual fallback that builds on the server directly via SSH.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

BINARY_NAME="asset-gateway"
REMOTE_HOST="${REMOTE_HOST:-oracle}"
REMOTE_DIR="${REMOTE_DIR:-/opt/asset-gateway}"
REMOTE_SRC="/tmp/asset-gateway-build"

main() {
  echo "[deploy] Syncing source to ${REMOTE_HOST}:${REMOTE_SRC}"
  rsync -az --delete \
    --exclude 'target/' \
    --exclude '.git/' \
    "${PROJECT_ROOT}/" "${REMOTE_HOST}:${REMOTE_SRC}/"

  echo "[deploy] Building release on ${REMOTE_HOST}"
  ssh "${REMOTE_HOST}" "\
    cd ${REMOTE_SRC} && \
    ~/.cargo/bin/cargo build --release 2>&1 | tail -3"

  echo "[deploy] Installing binary"
  ssh "${REMOTE_HOST}" "\
    sudo mkdir -p '${REMOTE_DIR}' && \
    sudo chown \$(id -un):\$(id -gn) '${REMOTE_DIR}' && \
    cp ${REMOTE_SRC}/target/release/${BINARY_NAME} ${REMOTE_DIR}/${BINARY_NAME}"

  echo "[deploy] Restarting service"
  ssh "${REMOTE_HOST}" "\
    sudo systemctl restart ${BINARY_NAME} && \
    sudo systemctl --no-pager --full status ${BINARY_NAME} | head -12"

  echo "[deploy] Done"
}

main "$@"
