# LOOM Schemas Reference

**Version**: 2.0.0-draft | **Protocol**: `loom/2` | **Status**: Draft

Defines AGENT.json schema, commit message format, branch naming, commit-based protocol (trailer vocabulary, state requirements, templates, extraction queries, validation rules).

See `mcagent-spec.md` for directory layout. See `protocol.md` for lifecycle state machine and operations.

---

## 1. AGENT.json Schema

Written by the orchestrator at assignment creation. Lives at `.mcagent/agents/<name>/<assignment>/AGENT.json` — outside the worktree. Read-only for the agent.

```json
{
  "agent_id": "<agent-name>",
  "assignment_id": "<sequence>-<slug>",
  "session_id": "<uuid-v4>",
  "protocol_version": "loom/2",
  "repo": "<org>/<repo>",
  "base_ref": "<branch-or-sha>",
  "context_window_tokens": 200000,
  "token_budget": 100000,
  "dependencies": [],
  "scope": {
    "paths_allowed": ["."],
    "paths_denied": []
  },
  "timeout_seconds": 3600,
  "dispatch": {
    "mode": "sync",
    "trigger_ref": ""
  }
}
```

| Field | Type | Required | Constraints |
|-------|------|----------|-------------|
| `agent_id` | string | yes | Kebab-case (`[a-z0-9]+(-[a-z0-9]+)*`) |
| `assignment_id` | string | yes | Matches the assignment directory name |
| `session_id` | string | yes | UUID v4, unique per invocation |
| `protocol_version` | string | yes | Literal `"loom/2"` |
| `repo` | string | yes | Target repo in `<org>/<repo>` format |
| `base_ref` | string | yes | Branch or commit SHA |
| `context_window_tokens` | integer | yes | Positive integer |
| `token_budget` | integer | yes | Positive integer, <= `context_window_tokens` |
| `dependencies` | string[] | yes | Array of `<agent>/<slug>` refs. Graph MUST be a DAG. |
| `scope.paths_allowed` | string[] | yes | Globs relative to worktree root. `["."]` means full access. |
| `scope.paths_denied` | string[] | yes | Globs. Deny takes precedence. May be empty `[]`. |
| `timeout_seconds` | integer | yes | Default: 3600 |
| `dispatch.mode` | string | yes | `"sync"` or `"push-event"` |
| `dispatch.trigger_ref` | string | conditional | Required when mode is `"push-event"` |

---

## 2. Commit Message Format

All agent and orchestrator commits MUST use Conventional Commits with required trailers.

```
<type>(<scope>): <subject>

<body -- optional, explains "why" not "what">

Agent-Id: <agent-id>
Session-Id: <session-id>
[Task-Status: <state>]
[...additional trailers]
```

**Type values:** `feat`, `fix`, `docs`, `refactor`, `test`, `chore`, `task`

- `task` — orchestrator assignment commits (`Task-Status: ASSIGNED`)
- `chore(loom)` — orchestrator post-terminal commits (no `Task-Status`)
- Other types — agent work commits

Both `Agent-Id` and `Session-Id` are REQUIRED on every commit.

---

## 3. Branch Naming Convention

**Pattern:** `loom/<agent>-<slug>`

The branch name encodes the agent and assignment slug. Dependencies in AGENT.json use `<agent>/<slug>` format. To resolve: replace `/` with `-`, prepend `loom/`.

| Dependency | Branch |
|-----------|--------|
| `ratchet/commit-schema` | `loom/ratchet-commit-schema` |
| `moss/migrate-identities` | `loom/moss-migrate-identities` |

**Constraints:**
- Kebab-case: `[a-z0-9]+(-[a-z0-9]+)*`
- Maximum length: 63 characters
- One agent per branch, one branch per assignment

---

## 4. Trailer Vocabulary

All trailers follow `git-interpret-trailers(1)` syntax.

### 4.1 Universal trailers (every commit)

| Trailer | Type | Description |
|---------|------|-------------|
| `Agent-Id` | string | Agent name (e.g., `ratchet`, `bitswell`). Kebab-case. |
| `Session-Id` | string | UUID v4. Unique per agent invocation. ASSIGNED commit carries orchestrator's session. |

### 4.2 State trailers

| Trailer | Type | Description |
|---------|------|-------------|
| `Task-Status` | enum | One of: `ASSIGNED`, `IMPLEMENTING`, `COMPLETED`, `BLOCKED`, `FAILED` |
| `Heartbeat` | string | ISO-8601 UTC timestamp. SHOULD appear on every commit while agent is running. |

### 4.3 Assignment trailers (ASSIGNED commits only)

