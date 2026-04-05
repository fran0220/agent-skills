#!/usr/bin/env bash
# Tripo3D 管线联调：需已部署的网关 + 有效 token + Tripo 额度。
#
# 用法：
#   export ASSET_GATEWAY_URL="https://upload.xiaomao.chat"
#   export ASSET_GATEWAY_TOKEN="agk_..."   # 或已 asset-gateway auth set
#   ./scripts/e2e-process3d-chain.sh [--skip-generate]
#
# 默认会跑「文本生成 3D → 取 task_id → convert(GLB)」；若已有 TRIPO_TASK_ID 可跳过生成：
#   TRIPO_TASK_ID="<uuid>" ./scripts/e2e-process3d-chain.sh --skip-generate
#
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
NPM_CLI="${ASSET_GATEWAY_NPM:-$ROOT/npm/dist/index.js}"

if [[ ! -f "$NPM_CLI" ]]; then
  echo "Build npm CLI first: cd $ROOT/npm && pnpm install && pnpm run build" >&2
  exit 1
fi

SKIP_GEN=false
for arg in "$@"; do
  [[ "$arg" == "--skip-generate" ]] && SKIP_GEN=true
done

: "${ASSET_GATEWAY_URL:=https://upload.xiaomao.chat}"
export ASSET_GATEWAY_URL

OUT="${TMPDIR:-/tmp}/agw-process3d-e2e"
mkdir -p "$OUT"

run() {
  echo "==> $*"
  node "$NPM_CLI" "$@"
}

if [[ "$SKIP_GEN" != true ]] && [[ -z "${TRIPO_TASK_ID:-}" ]]; then
  echo ">>> generate model (text → Tripo task)…"
  run generate model --prompt "low poly toy cube" --output-dir "$OUT" | tee "$OUT/generate.json"
  # task id 在响应 JSON 的 metadata 或 tripo_task_id 字段（依网关版本）
  TID="$(node -e "
    const fs=require('fs');
    const j=JSON.parse(fs.readFileSync('$OUT/generate.json','utf8'));
    const d=j.data||j;
    const m=d.metadata||{};
    const id=d.tripo_task_id||m.tripo_task_id||m.task_id;
    if(!id){console.error('Could not find tripo_task_id in response'); process.exit(1)}
    console.log(String(id));
  ")"
  export TRIPO_TASK_ID="$TID"
  echo "TRIPO_TASK_ID=$TRIPO_TASK_ID"
else
  : "${TRIPO_TASK_ID:?Set TRIPO_TASK_ID or run without --skip-generate}"
fi

echo ">>> process3d convert → GLB"
run process3d convert --task-id "$TRIPO_TASK_ID" --format GLTF --output-dir "$OUT"

echo "Done. Artifacts under $OUT"
