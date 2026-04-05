# LOOM Worker Template

**Version**: 2.0.0-draft | **Protocol**: `loom/2` | **Status**: Draft

A runnable guide for agents operating under LOOM v2. Follow these steps in order.
The worktree is your workspace; commit messages are your protocol layer. No files
exist to track your state — only commits.

See `schemas.md` for the full commit template vocabulary. See `protocol.md` for the
lifecycle state machine.

---

## 1. Read Your Assignment

Your task is encoded in the ASSIGNED commit on your branch — the first commit placed
there by the orchestrator. Read the full commit body:

```bash
git log -1 --format='%B' HEAD
```

The commit body is your task specification: objective, context, acceptance criteria.
Read it completely before writing a single line of code.

Also read AGENT.json for your configuration — scope, budget, dependencies:

```bash
cat ../AGENT.json
```

AGENT.json lives at `.mcagent/agents/<name>/<assignment>/AGENT.json`, one level above
your worktree. The dispatch mechanism ensures it is readable at `../AGENT.json` from
your working directory.

**What you are looking for:**

- `scope.paths_allowed` — the files you are permitted to change
- `token_budget` — your total allowed spend; reserve 10% for the final commit
- `dependencies` — assignments that must be COMPLETED before you may start work
- `timeout_seconds` — heartbeat threshold; commit at least every 5 minutes

---

## 2. Check Dependencies

If `dependencies` in AGENT.json is non-empty, verify each is COMPLETED before
proceeding:

```bash
# Check a dependency's terminal status
git log -1 \
  --format='%(trailers:key=Task-Status,valueonly)' \
  --grep='Task-Status:' \
  loom/<dep-agent>-<dep-slug>
# Expected output: COMPLETED
```

If a dependency is not COMPLETED, commit a BLOCKED status (Section 7) and stop.
Do not begin work on a task whose prerequisites are unmet.

---

## 3. First Commit: Signal IMPLEMENTING

Your first commit signals that you are working. It MAY include file changes, but
`--allow-empty` is permitted when you need to signal start before files are ready.

```
chore(<scope>): begin <short task description>

Agent-Id: <your-agent-id>
Session-Id: <your-session-id>
Task-Status: IMPLEMENTING
Heartbeat: <ISO-8601 UTC>
```

This is the transition from ASSIGNED to IMPLEMENTING. The orchestrator monitors
for this commit to confirm the agent started.

---

## 4. Do the Work

Make changes within your worktree. Stay within `scope.paths_allowed`. Commit
regularly — at minimum every 5 minutes with a `Heartbeat` trailer. If you have
no file changes to commit, use `--allow-empty` to keep the heartbeat alive.

**Every work commit:**

```
<type>(<scope>): <subject>

<explain why if the change is non-obvious>

Agent-Id: <your-agent-id>
Session-Id: <your-session-id>
Heartbeat: <ISO-8601 UTC>
```

**Record discoveries as you make them:**

- `Key-Finding: <one-line discovery>` — important thing learned; repeatable
- `Decision: <what> -- <why>` — non-obvious choice; repeatable
- `Deviation: <what> -- <why>` — departure from the task spec; repeatable

These trailers accumulate in your branch history and become the audit trail
the orchestrator reads after integration. Write them incrementally; do not
try to reconstruct them at the end from memory.

---

## 5. Context Recovery

If your context is compacted mid-task, reconstruct state from your branch history
before resuming. `git log` is the authoritative record of everything you have done.

```bash
# What was I assigned?
git log --format='%B' --reverse | head -60

# What have I found so far?
git log --format='%(trailers:key=Key-Finding,valueonly)' | grep -v '^$'

# What decisions did I make?
git log --format='%(trailers:key=Decision,valueonly)' | grep -v '^$'

# What files have I changed?
git diff HEAD~$(git rev-list HEAD --count) --name-only

# Am I BLOCKED or still IMPLEMENTING?
git log -1 \
  --format='%(trailers:key=Task-Status,valueonly)' \
  --grep='Task-Status:' \
  HEAD
```

Re-read AGENT.json to confirm scope and budget. Resume from the last committed
state — never re-do work that is already committed.