| Trailer | Type | Description |
|---------|------|-------------|
| `Assigned-To` | string | Agent-id of the assignee. |
| `Assignment` | string | Assignment identifier (e.g., `2-commit-schema`). |
| `Scope` | string | Allowed paths (e.g., `.`). |
| `Scope-Denied` | string | Denied paths. OPTIONAL. Omit if none. |
| `Dependencies` | string | Comma-separated `<agent>/<slug>` refs or `none`. |
| `Budget` | integer | Token budget. |

### 4.4 Completion trailers (COMPLETED commits)

| Trailer | Type | Description |
|---------|------|-------------|
| `Files-Changed` | integer | Files modified (>= 0). REQUIRED. |
| `Key-Finding` | string | Important discovery. Repeatable. At least one REQUIRED. |
| `Decision` | string | Non-obvious choice, format `<what> -- <why>`. Repeatable. OPTIONAL. |
| `Deviation` | string | Departure from task spec, format `<what> -- <why>`. Repeatable. OPTIONAL. |

### 4.5 Error trailers (BLOCKED and FAILED commits)

| Trailer | Type | Description |
|---------|------|-------------|
| `Blocked-Reason` | string | What is preventing progress. REQUIRED on BLOCKED. |
| `Error-Category` | enum | `task_unclear`, `blocked`, `resource_limit`, `conflict`, `internal`. REQUIRED on FAILED. |
| `Error-Retryable` | boolean | `true` or `false`. REQUIRED on FAILED. |

---

## 5. Required Trailers Per State

### 5.1 ASSIGNED (orchestrator writes)

