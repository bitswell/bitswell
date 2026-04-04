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

# --- dependency helpers ---

# Convert dependency format "agent/slug" to branch name "loom/agent-slug".
dep_to_branch() {
  local dep="$1"
  echo "loom/${dep//\//-}"
}

# --- dependency checking ---

check_dependencies() {
  local deps="$1"
  [[ "$deps" == "none" || -z "$deps" ]] && return 0
  local IFS=','
  for dep in $deps; do
    dep="$(echo "$dep" | xargs)"
    local branch_name
    branch_name="$(dep_to_branch "$dep")"
    # Check the exact branch for a COMPLETED status
    if git rev-parse --verify "$branch_name" >/dev/null 2>&1; then
      local s
      s="$(git log -1 --format='%(trailers:key=Task-Status,valueonly)' --grep='Task-Status:' "$branch_name" 2>/dev/null | head -1 | xargs)"
      [[ "$s" == "COMPLETED" ]] && continue
    fi
    echo "Dep not met: $dep (branch: $branch_name)" >&2
    return 1
  done
  return 0
}

# --- idempotency ---

lock_file_path() {
  echo "$MCAGENT_DIR/agents/$1/$2/.dispatch-lock"
}

is_dispatched() {
  local lock
  lock="$(lock_file_path "$1" "$2")"
  [[ -f "$lock" ]]
}

write_lock() {
  local lock pid="$2"
  lock="$(lock_file_path "$3" "$4")"
  mkdir -p "$(dirname "$lock")"
  echo "pid=$pid" > "$lock"
  echo "dispatched=$(date -u +%Y-%m-%dT%H:%M:%SZ)" >> "$lock"
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

  # Idempotency: skip if already dispatched
  if is_dispatched "$AGENT_ID" "$ASSIGNMENT_ID"; then
    echo "  SKIP: already dispatched (lock exists at $(lock_file_path "$AGENT_ID" "$ASSIGNMENT_ID"))"
    return 0
  fi

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

  # Write prompt to temp file for the spawn command.
  # Use quoted heredoc to prevent shell expansion of task_body.
  local prompt_file
  prompt_file="$(mktemp)"
  trap 'rm -f "$prompt_file"' EXIT

  cat > "$prompt_file" <<'PROMPT'
You are agent "__AGENT_ID__". Your assignment is "__ASSIGNMENT_ID__".
Your working directory is this git checkout. Use git commands normally (no -C flags needed).
Read your identity: __IDENTITY__
Read your AGENT.json: __AGENT_JSON__
Your task (from the assignment commit):
__TASK_BODY__
Commit protocol:
- Every commit: Agent-Id: __AGENT_ID__, Session-Id: __SESSION_ID__
- First commit: Task-Status: IMPLEMENTING
- Final commit: Task-Status: COMPLETED with Key-Finding trailer(s)
Build it. No ceremony.
PROMPT

  # Inject controlled variables into the template via sed.
  # task_body uses a temp file approach to handle multiline + special chars safely.
  sed -i "s|__AGENT_ID__|${AGENT_ID}|g" "$prompt_file"
  sed -i "s|__ASSIGNMENT_ID__|${ASSIGNMENT_ID}|g" "$prompt_file"
  sed -i "s|__IDENTITY__|${identity}|g" "$prompt_file"
  sed -i "s|__AGENT_JSON__|${agent_json}|g" "$prompt_file"
  sed -i "s|__SESSION_ID__|${agent_session_id}|g" "$prompt_file"

  # Replace __TASK_BODY__ with the actual task body.
  # Use a temp file + awk to handle multiline content and special characters.
  local body_file
  body_file="$(mktemp)"
  printf '%s' "$task_body" > "$body_file"
  awk -v placeholder="__TASK_BODY__" -v bodyfile="$body_file" '
    $0 ~ placeholder {
      while ((getline line < bodyfile) > 0) print line
      close(bodyfile)
      next
    }
    { print }
  ' "$prompt_file" > "${prompt_file}.tmp"
  mv "${prompt_file}.tmp" "$prompt_file"
  rm -f "$body_file"

  echo "  Prompt: $prompt_file"
  echo "  Spawning via: $SPAWN_CMD"
  echo "  PWD: $worktree"

  # cd into the worktree before spawning -- agent sees it as PWD
  (cd "$worktree" && "$SPAWN_CMD" "$prompt_file") &
  local spawn_pid=$!
  echo "  PID: $spawn_pid"

  # Write lock file with PID to prevent double-dispatch
  write_lock "$prompt_file" "$spawn_pid" "$AGENT_ID" "$ASSIGNMENT_ID"

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
