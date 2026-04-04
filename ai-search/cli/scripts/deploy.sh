#!/usr/bin/env bash
set -euo pipefail

# Usage: ./scripts/deploy.sh
#
# Primary deploy path: push to main → GitHub Actions CI builds & deploys automatically.
# This script is a manual fallback that builds on the server directly via SSH.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

BINARY_NAME="ai-search"
REMOTE_HOST="${REMOTE_HOST:-bwg}"
REMOTE_DIR="${REMOTE_DIR:-/opt/ai-search}"
REMOTE_SRC="/tmp/ai-search-build"

main() {
  echo "[deploy] Syncing source to ${REMOTE_HOST}:${REMOTE_SRC}"
  rsync -az --delete \
    --exclude 'target/' \
    --exclude '.git/' \
    "${PROJECT_ROOT}/" "${REMOTE_HOST}:${REMOTE_SRC}/"

  echo "[deploy] Building release on ${REMOTE_HOST}"
  ssh "${REMOTE_HOST}" "cd ${REMOTE_SRC} && cargo build --release"

  echo "[deploy] Installing binary"
  ssh "${REMOTE_HOST}" "\
    mkdir -p '${REMOTE_DIR}' && \
    cp ${REMOTE_SRC}/target/release/${BINARY_NAME} ${REMOTE_DIR}/${BINARY_NAME}"

  echo "[deploy] Restarting service"
  ssh "${REMOTE_HOST}" "\
    systemctl restart ${BINARY_NAME} && \
    systemctl --no-pager --full status ${BINARY_NAME} | head -12"

  echo "[deploy] Done"
}

main "$@"