| Trailer | Required |
|---------|----------|
| `Agent-Id` | yes (orchestrator's id: `bitswell`) |
| `Session-Id` | yes |
| `Task-Status` | yes — value `ASSIGNED` |
| `Assigned-To` | yes |
| `Assignment` | yes |
| `Scope` | yes |
| `Dependencies` | yes |
| `Budget` | yes |

### 5.2 IMPLEMENTING (agent writes)

| Trailer | Required |
|---------|----------|
| `Agent-Id` | yes |
| `Session-Id` | yes |
| `Task-Status` | yes — value `IMPLEMENTING` |
| `Heartbeat` | yes |

### 5.3 COMPLETED (agent writes)

| Trailer | Required |
|---------|----------|
| `Agent-Id` | yes |
| `Session-Id` | yes |
| `Task-Status` | yes — value `COMPLETED` |
| `Files-Changed` | yes |
| `Key-Finding` | yes (at least one) |
| `Heartbeat` | yes |

### 5.4 BLOCKED (agent writes)

| Trailer | Required |
|---------|----------|
| `Agent-Id` | yes |
| `Session-Id` | yes |
| `Task-Status` | yes — value `BLOCKED` |
| `Blocked-Reason` | yes |

### 5.5 FAILED (agent writes)

| Trailer | Required |
|---------|----------|
| `Agent-Id` | yes |
| `Session-Id` | yes |
| `Task-Status` | yes — value `FAILED` |
| `Error-Category` | yes |
| `Error-Retryable` | yes |

---

## 6. Commit Templates

### 6.1 Orchestrator: Task Assignment

```
task(<agent-id>): <short task description>

<Full task description. This replaces TASK.md.
Include objective, context, acceptance criteria.>

Agent-Id: bitswell
Session-Id: <bitswell-session-id>
Task-Status: ASSIGNED
Assigned-To: <agent-id>
Assignment: <assignment-id>
Scope: .
Scope-Denied: <paths, omit if none>
Dependencies: <agent/slug refs, comma-separated, or "none">
Budget: <integer>
```

### 6.2 Agent: Start (first commit)

```
chore(<scope>): begin <assignment description>

Agent-Id: <agent-id>
Session-Id: <session-id>
Task-Status: IMPLEMENTING
Heartbeat: <ISO-8601 UTC>
```

### 6.3 Agent: Work (intermediate commits)

```
<type>(<scope>): <subject>

<body>

Agent-Id: <agent-id>
Session-Id: <session-id>
Heartbeat: <ISO-8601 UTC>
```

### 6.4 Agent: Completion

```
<type>(<scope>): <subject>

<body summarizing what was accomplished>

Agent-Id: <agent-id>
Session-Id: <session-id>
Task-Status: COMPLETED
Files-Changed: <integer>
Key-Finding: <discovery>
Heartbeat: <ISO-8601 UTC>
```

### 6.5 Agent: Blocked

```
chore(<scope>): blocked -- <short reason>

<Detailed explanation>

Agent-Id: <agent-id>
Session-Id: <session-id>
Task-Status: BLOCKED
Blocked-Reason: <description>
Heartbeat: <ISO-8601 UTC>
```

### 6.6 Agent: Failed

```
chore(<scope>): failed -- <short reason>

<Detailed explanation>

Agent-Id: <agent-id>
Session-Id: <session-id>
Task-Status: FAILED
Error-Category: <category>
Error-Retryable: <true|false>
```

### 6.7 Orchestrator: Post-Terminal (hotfix/amendment)

```
chore(loom): <description of change>

<body>

Agent-Id: bitswell
Session-Id: <bitswell-session-id>
```

Note: No `Task-Status` trailer. This commit is outside the state machine.

---

## 7. State Extraction Queries

```bash
# Latest status of a branch
git log -1 --format='%(trailers:key=Task-Status,valueonly)' \
  --grep='Task-Status:' loom/<agent>-<slug>

# All findings from a completed agent
git log --format='%(trailers:key=Key-Finding,valueonly)' loom/<agent>-<slug> \
  | grep -v '^$'

# All decisions
git log --format='%(trailers:key=Decision,valueonly)' loom/<agent>-<slug> \
  | grep -v '^$'

# Check if a dependency is met
git log -1 --format='%(trailers:key=Task-Status,valueonly)' \
  --grep='Task-Status:' loom/<dep-agent>-<dep-slug>
# Result: "COMPLETED" means met

# Last heartbeat
git log -1 --format='%(trailers:key=Heartbeat,valueonly)' loom/<agent>-<slug>

# Full trailer dump for a branch
git log --format='%H %s%n%(trailers)%n---' loom/<agent>-<slug>

# Find all ASSIGNED branches (undispatched work)
for b in $(git branch --list 'loom/*' --format='%(refname:short)'); do
  s=$(git log -1 --format='%(trailers:key=Task-Status,valueonly)' \
    --grep='Task-Status:' "$b" | head -1 | xargs)
  [[ "$s" == "ASSIGNED" ]] && echo "$b"
done
```

---

## 8. Validation Rules

**Status: Not Yet Enforced.** These rules describe the target state for CI/validator tooling. They are normative requirements that will be checked once a validator exists. Until then, they serve as the compliance checklist for manual review.

### 8.1 Per-commit validation

1. Every commit MUST have `Agent-Id` and `Session-Id` trailers.
2. `Agent-Id` MUST match `[a-z0-9]+(-[a-z0-9]+)*` (kebab-case).
3. `Session-Id` MUST be a valid UUID v4.
4. If `Task-Status` is present, its value MUST be one of: `ASSIGNED`, `IMPLEMENTING`, `COMPLETED`, `BLOCKED`, `FAILED`.
5. Commits with `Task-Status: ASSIGNED` MUST also have `Assigned-To`, `Assignment`, `Scope`, `Dependencies`, and `Budget`.
6. Commits with `Task-Status: COMPLETED` MUST also have `Files-Changed` (integer >= 0) and at least one `Key-Finding`.
7. Commits with `Task-Status: BLOCKED` MUST also have `Blocked-Reason`.
8. Commits with `Task-Status: FAILED` MUST also have `Error-Category` and `Error-Retryable`.

### 8.2 Branch-level validation

9. The first commit on the branch MUST have `Task-Status: ASSIGNED` (from bitswell).
10. The agent's first commit MUST have `Task-Status: IMPLEMENTING`.
11. A branch MUST NOT have more than one `COMPLETED` or `FAILED` commit. These are terminal states.
12. After a terminal state, no further commits with `Task-Status` are permitted. Orchestrator post-terminal commits use `chore(loom):` with no `Task-Status`.
13. All agent commits on the branch MUST share the same `Session-Id`. The `ASSIGNED` commit carries bitswell's session ID.
14. `BLOCKED` is non-terminal. An agent MAY transition from `BLOCKED` back to `IMPLEMENTING`.

### 8.3 State machine

```
ASSIGNED --> IMPLEMENTING --> COMPLETED
                  |     ^
                  |     |
                  +---> BLOCKED
                  |
                  +---> FAILED
```

Valid transitions:
- `ASSIGNED` -> `IMPLEMENTING` (agent starts work)
- `IMPLEMENTING` -> `COMPLETED` (agent finishes)
- `IMPLEMENTING` -> `BLOCKED` (agent cannot proceed)
- `IMPLEMENTING` -> `FAILED` (unrecoverable error)
- `BLOCKED` -> `IMPLEMENTING` (blocker resolved)

Invalid transitions (MUST reject):
- Any state -> `ASSIGNED` (assignment happens once)
- `COMPLETED` -> any state (terminal)
- `FAILED` -> any state (terminal)
- `BLOCKED` -> `COMPLETED` (must resume `IMPLEMENTING` first)
- `BLOCKED` -> `FAILED` (must resume `IMPLEMENTING` first)

---

*End of LOOM Schemas Reference v2.0.0-draft.*
