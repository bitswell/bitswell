# LOOM File Format Schemas

Reference for all file formats used by the LOOM protocol (version `loom/1`).
Two independent implementations reading this document MUST produce interoperable artifacts.

---

## 1. TASK.md Template

Written by the orchestrator. Immutable after agent spawn.

```markdown
# Task: <short title>

## Objective
<What the agent must accomplish. One paragraph.>

## Context
<Background the agent needs: relevant code, prior decisions, links to specs.>

## Scope
- **Allowed paths**: <glob list, e.g. `src/config/**`, `tests/config/**`>
- **Denied paths**: <glob list, e.g. `src/auth/**`>

## Acceptance Criteria
- [ ] <Criterion 1 -- measurable, verifiable>
- [ ] <Criterion 2>

## Dependencies
- <agent-id that must complete before this agent can be integrated, or "none">

## Constraints
- **Token budget**: <integer, e.g. 100000>
- **Timeout**: <seconds, e.g. 3600>
```

All sections are REQUIRED. The orchestrator MAY append a `## Feedback` section after spawn to communicate plan-review results; the original sections remain unchanged.

---

## 2. AGENT.json Schema

Written by the orchestrator at spawn. Read-only for the agent.

```json
{
  "agent_id":              "<string, unique, kebab-case, e.g. \"config-parser\">",
  "session_id":            "<string, UUID v4, unique per invocation>",
  "protocol_version":      "loom/1",
  "context_window_tokens": "<integer, > 0, model context size in tokens>",
  "token_budget":          "<integer, > 0, max tokens the agent may consume>",
  "dependencies":          ["<agent-id>"],
  "scope": {
    "paths_allowed":       ["<glob>"],
    "paths_denied":        ["<glob>"]
  },
  "timeout_seconds":       "<integer, > 0, default 3600>"
}
```

**Field constraints:**

| Field | Type | Required | Constraints |
|-------|------|----------|-------------|
| `agent_id` | string | yes | Kebab-case (`[a-z0-9]+(-[a-z0-9]+)*`), unique within the repository |
| `session_id` | string | yes | UUID v4, unique per agent invocation |
| `protocol_version` | string | yes | Literal `"loom/1"` |
| `context_window_tokens` | integer | yes | Positive integer |
| `token_budget` | integer | yes | Positive integer, <= `context_window_tokens` |
| `dependencies` | string[] | yes | Array of valid `agent_id` values (may be empty `[]`). Graph MUST be a DAG. |
| `scope.paths_allowed` | string[] | yes | Non-empty array of globs relative to repo root |
| `scope.paths_denied` | string[] | yes | Array of globs (may be empty `[]`). Deny takes precedence over allow. |
| `timeout_seconds` | integer | yes | Positive integer. Default: 3600 |

**Example (filled):**

```json
{
  "agent_id": "config-parser",
  "session_id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
  "protocol_version": "loom/1",
  "context_window_tokens": 200000,
  "token_budget": 150000,
  "dependencies": [],
  "scope": {
    "paths_allowed": ["src/config/**", "tests/config/**"],
    "paths_denied": ["src/config/secrets.rs"]
  },
  "timeout_seconds": 3600
}
```

---

## 3. STATUS.md YAML Schema

> **Level 1 only.** At Level 2+, protocol state moves to commit trailers (Section 8). STATUS.md is not used.

YAML front matter delimited by `---`. Parsers MUST treat the block as YAML, not freeform text. Markdown body after the closing `---` is permitted but not parsed by the protocol.

**Full field listing:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `status` | enum | always | One of: `PLANNING`, `IMPLEMENTING`, `COMPLETED`, `BLOCKED`, `FAILED` |
| `updated_at` | string | always | ISO-8601 UTC timestamp (e.g. `2026-04-02T14:30:00Z`) |
| `heartbeat_at` | string | always | ISO-8601 UTC timestamp. MUST be updated and committed at least every 5 minutes while the agent is running. |
| `branch` | string | always | The agent's branch name (e.g. `loom/config-parser`) |
| `base_commit` | string | always | SHA of the commit the agent's worktree was created from |
| `files_changed` | integer | when `COMPLETED` | Number of files modified by the agent |
| `summary` | string | always | One-line human-readable description of current state |
| `error` | object | when `FAILED` | See error sub-fields below |
| `error.category` | enum | when `FAILED` | One of: `task_unclear`, `blocked`, `resource_limit`, `conflict`, `internal` |
| `error.message` | string | when `FAILED` | Human-readable detail |
| `error.retryable` | boolean | when `FAILED` | Whether the orchestrator may retry |
| `blocked_reason` | string | when `BLOCKED` | Why the agent cannot proceed |
| `budget` | object | Level 2+ | Token/cost tracking |
| `budget.tokens_used` | integer | Level 2+ | Tokens consumed so far |
| `budget.tokens_limit` | integer | Level 2+ | Token ceiling from AGENT.json |
| `budget.cost_usd` | float | optional | Estimated monetary cost |

**Validation rules:**
- `error` block is REQUIRED when `status` is `FAILED`; MUST NOT be present otherwise.
- `blocked_reason` is REQUIRED when `status` is `BLOCKED`; MUST NOT be present otherwise.
- `files_changed` is REQUIRED when `status` is `COMPLETED`.
- `budget` block is REQUIRED at Level 2+; optional at Level 1.

**Example -- PLANNING state:**

```yaml
---
status: PLANNING
updated_at: "2026-04-02T14:00:00Z"
heartbeat_at: "2026-04-02T14:00:00Z"
branch: loom/config-parser
base_commit: abc1234def5678
summary: Drafting implementation plan
---
```

**Example -- COMPLETED state:**

```yaml
---
status: COMPLETED
updated_at: "2026-04-02T15:12:00Z"
heartbeat_at: "2026-04-02T15:12:00Z"
branch: loom/config-parser
base_commit: abc1234def5678
files_changed: 4
summary: Config parser implemented with tests
budget:
  tokens_used: 87000
  tokens_limit: 150000
  cost_usd: 0.42
---
```

**Example -- FAILED state:**

```yaml
---
status: FAILED
updated_at: "2026-04-02T14:45:00Z"
heartbeat_at: "2026-04-02T14:45:00Z"
branch: loom/config-parser
base_commit: abc1234def5678
summary: Token budget exhausted before completion
error:
  category: resource_limit
  message: "Consumed 90% of token budget during implementation phase"
  retryable: true
---
```

---

## 4. MEMORY.md Template

> **Level 1 only.** At Level 2+, memory is encoded as commit trailers (Section 8.5). MEMORY.md is not used.

Written by the agent. MUST be updated incrementally during work, not only at completion. Serves as the checkpoint for context-window recovery: an agent whose context is compacted re-reads TASK.md, AGENT.json, and MEMORY.md.

```markdown
## Key Findings
- <Facts discovered during work that downstream agents or the orchestrator need.>
- <E.g., "The config module uses TOML, not YAML as documented.">

## Decisions
- **<Decision>**: <Rationale>
- **<E.g., Used serde_derive over manual impl>**: <Reduces boilerplate, all fields are simple types.>

## Deviations from Plan
- <What changed from PLAN.md and why.>
- <E.g., "Added src/config/defaults.rs (not in plan) to hold fallback values.">
```

All three sections are REQUIRED. Sections may be empty (`- (none yet)`) during early work but MUST contain substantive entries before the agent sets status to `COMPLETED`.

---

## 5. PLAN.md Template

> **Level 1 only.** At Level 2+, the plan is the commit body of the agent's first work commit. PLAN.md is not used.

Written by the agent during the PLANNING state. MUST be committed before the agent can transition to IMPLEMENTING.

```markdown
# Plan: <task title>

## Approach
<High-level strategy in 2-3 sentences.>

## Steps
1. <Step with concrete action, e.g. "Create `src/config/parser.rs` with public `parse()` function">
2. <Next step>
3. ...

## Files to Modify
- `<path>` -- <what changes and why>
- `<path>` -- <what changes and why>

## Risks
- <What could go wrong and how you will handle it.>

## Estimated Effort
- **Tokens**: <estimated token usage, e.g. ~80,000>
- **Files**: <count of files to create or modify>
```

All sections are REQUIRED. The orchestrator reviews this file before approving the transition to IMPLEMENTING.

---

## 6. Commit Message Format

All agent commits MUST use Conventional Commits format with trailers. This section defines the base format. Section 8 defines the full trailer vocabulary per state.

**Base format:**

```
<type>(<scope>): <subject>

<body -- optional, explains "why" not "what">

Agent-Id: <agent-id>
Session-Id: <session-id, UUID v4>
```

**Type values:** `task`, `feat`, `fix`, `docs`, `refactor`, `test`, `chore`

- `task` -- orchestrator task-assignment commits (new in v2). Not used by agents.
- `feat`, `fix`, `docs`, `refactor`, `test`, `chore` -- standard Conventional Commits types, used by agents.

**Trailer rules:**

- Trailers follow the git trailer convention: `Key: Value`, one per line, separated from the body by a blank line.
- `Agent-Id` and `Session-Id` are REQUIRED on every commit (orchestrator and agent).
- Additional trailers are defined per state in Section 8.
- Trailer keys are case-sensitive. Use the exact casing specified.
- Multi-value trailers: repeat the key on separate lines (e.g., multiple `Key-Finding:` trailers).

**Good example:**

```
feat(config): add TOML parser with validation

Supports nested tables and type coercion for string-to-int fields.

Agent-Id: config-parser
Session-Id: a1b2c3d4-e5f6-7890-abcd-ef1234567890
Task-Status: IMPLEMENTING
```

**Bad example (missing trailers, vague subject):**

```
update stuff
```

---

## 7. Branch Naming Convention

**Pattern:** `loom/<agent-id>` or `loom/<agent-id>-<assignment>`

**Examples:**
- `loom/config-parser`
- `loom/ratchet-commit-schema`

**agent-id constraints:**
- Kebab-case: matches `[a-z0-9]+(-[a-z0-9]+)*`
- Unique within the repository at any given time
- Maximum length: 63 characters (git ref component limit)

Each agent works exclusively on its own branch. The orchestrator creates the branch at worktree creation time, branching from the current workspace HEAD. Agents MUST NOT push to or modify any other branch.

---

## 8. Commit-Based Protocol

> **Applies at Level 2+.** Replaces STATUS.md, PLAN.md, and MEMORY.md with commit message trailers. All protocol state is encoded in commits. The worktree contains only deliverable code.

### 8.1 Design Principles

1. **Git is the protocol.** Every state transition is a commit. No protocol files in the worktree.
2. **Trailers are structured.** Parseable by `git log --format='%(trailers)'` and `git log --format='%(trailers:key=Task-Status)'`.
3. **Commits are append-only.** State is determined by the latest commit with a `Task-Status` trailer, not by mutation.
4. **Backward-compatible.** Level 1 file-based protocol remains valid. Level 2 commit-based protocol is a strict superset.

### 8.2 Trailer Vocabulary

All trailers beyond `Agent-Id` and `Session-Id` are OPTIONAL unless marked REQUIRED for a specific state.

| Trailer | Type | Description |
|---------|------|-------------|
| `Agent-Id` | string | Agent identifier. REQUIRED on every commit. |
| `Session-Id` | string | UUID v4, unique per agent invocation. REQUIRED on every commit. |
| `Task-Status` | enum | One of: `ASSIGNED`, `IMPLEMENTING`, `COMPLETED`, `BLOCKED`, `FAILED`. Present on state-transition commits. |
| `Assigned-To` | string | Agent-id of the assignee. REQUIRED on `ASSIGNED` commits. |
| `Assignment` | string | Assignment identifier (e.g., `2-commit-schema`). REQUIRED on `ASSIGNED` commits. |
| `Scope` | string | Glob or path list defining allowed scope. REQUIRED on `ASSIGNED` commits. In v2, typically `"."` (agent owns its worktree). |
| `Scope-Denied` | string | Glob or path list of denied paths within scope. OPTIONAL on `ASSIGNED` commits. Omit if no denials. |
| `Dependencies` | string | Comma-separated `<agent>/<assignment>` refs or `none`. REQUIRED on `ASSIGNED` commits. |
| `Budget` | integer | Token budget for the task. REQUIRED on `ASSIGNED` commits. |
| `Files-Changed` | integer | Number of files modified. REQUIRED on `COMPLETED` commits. |
| `Key-Finding` | string | A fact discovered during work. Repeatable. REQUIRED on `COMPLETED` commits (at least one). |
| `Decision` | string | A non-obvious choice made during work, format: `<what> -- <why>`. Repeatable. OPTIONAL. |
| `Deviation` | string | A departure from the original task spec, format: `<what> -- <why>`. Repeatable. OPTIONAL. |
| `Blocked-Reason` | string | What is preventing progress. REQUIRED on `BLOCKED` commits. |
| `Error-Category` | enum | One of: `task_unclear`, `blocked`, `resource_limit`, `conflict`, `internal`. REQUIRED on `FAILED` commits. |
| `Error-Retryable` | boolean | `true` or `false`. REQUIRED on `FAILED` commits. |

### 8.3 Required Trailers Per State

#### ASSIGNED (orchestrator writes)

| Trailer | Required |
|---------|----------|
| `Agent-Id` | yes |
| `Session-Id` | yes |
| `Task-Status` | yes -- value `ASSIGNED` |
| `Assigned-To` | yes |
| `Assignment` | yes |
| `Scope` | yes |
| `Dependencies` | yes |
| `Budget` | yes |

#### IMPLEMENTING (agent writes)

| Trailer | Required |
|---------|----------|
| `Agent-Id` | yes |
| `Session-Id` | yes |
| `Task-Status` | yes -- value `IMPLEMENTING` |

The agent's first commit MUST carry `Task-Status: IMPLEMENTING`. Subsequent work commits MAY omit `Task-Status` (the state is inherited from the most recent state-transition commit).

#### COMPLETED (agent writes)

| Trailer | Required |
|---------|----------|
| `Agent-Id` | yes |
| `Session-Id` | yes |
| `Task-Status` | yes -- value `COMPLETED` |
| `Files-Changed` | yes |
| `Key-Finding` | yes (at least one) |
| `Decision` | no (recommended if non-obvious choices were made) |
| `Deviation` | no (required if the implementation diverged from the task) |

#### BLOCKED (agent writes)

| Trailer | Required |
|---------|----------|
| `Agent-Id` | yes |
| `Session-Id` | yes |
| `Task-Status` | yes -- value `BLOCKED` |
| `Blocked-Reason` | yes |

#### FAILED (agent writes)

| Trailer | Required |
|---------|----------|
| `Agent-Id` | yes |
| `Session-Id` | yes |
| `Task-Status` | yes -- value `FAILED` |
| `Error-Category` | yes |
| `Error-Retryable` | yes |

### 8.4 Commit Templates

#### Orchestrator: Task Assignment

```
task(<agent-id>): <short task description>

<Full task description. This replaces TASK.md.
Include objective, context, acceptance criteria.
The commit body IS the task specification.>

Agent-Id: bitswell
Session-Id: <bitswell-session-id>
Task-Status: ASSIGNED
Assigned-To: <agent-id>
Assignment: <assignment-id>
Scope: .
Scope-Denied: <glob-or-path-list, omit if none>
Dependencies: <agent/assignment refs, comma-separated, or "none">
Budget: <integer>
```

#### Agent: Start (first commit)

```
<type>(<scope>): <what this commit does>

<Optional body.>

Agent-Id: <agent-id>
Session-Id: <session-id>
Task-Status: IMPLEMENTING
```

#### Agent: Work Commits (intermediate)

```
<type>(<scope>): <what this commit does>

<Optional body.>

Agent-Id: <agent-id>
Session-Id: <session-id>
```

Work commits do not require `Task-Status`. The agent is implicitly in `IMPLEMENTING` state until a terminal trailer appears. Work commits MAY include `Key-Finding`, `Decision`, or `Deviation` trailers if the agent discovers something worth recording mid-task.

#### Agent: Completion

```
<type>(<scope>): <what this final commit does>

<Summary of what was accomplished.>

Agent-Id: <agent-id>
Session-Id: <session-id>
Task-Status: COMPLETED
Files-Changed: <integer>
Key-Finding: <finding-1>
Key-Finding: <finding-2>
Decision: <choice -- rationale>
Deviation: <change -- reason>
```

#### Agent: Blocked

```
chore(<scope>): blocked -- <short reason>

<Detailed explanation of what is blocking progress.>

Agent-Id: <agent-id>
Session-Id: <session-id>
Task-Status: BLOCKED
Blocked-Reason: <description of blocker>
```

#### Agent: Failed

```
chore(<scope>): failed -- <short reason>

<Detailed explanation of what went wrong.>

Agent-Id: <agent-id>
Session-Id: <session-id>
Task-Status: FAILED
Error-Category: <task_unclear|blocked|resource_limit|conflict|internal>
Error-Retryable: <true|false>
```

### 8.5 Memory and Findings Mapping

In Level 1, agents recorded discoveries in MEMORY.md under three sections: Key Findings, Decisions, and Deviations. In Level 2, these map directly to trailers.

| MEMORY.md Section | Trailer | When to Write |
|-------------------|---------|---------------|
| Key Findings | `Key-Finding: <text>` | On any commit where the agent discovers a fact relevant to downstream agents or the orchestrator. REQUIRED on `COMPLETED` commit. |
| Decisions | `Decision: <what> -- <why>` | On the commit where the decision is made. Recommended on `COMPLETED` commit if not already recorded. |
| Deviations from Plan | `Deviation: <what> -- <why>` | On the commit where the deviation occurs. Required on `COMPLETED` commit if the implementation diverged from the task specification. |

**Accumulation rule:** To reconstruct the full memory for an agent session, collect all `Key-Finding`, `Decision`, and `Deviation` trailers across all commits with matching `Session-Id`. The `COMPLETED` commit's trailers are the canonical summary; earlier trailers provide incremental context.

**Query to extract all findings for a session:**

```bash
git log --format='%(trailers:key=Key-Finding,valueonly)' \
  --grep='Session-Id: <session-id>' <branch>
```

### 8.6 State Extraction Queries

These `git log` commands extract protocol state from commit history. All queries assume the agent's branch.

#### Current task status

```bash
git log -1 --format='%(trailers:key=Task-Status,valueonly)' \
  --grep='Task-Status:' <branch>
```

#### Task assignment details

```bash
git log -1 --format='%B' --grep='Task-Status: ASSIGNED' <branch>
```

#### All state transitions for a session

```bash
git log --format='%h %s | %(trailers:key=Task-Status,valueonly)' \
  --grep='Session-Id: <session-id>' <branch>
```

#### All key findings for a session

```bash
git log --format='%(trailers:key=Key-Finding,valueonly)' \
  --grep='Session-Id: <session-id>' <branch> | grep -v '^$'
```

#### All decisions for a session

```bash
git log --format='%(trailers:key=Decision,valueonly)' \
  --grep='Session-Id: <session-id>' <branch> | grep -v '^$'
```

#### All deviations for a session

```bash
git log --format='%(trailers:key=Deviation,valueonly)' \
  --grep='Session-Id: <session-id>' <branch> | grep -v '^$'
```

#### Files changed count from completion commit

```bash
git log -1 --format='%(trailers:key=Files-Changed,valueonly)' \
  --grep='Task-Status: COMPLETED' <branch>
```

#### List all active agents (branches with IMPLEMENTING, no terminal state)

```bash
for branch in $(git branch -a --list 'loom/*' --format='%(refname:short)'); do
  status=$(git log -1 --format='%(trailers:key=Task-Status,valueonly)' \
    --grep='Task-Status:' "$branch")
  if [ "$status" = "IMPLEMENTING" ]; then
    agent=$(git log -1 --format='%(trailers:key=Agent-Id,valueonly)' \
      --grep='Task-Status:' "$branch")
    echo "$agent on $branch"
  fi
done
```

### 8.7 Validation Rules

Orchestrators and CI systems MUST validate the following before integrating an agent's branch.

#### Per-commit validation

1. Every commit MUST have `Agent-Id` and `Session-Id` trailers.
2. `Agent-Id` MUST match `[a-z0-9]+(-[a-z0-9]+)*` (kebab-case). The orchestrator uses its own agent name (e.g., `bitswell`), not a special literal.
3. `Session-Id` MUST be a valid UUID v4.
4. If `Task-Status` is present, its value MUST be one of: `ASSIGNED`, `IMPLEMENTING`, `COMPLETED`, `BLOCKED`, `FAILED`.
5. Commits with `Task-Status: ASSIGNED` MUST also have `Assigned-To`, `Assignment`, `Scope`, `Dependencies`, and `Budget`.
6. Commits with `Task-Status: COMPLETED` MUST also have `Files-Changed` (integer >= 0) and at least one `Key-Finding`.
7. Commits with `Task-Status: BLOCKED` MUST also have `Blocked-Reason`.
8. Commits with `Task-Status: FAILED` MUST also have `Error-Category` (valid enum value) and `Error-Retryable` (`true` or `false`).

#### Branch-level validation

9. The first commit on the branch MUST have `Task-Status: ASSIGNED` (from the orchestrator).
10. The agent's first commit MUST have `Task-Status: IMPLEMENTING`.
11. A branch MUST NOT have more than one `COMPLETED` or `FAILED` commit. These are terminal states.
12. After a terminal state (`COMPLETED` or `FAILED`), no further commits with `Task-Status` are permitted.
13. All commits on the branch MUST share the same `Session-Id`, except the `ASSIGNED` commit which carries bitswell's session ID.
14. `BLOCKED` is non-terminal. An agent MAY transition from `BLOCKED` back to `IMPLEMENTING`.

#### State machine

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
- `IMPLEMENTING` -> `FAILED` (agent encountered unrecoverable error)
- `BLOCKED` -> `IMPLEMENTING` (blocker resolved, agent resumes)

Invalid transitions (MUST reject):
- Any state -> `ASSIGNED` (assignment happens once)
- `COMPLETED` -> any state (terminal)
- `FAILED` -> any state (terminal)
- `BLOCKED` -> `COMPLETED` (must resume `IMPLEMENTING` first)
- `BLOCKED` -> `FAILED` (must resume `IMPLEMENTING` first)