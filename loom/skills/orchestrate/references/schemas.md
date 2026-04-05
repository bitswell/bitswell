# LOOM Schemas Reference

**Version**: 2.0.0 | **Protocol**: `loom/2` | **Status**: Active

Defines commit message format, branch naming, trailer vocabulary, state requirements, commit templates, extraction queries, and validation rules.

See `protocol.md` for the lifecycle state machine and operations. See `mcagent-spec.md` for agent conformance rules.

---

## 1. Commit Message Format

All agent and orchestrator commits MUST use Conventional Commits with required trailers.

```
<type>(<scope>): <subject>

<body — optional, explains "why" not "what">

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

## 2. Branch Naming Convention

**Pattern:** `loom/<agent>-<slug>`

The branch name encodes the agent and assignment slug. Dependencies use `<agent>/<slug>` format. To resolve: replace `/` with `-`, prepend `loom/`.

| Dependency | Branch |
|-----------|--------|
| `ratchet/commit-schema` | `loom/ratchet-commit-schema` |
| `moss/migrate-identities` | `loom/moss-migrate-identities` |

**Constraints:**
- Kebab-case: `[a-z0-9]+(-[a-z0-9]+)*`
- Maximum length: 63 characters
- One agent per branch, one branch per assignment

---

## 3. Trailer Vocabulary

All trailers follow `git-interpret-trailers(1)` syntax.

### 3.1 Universal trailers (every commit)

| Trailer | Type | Description |
|---------|------|-------------|
| `Agent-Id` | string | Agent name (e.g., `ratchet`, `bitswell`). Kebab-case. |
| `Session-Id` | string | UUID v4. Unique per agent invocation. ASSIGNED commit carries orchestrator's session. |

### 3.2 State trailers

| Trailer | Type | Description |
|---------|------|-------------|
| `Task-Status` | enum | One of: `ASSIGNED`, `IMPLEMENTING`, `COMPLETED`, `BLOCKED`, `FAILED` |
| `Heartbeat` | string | ISO-8601 UTC timestamp. SHOULD appear on every commit while agent is running. |

### 3.3 Assignment trailers (ASSIGNED commits only)

| Trailer | Type | Description |
|---------|------|-------------|
| `Assigned-To` | string | Agent-id of the assignee. |
| `Assignment` | string | Assignment slug (e.g., `plugin-scaffold`). |
| `Scope` | string | Allowed paths (e.g., `loom/skills/**`). |
| `Scope-Denied` | string | Denied paths. OPTIONAL. Omit if none. |
| `Dependencies` | string | Comma-separated `<agent>/<slug>` refs or `none`. |
| `Budget` | integer | Token budget. |

### 3.4 Completion trailers (COMPLETED commits)

| Trailer | Type | Description |
|---------|------|-------------|
| `Files-Changed` | integer | Files modified (>= 0). REQUIRED. |
| `Key-Finding` | string | Important discovery. Repeatable. At least one REQUIRED. |
| `Decision` | string | Non-obvious choice, format `<what> -- <why>`. Repeatable. OPTIONAL. |
| `Deviation` | string | Departure from task spec, format `<what> -- <why>`. Repeatable. OPTIONAL. |

### 3.5 Error trailers (BLOCKED and FAILED commits)

| Trailer | Type | Description |
|---------|------|-------------|
| `Blocked-Reason` | string | What is preventing progress. REQUIRED on BLOCKED. |
| `Error-Category` | enum | `task_unclear`, `blocked`, `resource_limit`, `conflict`, `internal`. REQUIRED on FAILED. |
| `Error-Retryable` | boolean | `true` or `false`. REQUIRED on FAILED. |

---

## 4. Required Trailers Per State

### 4.1 ASSIGNED (orchestrator writes)

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

### 4.2 IMPLEMENTING (agent writes — first commit)

| Trailer | Required |
|---------|----------|
| `Agent-Id` | yes |
| `Session-Id` | yes |
| `Task-Status` | yes — value `IMPLEMENTING` |
| `Heartbeat` | yes |

### 4.3 COMPLETED (agent writes)

| Trailer | Required |
|---------|----------|
| `Agent-Id` | yes |
| `Session-Id` | yes |
| `Task-Status` | yes — value `COMPLETED` |
| `Files-Changed` | yes |
| `Key-Finding` | yes (at least one) |
| `Heartbeat` | yes |

### 4.4 BLOCKED (agent writes)

| Trailer | Required |
|---------|----------|
| `Agent-Id` | yes |
| `Session-Id` | yes |
| `Task-Status` | yes — value `BLOCKED` |
| `Blocked-Reason` | yes |
| `Heartbeat` | yes |

### 4.5 FAILED (agent writes)

| Trailer | Required |
|---------|----------|
| `Agent-Id` | yes |
| `Session-Id` | yes |
| `Task-Status` | yes — value `FAILED` |
| `Error-Category` | yes |
| `Error-Retryable` | yes |

---

## 5. Commit Templates

### 5.1 Orchestrator: Task Assignment

```
task(<agent-id>): <short task description>

<Full task description. Include objective, context, acceptance criteria.>

Agent-Id: bitswell
Session-Id: <bitswell-session-id>
Task-Status: ASSIGNED
Assigned-To: <agent-id>
Assignment: <slug>
Scope: <paths>
Scope-Denied: <paths, omit if none>
Dependencies: <agent/slug refs, comma-separated, or "none">
Budget: <integer>
```

### 5.2 Agent: Start (first commit)

```
chore(<scope>): begin <assignment description>

Agent-Id: <agent-id>
Session-Id: <session-id>
Task-Status: IMPLEMENTING
Heartbeat: <ISO-8601 UTC>
```

### 5.3 Agent: Work (intermediate commits)

```
<type>(<scope>): <subject>

<body>

Agent-Id: <agent-id>
Session-Id: <session-id>
Heartbeat: <ISO-8601 UTC>
```

### 5.4 Agent: Completion

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

### 5.5 Agent: Blocked

```
chore(<scope>): blocked -- <short reason>

<Detailed explanation>

Agent-Id: <agent-id>
Session-Id: <session-id>
Task-Status: BLOCKED
Blocked-Reason: <description>
Heartbeat: <ISO-8601 UTC>
```

### 5.6 Agent: Failed

```
chore(<scope>): failed -- <short reason>

<Detailed explanation>

Agent-Id: <agent-id>
Session-Id: <session-id>
Task-Status: FAILED
Error-Category: <category>
Error-Retryable: <true|false>
```

### 5.7 Orchestrator: Post-Terminal (hotfix/amendment)

```
chore(loom): <description of change>

<body>

Agent-Id: bitswell
Session-Id: <bitswell-session-id>
```

Note: No `Task-Status` trailer. This commit is outside the state machine.

---

## 6. State Extraction Queries

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

## 7. Validation Rules

### 7.1 Per-commit validation

1. Every commit MUST have `Agent-Id` and `Session-Id` trailers.
2. `Agent-Id` MUST match `[a-z0-9]+(-[a-z0-9]+)*` (kebab-case).
3. `Session-Id` MUST be a valid UUID v4.
4. If `Task-Status` is present, its value MUST be one of: `ASSIGNED`, `IMPLEMENTING`, `COMPLETED`, `BLOCKED`, `FAILED`.
5. Commits with `Task-Status: ASSIGNED` MUST also have `Assigned-To`, `Assignment`, `Scope`, `Dependencies`, and `Budget`.
6. Commits with `Task-Status: COMPLETED` MUST also have `Files-Changed` (integer >= 0) and at least one `Key-Finding`.
7. Commits with `Task-Status: BLOCKED` MUST also have `Blocked-Reason`.
8. Commits with `Task-Status: FAILED` MUST also have `Error-Category` and `Error-Retryable`.

### 7.2 Branch-level validation

9. The first commit on the branch MUST have `Task-Status: ASSIGNED` (from bitswell).
10. The agent's first commit MUST have `Task-Status: IMPLEMENTING`.
11. A branch MUST NOT have more than one `COMPLETED` or `FAILED` commit. These are terminal states.
12. After a terminal state, no further commits with `Task-Status` are permitted. Orchestrator post-terminal commits use `chore(loom):` with no `Task-Status`.
13. All commits from a single agent invocation MUST share the same `Session-Id`. The `ASSIGNED` commit carries bitswell's session ID. A new agent invocation resuming a BLOCKED branch uses a new `Session-Id`.
14. `BLOCKED` is non-terminal. An agent MAY transition from `BLOCKED` back to `IMPLEMENTING`.

### 7.3 State machine

```
ASSIGNED --> IMPLEMENTING --> COMPLETED
                  |     ^
                  |     |
                  +---> BLOCKED --+
                  |               |
                  +---> FAILED <--+
```

Valid transitions:
- `ASSIGNED` -> `IMPLEMENTING` (agent starts work)
- `IMPLEMENTING` -> `COMPLETED` (agent finishes)
- `IMPLEMENTING` -> `BLOCKED` (agent cannot proceed)
- `IMPLEMENTING` -> `FAILED` (unrecoverable error)
- `BLOCKED` -> `IMPLEMENTING` (blocker resolved, agent resumes)
- `BLOCKED` -> `FAILED` (orchestrator timeout only — not agent-initiated)

Invalid transitions (MUST reject):
- Any state -> `ASSIGNED` (assignment happens once)
- `COMPLETED` -> any state (terminal)
- `FAILED` -> any state (terminal)
- `BLOCKED` -> `COMPLETED` (must resume `IMPLEMENTING` first)

---

*End of LOOM Schemas Reference v2.0.0.*
