#!/usr/bin/env bash
# loom-dispatch.sh -- Detect ASSIGNED commits and spawn agents.
#
# Usage:
#   loom-dispatch.sh [--commit <sha>] [--branch <branch>] [--dry-run]
#   loom-dispatch.sh --scan [--dry-run]
#
# Modes:
#   --commit SHA    Check a specific commit for ASSIGNED trailers and dispatch.
#   --branch NAME   Check the tip of a specific branch.
#   --scan          Scan all loom/* branches for undispatched assignments.
#   --dry-run       Print what would happen without spawning.
#
# Environment:
#   LOOM_SPAWN_CMD  Command to spawn an agent. Receives prompt on stdin.
#                   Default: loom-spawn.sh (sibling script)
#
# Requirements: git, jq

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
MCAGENT_DIR="$REPO_ROOT/.mcagent"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
SPAWN_CMD="${LOOM_SPAWN_CMD:-$SCRIPT_DIR/loom-spawn.sh}"

MODE=""
TARGET_COMMIT=""
TARGET_BRANCH=""
DRY_RUN=false

while [[ $# -gt 0 ]]; do
  case "$1" in
    --commit)  MODE="commit";  TARGET_COMMIT="$2"; shift 2 ;;
    --branch)  MODE="branch";  TARGET_BRANCH="$2"; shift 2 ;;
    --scan)    MODE="scan";    shift ;;
    --dry-run) DRY_RUN=true;   shift ;;
    -h|--help) sed -n '2,/^$/{ s/^# //; s/^#//; p }' "$0"; exit 0 ;;
    *) echo "Unknown option: $1" >&2; exit 1 ;;
  esac
done

[[ -z "$MODE" ]] && { echo "Error: specify --commit, --branch, or --scan" >&2; exit 1; }

# --- trailer extraction ---

trailer_value() {
  local sha="$1" key="$2"
  git log -1 --format="%(trailers:key=$key,valueonly)" "$sha" | head -1 | xargs
}

is_assignment_commit() {
  [[ "$(trailer_value "$1" "Task-Status")" == "ASSIGNED" ]]
}

extract_assignment() {
  local sha="$1"
  AGENT_ID="$(trailer_value "$sha" "Assigned-To")"
  ASSIGNMENT_ID="$(trailer_value "$sha" "Assignment")"
  SCOPE="$(trailer_value "$sha" "Scope")"
  DEPENDENCIES="$(trailer_value "$sha" "Dependencies")"
  BUDGET="$(trailer_value "$sha" "Budget")"
  SESSION_ID="$(trailer_value "$sha" "Session-Id")"
}

# --- agent lookup ---

find_agent_json() {
  local p="$MCAGENT_DIR/agents/$1/$2/AGENT.json"
  [[ -f "$p" ]] && echo "$p" || { echo ""; return 1; }
}

find_worktree() {
  local p="$MCAGENT_DIR/agents/$1/$2/worktree"
  [[ -d "$p" ]] && echo "$p" || { echo ""; return 1; }
}

find_identity() {
  local p="$MCAGENT_DIR/agents/$1/identity.md"
  [[ -f "$p" ]] && { echo "$p"; return 0; }
  p="$REPO_ROOT/agents/$1/identity.md"
  [[ -f "$p" ]] && { echo "$p"; return 0; }
  echo ""; return 1
}

# --- dependency checking ---

check_dependencies() {
  local deps="$1"
  [[ "$deps" == "none" || -z "$deps" ]] && return 0
  local IFS=','
  for dep in $deps; do
    dep="$(echo "$dep" | xargs)"
    local dep_agent="${dep%%/*}" completed=false
    for branch in $(git branch --list "loom/${dep_agent}*" --format='%(refname:short)' 2>/dev/null); do
      local s
      s="$(git log -1 --format='%(trailers:key=Task-Status,valueonly)' --grep='Task-Status:' "$branch" 2>/dev/null | head -1 | xargs)"
      [[ "$s" == "COMPLETED" ]] && { completed=true; break; }
    done
    [[ "$completed" != "true" ]] && { echo "Dep not met: $dep" >&2; return 1; }
  done
  return 0
}

# --- dispatch ---

