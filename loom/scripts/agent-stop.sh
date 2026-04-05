#!/usr/bin/env bash
# loom/scripts/agent-stop.sh
#
# Hook: SubagentStop (blockable — exit 2 blocks completion)
#
# Checks that the agent's final commit on its loom/* branch includes a
# Task-Status trailer. Warns if missing; does not block (exit 0) so normal
# operation is never interrupted by a missing trailer.
#
# Non-loom branches are a graceful no-op (exit 0).
#
# stdin: JSON with at least { "agent_id": "...", "session_id": "...", "cwd": "..." }

set -euo pipefail

INPUT=$(cat)

AGENT_ID=$(printf '%s' "$INPUT" | jq -r '.agent_id // .subagent_type // "unknown"' 2>/dev/null || true)
SESSION_ID=$(printf '%s' "$INPUT" | jq -r '.session_id // ""' 2>/dev/null || true)
SUB_CWD=$(printf '%s' "$INPUT" | jq -r '.cwd // ""' 2>/dev/null || true)
TIMESTAMP=$(date -u +%Y-%m-%dT%H:%M:%SZ)

# Determine the branch the subagent was working on
BRANCH=""

# Prefer branch from hook payload
BRANCH=$(printf '%s' "$INPUT" | jq -r '.branch // ""' 2>/dev/null || true)

# Fall back: derive from subagent cwd
if [[ -z "$BRANCH" ]] && [[ -n "$SUB_CWD" ]] && [[ -d "$SUB_CWD" ]]; then
  BRANCH=$(git -C "$SUB_CWD" rev-parse --abbrev-ref HEAD 2>/dev/null || true)
fi

# Not a loom/* branch — graceful no-op
if [[ -z "$BRANCH" ]] || [[ "$BRANCH" != loom/* ]]; then
  exit 0
fi

printf 'LOOM: SubagentStop: Agent-Id=%s Session-Id=%s branch=%s time=%s\n' \
  "$AGENT_ID" "$SESSION_ID" "$BRANCH" "$TIMESTAMP"

# Check the final commit on the agent's branch for a Task-Status trailer
GIT_DIR_ARGS=()
if [[ -n "$SUB_CWD" ]] && [[ -d "$SUB_CWD" ]]; then
  GIT_DIR_ARGS=(-C "$SUB_CWD")
fi

LAST_STATUS=$(git "${GIT_DIR_ARGS[@]}" log -1 \
  --format='%(trailers:key=Task-Status,valueonly)' \
  2>/dev/null | head -1 | tr -d '[:space:]')

if [[ -z "$LAST_STATUS" ]]; then
  printf 'LOOM: WARN: Final commit on %s has no Task-Status trailer\n' "$BRANCH" >&2
  printf 'LOOM: WARN: Agent-Id=%s may have stopped without reaching terminal state\n' "$AGENT_ID" >&2
  printf 'LOOM: WARN: Expected Task-Status: COMPLETED or FAILED in final commit\n' >&2
  # Exit 0 — warn only; do not block agent stop
  exit 0
fi

case "$LAST_STATUS" in
  COMPLETED|FAILED)
    printf 'LOOM: Task-Status confirmed: %s on %s\n' "$LAST_STATUS" "$BRANCH"
    ;;
  BLOCKED)
    printf 'LOOM: WARN: Agent stopped in BLOCKED state on %s\n' "$BRANCH" >&2
    ;;
  *)
    printf 'LOOM: WARN: Non-terminal Task-Status "%s" on %s\n' "$LAST_STATUS" "$BRANCH" >&2
    ;;
esac

exit 0
