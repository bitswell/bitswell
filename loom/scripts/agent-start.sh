#!/usr/bin/env bash
# loom/scripts/agent-start.sh
#
# Hook: SubagentStart (non-blockable)
#
# Logs agent spawn with Agent-Id and Session-Id.
#
# stdin: JSON with at least { "agent_id": "...", "session_id": "...", "cwd": "..." }

set -euo pipefail

INPUT=$(cat)

AGENT_ID=$(printf '%s' "$INPUT" | jq -r '.agent_id // .subagent_type // "unknown"' 2>/dev/null || true)
SESSION_ID=$(printf '%s' "$INPUT" | jq -r '.session_id // ""' 2>/dev/null || true)
SUB_CWD=$(printf '%s' "$INPUT" | jq -r '.cwd // ""' 2>/dev/null || true)
TIMESTAMP=$(date -u +%Y-%m-%dT%H:%M:%SZ)

# Determine branch from subagent's cwd if available
BRANCH=""
if [[ -n "$SUB_CWD" ]] && [[ -d "$SUB_CWD" ]]; then
  BRANCH=$(git -C "$SUB_CWD" rev-parse --abbrev-ref HEAD 2>/dev/null || true)
fi

printf 'LOOM: SubagentStart: Agent-Id=%s Session-Id=%s branch=%s time=%s\n' \
  "$AGENT_ID" "$SESSION_ID" "${BRANCH:-unknown}" "$TIMESTAMP"

exit 0
