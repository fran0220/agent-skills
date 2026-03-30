#!/usr/bin/env bash
# Migrate ai-search config from flat format to sectioned TOML
# Reads keys from ~/.openclaw/credentials/search.json and old config

set -euo pipefail

CONFIG_DIR="$HOME/.config/ai-search"
OLD_CONFIG="$CONFIG_DIR/config.toml"
NEW_CONFIG="$CONFIG_DIR/config.toml"
CREDS="$HOME/.openclaw/credentials/search.json"

echo "==> Migrating ai-search config to sectioned format..."

# Read old config values
OLD_API_URL=$(grep '^api_url' "$OLD_CONFIG" 2>/dev/null | sed 's/api_url = "\(.*\)"/\1/' || echo "")
OLD_API_KEY=$(grep '^api_key' "$OLD_CONFIG" 2>/dev/null | sed 's/api_key = "\(.*\)"/\1/' || echo "")
OLD_SEARCH_MODEL=$(grep '^search_model' "$OLD_CONFIG" 2>/dev/null | sed 's/search_model = "\(.*\)"/\1/' || echo "grok-4.1-fast")
OLD_ANALYSIS_MODEL=$(grep '^analysis_model' "$OLD_CONFIG" 2>/dev/null | sed 's/analysis_model = "\(.*\)"/\1/' || echo "gemini-3-flash-preview")

# Read keys from openclaw credentials
EXA_KEY=""
TAVILY_KEY=""
if [ -f "$CREDS" ]; then
    EXA_KEY=$(python3 -c "import json; print(json.load(open('$CREDS')).get('exa',''))" 2>/dev/null || echo "")
    TAVILY_KEY=$(python3 -c "import json; print(json.load(open('$CREDS')).get('tavily',''))" 2>/dev/null || echo "")
fi

# Write new config
cat > "$NEW_CONFIG" << EOF
[server]
port = 6900
gateway_token = ""

[proxy]
url = "${OLD_API_URL:-https://api.xiaomao.chat}"
key = "${OLD_API_KEY}"
search_model = "${OLD_SEARCH_MODEL}"
analysis_model = "${OLD_ANALYSIS_MODEL}"

[exa]
key = "${EXA_KEY}"

[tavily]
key = "${TAVILY_KEY}"

[search]
max_split = 10
timeout_secs = 180
default_mode = "fast"
EOF

chmod 600 "$NEW_CONFIG"
echo "==> Config written to $NEW_CONFIG"
echo "==> Proxy key: ${OLD_API_KEY:0:8}..."
echo "==> Exa key: ${EXA_KEY:+set}${EXA_KEY:-empty}"
echo "==> Tavily key: ${TAVILY_KEY:+set}${TAVILY_KEY:-empty}"
echo "==> Done!"
