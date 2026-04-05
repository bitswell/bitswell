---
name: loom:orchestrate
description: Orchestrate LOOM v2 agents — assign work, dispatch via commit-based protocol, monitor progress, integrate results.
---

# LOOM Orchestrate

You are the orchestrator (`bitswell`). This skill walks you through the full LOOM v2 workflow.

Reference docs: `loom/skills/orchestrate/references/`
- `protocol.md` — lifecycle, state machine, operations
- `schemas.md` — commit format, trailer vocabulary, branch naming
- `mcagent-spec.md` — agent spec and conformance rules
- `examples.md` — concrete usage examples

---

## 1. Assign Work

Create a branch and commit the task with `Task-Status: ASSIGNED`.

```bash
# Create branch from main (or a specific base ref)
git checkout main
git checkout -b loom/<agent>-<slug>

# Commit the assignment
git commit --allow-empty -m "$(cat <<'EOF'
task(<agent>): <short task description>

<Full task description. Include: objective, context, acceptance criteria.
This is the agent's task spec — be precise.>

Agent-Id: bitswell
Session-Id: <your-session-id>
Task-Status: ASSIGNED
Assigned-To: <agent>
Assignment: <slug>
Scope: <paths, e.g. loom/skills/**>
Dependencies: <agent/slug refs, comma-separated, or none>
Budget: 150000
EOF
)"
```

**Required ASSIGNED trailers**: `Agent-Id`, `Session-Id`, `Task-Status`, `Assigned-To`, `Assignment`, `Scope`, `Dependencies`, `Budget`.

---

## 2. Dispatch an Agent

### Option A — loom-dispatch (automated, from PATH)

Scans for ASSIGNED commits and spawns agents:

```bash
# Dispatch a specific branch
loom-dispatch --branch loom/<agent>-<slug>

# Dispatch a specific commit
loom-dispatch --commit <sha>

# Scan all loom/* branches for undispatched work
loom-dispatch --scan

# Dry run — print what would happen
loom-dispatch --branch loom/<agent>-<slug> --dry-run
```

### Option B — @loom:loom-worker (direct agent spawn)

Spawn the worker agent directly via the Agent tool. The agent reads the ASSIGNED commit from its branch, reads its identity from `agents/<name>/identity.md`, and executes the task.

```
Spawn @loom:loom-worker with:
  - branch: loom/<agent>-<slug>
  - The agent reads its own ASSIGNED commit for context
```

### Option C — loom-spawn (manual)

```bash
# Spawn directly with a prompt file
loom-spawn <prompt-file>
# Must be run with PWD set to the agent's worktree
```

---

## 3. Monitor Progress

All state lives in commit trailers. Query with `git log --format`.

```bash
# Latest status of a branch
git log -1 --format='%(trailers:key=Task-Status,valueonly)' \
  --grep='Task-Status:' loom/<agent>-<slug>

# Last heartbeat (liveness check)
git log -1 --format='%(trailers:key=Heartbeat,valueonly)' loom/<agent>-<slug>

# All findings from a completed agent
git log --format='%(trailers:key=Key-Finding,valueonly)' loom/<agent>-<slug> \
  | grep -v '^$'

# Full trailer dump
git log --format='%H %s%n%(trailers)%n---' loom/<agent>-<slug>

# Find all ASSIGNED (undispatched) branches
for b in $(git branch --list 'loom/*' --format='%(refname:short)'); do
  s=$(git log -1 --format='%(trailers:key=Task-Status,valueonly)' \
    --grep='Task-Status:' "$b" | head -1 | xargs)
  [[ "$s" == "ASSIGNED" ]] && echo "$b"
done
```

States: `ASSIGNED` → `IMPLEMENTING` → `COMPLETED` (or `BLOCKED` / `FAILED`).

---

## 4. Check Dependencies

A dependency `<agent>/<slug>` maps to branch `loom/<agent>-<slug>`. It is met when that branch's latest `Task-Status` is `COMPLETED`.

```bash
git log -1 --format='%(trailers:key=Task-Status,valueonly)' \
  --grep='Task-Status:' loom/<dep-agent>-<dep-slug>
# Result: "COMPLETED" means met
```

`loom-dispatch` checks dependencies automatically before spawning.

---

## 5. Integrate Completed Work

After an agent's branch shows `Task-Status: COMPLETED`:

1. Verify scope — all changed files must be within the assignment's `Scope`.
2. Merge into the workspace branch.
3. Run project validation (tests, lint).
4. If validation passes, commit. If not, record a `Deviation` and coordinate resolution.

```bash
git checkout main
git merge --no-ff loom/<agent>-<slug>
# Run validation
# If OK: commit with Agent-Id: bitswell, Session-Id: <session>
```

Post-integration, the agent's branch is retained (minimum 30 days).

---

## 6. Handle Blocked or Failed Agents

**BLOCKED**: Wait for the blocker to resolve, then the agent resumes IMPLEMENTING.

**FAILED**: Read `Error-Category` and `Error-Retryable` from the commit. If retryable, spawn a new agent on a new branch (do not reuse the failed branch).

```bash
git log -1 --format='%(trailers:key=Error-Category,valueonly)%n%(trailers:key=Error-Retryable,valueonly)' loom/<agent>-<slug>
```

Failed branches MUST NOT be deleted. Retain for post-mortem.

---

## 7. Orchestrator Commit Protocol

Every orchestrator commit must include:
- `Agent-Id: bitswell`
- `Session-Id: <session-id>`

Post-terminal commits (after agent reaches COMPLETED or FAILED) use `chore(loom):` type with NO `Task-Status` trailer.

---

See `references/` for full schemas, protocol details, and examples.
