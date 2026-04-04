#!/usr/bin/env bash
# loom-spawn.sh -- Spawn a LOOM agent via Claude Code CLI.
#
# Usage:
#   loom-spawn.sh <prompt-file>
#
# The prompt file contains the agent's full instructions.
# This script invokes the Claude Code CLI with the prompt as input.
# It must be run with PWD set to the agent's worktree (loom-dispatch.sh handles this).
#
# Environment:
#   CLAUDE_CMD    Claude Code CLI command. Default: claude
#   CLAUDE_ARGS   Extra args for claude CLI. Default: empty

set -euo pipefail

PROMPT_FILE="${1:?Usage: loom-spawn.sh <prompt-file>}"
CLAUDE="${CLAUDE_CMD:-claude}"
EXTRA_ARGS="${CLAUDE_ARGS:-}"

[[ -f "$PROMPT_FILE" ]] || { echo "Error: prompt file not found: $PROMPT_FILE" >&2; exit 1; }

# Clean up prompt file on exit
trap 'rm -f "$PROMPT_FILE"' EXIT

# Verify we're in a git worktree
git rev-parse --is-inside-work-tree >/dev/null 2>&1 || {
  echo "Error: not inside a git worktree. loom-dispatch.sh should cd into the worktree before calling this." >&2
  exit 1
}

echo "loom-spawn: worktree=$(pwd)"
echo "loom-spawn: prompt=$PROMPT_FILE ($(wc -c < "$PROMPT_FILE") bytes)"
echo "loom-spawn: invoking $CLAUDE"

# Build args array. Agent needs tools (interactive mode, no --print).
# Add --yes for auto-approval unless CLAUDE_ARGS already contains it.
ARGS=()
if [[ -n "$EXTRA_ARGS" ]]; then
  # shellcheck disable=SC2206
  ARGS=($EXTRA_ARGS)
fi
if [[ "$EXTRA_ARGS" != *"--yes"* ]]; then
  ARGS+=(--yes)
fi

# Spawn Claude Code with the prompt. Agent runs in interactive mode with tools.
exec "$CLAUDE" "${ARGS[@]}" < "$PROMPT_FILE"