---

## 6. Final Commit: Signal COMPLETED

When all acceptance criteria are met, make the completion commit. This is the
transition to COMPLETED — a terminal state. There is no un-completing.

```
<type>(<scope>): <short description of what was accomplished>

<Summary of what was done and why it matters. Include anything the orchestrator
needs to know before integrating this branch.>

Agent-Id: <your-agent-id>
Session-Id: <your-session-id>
Task-Status: COMPLETED
Files-Changed: <integer count of files modified across all commits>
Key-Finding: <most important discovery>
Heartbeat: <ISO-8601 UTC>
```

Requirements:
- `Files-Changed` is REQUIRED (integer >= 0)
- At least one `Key-Finding` is REQUIRED
- Multiple `Key-Finding` lines are permitted and encouraged
- `Decision` and `Deviation` trailers MAY be included here or in earlier commits

After this commit, do not push any further `Task-Status` commits. The orchestrator
takes over for integration.

---

## 7. If You Cannot Proceed: Signal BLOCKED

If you hit a blocker — missing dependency, unclear requirement, resource limit —
commit a BLOCKED status and stop work.

```
chore(<scope>): blocked -- <short reason>

<Detailed explanation: what is needed, who can unblock this, and what
should happen next. Be specific.>

Agent-Id: <your-agent-id>
Session-Id: <your-session-id>
Task-Status: BLOCKED
Blocked-Reason: <concise description of what is blocking progress>
Heartbeat: <ISO-8601 UTC>
```

The orchestrator will resolve the blocker and resume you by committing additional
context to your branch. When unblocked, make a new IMPLEMENTING commit and continue.

**Resource limit blocker** (at 90% of `token_budget`):

```
chore(<scope>): blocked -- resource limit

Reached 90% of token budget. Committing current state before context
window is exhausted.

Agent-Id: <your-agent-id>
Session-Id: <your-session-id>
Task-Status: BLOCKED
Blocked-Reason: resource_limit
Heartbeat: <ISO-8601 UTC>
```

---

## 8. If Work Cannot Complete: Signal FAILED

If you encounter an unrecoverable error:

```
chore(<scope>): failed -- <short reason>

<Detailed explanation of what went wrong, what was attempted, and
what information might help a retry agent succeed.>

Agent-Id: <your-agent-id>
Session-Id: <your-session-id>
Task-Status: FAILED
Error-Category: <task_unclear|blocked|resource_limit|conflict|internal>
Error-Retryable: <true|false>
```

FAILED is a terminal state. Do not commit further `Task-Status` after FAILED.
The orchestrator MAY spawn a new agent to retry. Your branch is preserved for
post-mortem — it will not be deleted.

---

## 9. Scope Enforcement

You MUST NOT modify files outside `scope.paths_allowed` in AGENT.json. The
orchestrator verifies scope at integration and will reject the branch if any
commit touches a file outside scope.

If you discover mid-task that you need to modify out-of-scope files, commit
BLOCKED and explain the scope expansion needed. Do not proceed.

---

## Quick Reference

| Situation | Required trailers |
|-----------|------------------|
| First commit | `Agent-Id`, `Session-Id`, `Task-Status: IMPLEMENTING`, `Heartbeat` |
| Work commit | `Agent-Id`, `Session-Id`, `Heartbeat` |
| Completed | `Agent-Id`, `Session-Id`, `Task-Status: COMPLETED`, `Files-Changed`, `Key-Finding` (>=1), `Heartbeat` |
| Blocked | `Agent-Id`, `Session-Id`, `Task-Status: BLOCKED`, `Blocked-Reason`, `Heartbeat` |
| Failed | `Agent-Id`, `Session-Id`, `Task-Status: FAILED`, `Error-Category`, `Error-Retryable` |

**Invariants:**
- Every commit: `Agent-Id` + `Session-Id`
- While running: `Heartbeat` on every commit, at least every 5 minutes
- Worktree: no protocol files of any kind — only deliverable code
- Scope: never write outside `scope.paths_allowed`
- Budget: reserve 10% of `token_budget` for the final commit

---

*End of LOOM Worker Template v2.0.0-draft.*