dispatch_agent() {
  local sha="$1"
  extract_assignment "$sha"
  [[ -z "$AGENT_ID" ]] && { echo "Error: no Assigned-To in $sha" >&2; return 1; }

  echo "--- Dispatch ---"
  echo "  Commit:       $sha"
  echo "  Agent:        $AGENT_ID"
  echo "  Assignment:   $ASSIGNMENT_ID"
  echo "  Scope:        $SCOPE"
  echo "  Dependencies: $DEPENDENCIES"
  echo "  Budget:       $BUDGET"

  local agent_json worktree identity
  agent_json="$(find_agent_json "$AGENT_ID" "$ASSIGNMENT_ID")" || true
  worktree="$(find_worktree "$AGENT_ID" "$ASSIGNMENT_ID")" || true
  identity="$(find_identity "$AGENT_ID")" || true

  [[ -z "$agent_json" ]] && echo "  WARN: AGENT.json not found" >&2 || echo "  AGENT.json:   $agent_json"
  [[ -z "$worktree" ]] && echo "  WARN: worktree not found" >&2 || echo "  Worktree:     $worktree"
  [[ -n "$identity" ]] && echo "  Identity:     $identity"

  check_dependencies "$DEPENDENCIES" || { echo "  BLOCKED on deps" >&2; return 1; }
  echo "  Dependencies: OK"

  [[ "$DRY_RUN" == "true" ]] && { echo "  [DRY RUN] Would spawn $AGENT_ID"; return 0; }
  [[ -z "$worktree" ]] && { echo "  ERROR: No worktree" >&2; return 1; }

  local agent_session_id=""
  [[ -n "$agent_json" ]] && agent_session_id="$(jq -r '.session_id // empty' "$agent_json" 2>/dev/null)"

  local task_body
  task_body="$(git log -1 --format='%b' "$sha")"

  # Write prompt to temp file for the spawn command
  local prompt_file
  prompt_file="$(mktemp)"
  cat > "$prompt_file" <<PROMPT
You are agent "$AGENT_ID". Your assignment is "$ASSIGNMENT_ID".
Your worktree: $worktree
Read your identity: $identity
Read your AGENT.json: $agent_json
Your task (from the assignment commit):
$task_body
Commit protocol:
- git -C $worktree for all git commands
- Every commit: Agent-Id: $AGENT_ID, Session-Id: $agent_session_id
- First commit: Task-Status: IMPLEMENTING
- Final commit: Task-Status: COMPLETED with Key-Finding trailer(s)
Build it. No ceremony.
PROMPT

  echo "  Prompt: $prompt_file"
  echo "  Spawning via: $SPAWN_CMD"

  "$SPAWN_CMD" "$prompt_file" &
  echo "  PID: $!"
  echo "  Agent $AGENT_ID dispatched."
}

dispatch_commit() {
  local sha="$1"
  is_assignment_commit "$sha" || { echo "$sha is not ASSIGNED" >&2; return 1; }
  dispatch_agent "$sha"
}

case "$MODE" in
  commit) dispatch_commit "$TARGET_COMMIT" ;;
  branch)
    sha="$(git log -1 --format='%H' --grep='Task-Status: ASSIGNED' "$TARGET_BRANCH" 2>/dev/null)"
    [[ -z "$sha" ]] && { echo "No ASSIGNED commit on $TARGET_BRANCH" >&2; exit 1; }
    dispatch_commit "$sha"
    ;;
  scan)
    echo "Scanning loom/* branches..."
    found=0
    for branch in $(git branch --list 'loom/*' --format='%(refname:short)' 2>/dev/null); do
      s="$(git log -1 --format='%(trailers:key=Task-Status,valueonly)' --grep='Task-Status:' "$branch" 2>/dev/null | head -1 | xargs)"
      if [[ "$s" == "ASSIGNED" ]]; then
        sha="$(git log -1 --format='%H' --grep='Task-Status: ASSIGNED' "$branch" 2>/dev/null)"
        echo ""; echo "Found: $branch"
        dispatch_agent "$sha" || true
        found=$((found + 1))
      fi
    done
    [[ "$found" -eq 0 ]] && echo "None found." || echo "Dispatched $found."
    ;;
esac
