#!/usr/bin/env bash
set -euo pipefail

SERVER="opc@161.33.13.122"
REMOTE_DIR="/opt/ai-search"
BUILD_DIR="/tmp/ai-search-build"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
CLI_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "==> Syncing source..."
rsync -az --exclude target --exclude .git \
  "$CLI_DIR/" "$SERVER:$BUILD_DIR/"

echo "==> Building on server..."
ssh "$SERVER" "cd $BUILD_DIR && cargo build --release"

echo "==> Deploying..."
ssh "$SERVER" "sudo systemctl stop ai-search || true"
ssh "$SERVER" "sudo mkdir -p $REMOTE_DIR"
ssh "$SERVER" "sudo cp $BUILD_DIR/target/release/ai-search $REMOTE_DIR/"
ssh "$SERVER" "sudo cp $BUILD_DIR/scripts/ai-search.service /etc/systemd/system/ai-search.service"
ssh "$SERVER" "sudo systemctl daemon-reload"
ssh "$SERVER" "sudo systemctl start ai-search"

echo "==> Done! Check: curl https://search.xiaomao.chat/health"
