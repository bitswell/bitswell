# `.mcagent/` Directory Specification

**Version**: 2.0.0-draft | **Protocol**: `loom/2` | **Status**: Draft

---

## 1. Purpose

Separate agent state from the codebase. The `.mcagent/` directory is the operational root for all agent activity. It contains identity, assignment metadata, and worktrees — but never protocol files inside deliverable code.

---

## 2. Three-Layer Ontology

Every piece of agent state belongs to exactly one layer.

| Layer | Lifecycle | Contents | Location |
|-------|-----------|----------|----------|
| **Identity** | Persistent across all assignments | Agent persona, values, accumulated skill | `.mcagent/agents/<name>/identity.md` |
| **Assignment** | Per-task, survives restarts | Agent config, scope, budget, dependencies | `.mcagent/agents/<name>/<assignment>/AGENT.json` |
| **Worktree** | Ephemeral, contains only deliverable code | Git worktree of target repo | `.mcagent/agents/<name>/<assignment>/worktree/` |

The layers are strictly nested. Identity outlives any assignment. An assignment outlives any single worktree session. A worktree contains nothing that is not intended for the target repository.

---

## 3. Directory Layout

```
.mcagent/
  agents/
    <agent-name>/
      identity.md                          # Persistent identity (Layer 1)
      <sequence>-<slug>/                   # Assignment directory (Layer 2)
        AGENT.json                         # Assignment config
        worktree/                          # Git worktree of target repo (Layer 3)
```

### 3.1 Agent Name

A lowercase alphanumeric string. No spaces, no underscores. Examples: `ratchet`, `moss`, `drift`, `vesper`.

Agent names are stable across the lifetime of the `.mcagent/` installation. They are proper nouns — they refer to a specific agent with a specific identity.

### 3.2 Identity File

`identity.md` is a Markdown file describing the agent's persistent traits, voice, and operational tendencies. It is read by the orchestrator when spawning the agent and by the agent itself for self-reference.

Identity files are NOT generated per-assignment. They evolve slowly through deliberate updates, not through task execution.

### 3.3 Assignment Directory

Named `<sequence>-<slug>` where:

- `<sequence>` is a monotonically increasing integer (1, 2, 3, ...), unique within the agent. No zero-padding.
- `<slug>` is a lowercase kebab-case descriptor. For single-repo work, use a task descriptor (e.g., `1-mcagent-spec`, `2-fix-auth-middleware`). For multi-repo work, include the repo identifier (e.g., `27-bitswell-bitswell`, `42-acme-frontend`).

The sequence provides ordering. The slug provides human readability. Together they form a unique assignment identifier within the agent's namespace.

### 3.4 AGENT.json

Lives inside the assignment directory, **outside** the worktree. This is the critical distinction — AGENT.json is never committed to the agent's code branch.

```json
{
  "agent_id": "<agent-name>",
  "assignment_id": "<sequence>-<slug>",
  "session_id": "<uuid>",
  "protocol_version": "loom/2",
  "repo": "<org>/<repo>",
  "base_ref": "<branch-or-sha>",
  "context_window_tokens": 1000000,
  "token_budget": 200000,
  "dependencies": [],
  "scope": {
    "paths_allowed": ["."],
    "paths_denied": []
  },
  "timeout_seconds": 3600,
  "dispatch": {
    "mode": "sync|push-event",
    "trigger_ref": "<branch, if push-event>"
  }
}
```

**Field constraints:**

| Field | Type | Required | Constraints |
|-------|------|----------|-------------|
| `agent_id` | string | yes | Kebab-case (`[a-z0-9]+(-[a-z0-9]+)*`), unique within the repository |
| `assignment_id` | string | yes | Matches the assignment directory name |
| `session_id` | string | yes | UUID v4, unique per agent invocation |
| `protocol_version` | string | yes | Literal `"loom/2"` |
| `repo` | string | yes | Target repository in `<org>/<repo>` format |
| `base_ref` | string | yes | Branch name or commit SHA the worktree was created from |
| `context_window_tokens` | integer | yes | Positive integer |
| `token_budget` | integer | yes | Positive integer, <= `context_window_tokens` |
| `dependencies` | string[] | yes | Array of `<agent>/<slug>` refs (may be empty `[]`). Graph MUST be a DAG. |
| `scope.paths_allowed` | string[] | yes | Non-empty array of globs relative to worktree root |
| `scope.paths_denied` | string[] | yes | Array of globs (may be empty `[]`). Deny takes precedence over allow. |
| `timeout_seconds` | integer | yes | Positive integer. Default: 3600 |
| `dispatch.mode` | string | yes | `"sync"` or `"push-event"` |
| `dispatch.trigger_ref` | string | conditional | Required when `dispatch.mode` is `"push-event"` |

### 3.5 Worktree

A git worktree created via `git worktree add`. Points to the target repository, branching from `base_ref`.

**The worktree contains ONLY deliverable code.** No protocol files of any kind. These artifacts either live in the assignment directory (AGENT.json) or in commit messages (protocol state).

The worktree path is always `.mcagent/agents/<name>/<assignment>/worktree/`. Implementations MUST NOT place the worktree elsewhere.

**PWD convention**: The dispatch mechanism MUST `cd` into the worktree before spawning the agent. From the agent's perspective, its working directory is the worktree root — it sees a normal git checkout. The agent does not need to know about `.mcagent/` structure. Scope paths in AGENT.json (e.g., `"."`) are relative to the worktree root because that is the agent's PWD.

### 3.6 Orchestrator Identity

The orchestrator (`bitswell`) MUST have its own agent directory at `.mcagent/agents/bitswell/` with:
- `identity.md` — persistent orchestrator identity
- `orchestrator.json` — declares orchestrator role and `.mcagent/**` write scope

All orchestrator commits use `Agent-Id: bitswell`. There is no separate "orchestrator" identity — the orchestrator is an agent with elevated privileges.

```json
{
  "agent_id": "bitswell",
  "role": "orchestrator",
  "protocol_version": "loom/2",
  "scope": {
    "paths_allowed": [".mcagent/**"],
    "paths_denied": []
  },
  "capabilities": [
    "create-assignments",
    "write-agent-json",
    "create-worktrees",
    "manage-branches",
    "integrate-work",
    "dispatch-agents"
  ]
}
```

Only the orchestrator writes to `.mcagent/`. Agents write only within their worktree (their PWD).

### 3.7 Branch Naming

**Pattern:** `loom/<agent>-<slug>`

The branch name encodes the agent and assignment. The `<agent>` segment is the agent name from Section 3.1. The `<slug>` segment is the slug from the assignment directory name (Section 3.3), without the sequence number.

**Examples:**
- Agent `ratchet`, assignment `2-commit-schema` → branch `loom/ratchet-commit-schema`
- Agent `moss`, assignment `4-migrate-identities` → branch `loom/moss-migrate-identities`

**Dependency encoding:** Dependencies in AGENT.json use the format `<agent>/<slug>`. To resolve a dependency to a branch name, convert `<agent>/<slug>` to `loom/<agent>-<slug>`. For example, dependency `ratchet/commit-schema` maps to branch `loom/ratchet-commit-schema`.

**Constraints:**
- Kebab-case throughout: matches `[a-z0-9]+(-[a-z0-9]+)*`
- Maximum length: 63 characters (git ref component limit)
- Each agent works exclusively on its own branch
- The orchestrator creates the branch at worktree creation time, branching from `base_ref`
- Agents MUST NOT push to or modify any other branch

---

## 4. Protocol State in Commits

Protocol state lives in commit messages and trailers. The worktree diff contains only deliverable changes.

Key principle: `git log --format` extracts all protocol state.

Required commit trailers:
- `Agent-Id`: Agent name
- `Session-Id`: UUID for this invocation
- `Task-Status`: Current lifecycle state (ASSIGNED, IMPLEMENTING, COMPLETED, BLOCKED, FAILED)

Optional trailers for richer state:
- `Files-Changed`: Integer count
- `Key-Finding`: One-line discovery worth preserving (repeatable)
- `Decision`: Non-obvious choice with rationale, format `<what> -- <why>` (repeatable)
- `Deviation`: Departure from task spec, format `<what> -- <why>` (repeatable)
- `Blocked-Reason`: Present when Task-Status is BLOCKED
- `Error-Category`: Present when Task-Status is FAILED
- `Error-Retryable`: Present when Task-Status is FAILED
- `Heartbeat`: ISO-8601 UTC timestamp (see Section 4.2)

See `schemas.md` for the full trailer vocabulary, required trailers per state, and commit templates.

### 4.1 Orchestrator Post-Terminal Commits

After an agent reaches a terminal state (COMPLETED or FAILED), the orchestrator MAY commit to the agent's branch for hotfixes, rebasing, or integration preparation. These commits:

- MUST use type `chore(loom):` in the commit subject
- MUST include `Agent-Id: bitswell` and `Session-Id` trailers
- MUST NOT carry a `Task-Status` trailer

Orchestrator post-terminal commits are explicitly outside the state machine. They do not change the agent's status. The agent's terminal status is determined by the last commit that carries a `Task-Status` trailer.

### 4.2 Heartbeat

Agents MUST commit a `Heartbeat` trailer at least every 5 minutes while running. The trailer value is an ISO-8601 UTC timestamp.

```
chore(loom): checkpoint

Agent-Id: ratchet
Session-Id: a1b2c3d4-e5f6-7890-abcd-ef1234567890
Heartbeat: 2026-04-03T14:30:00Z
```

Work commits MAY include the `Heartbeat` trailer alongside other trailers. Dedicated heartbeat commits (with no file changes) are permitted when the agent has no deliverable changes to commit.

The orchestrator monitors agent branches for stale heartbeats. If `now - latest_heartbeat > timeout_seconds` from AGENT.json, the agent is considered stale. Stale agents are terminated: SIGTERM, wait 10 seconds, then SIGKILL.

**Query to check liveness:**

```bash
git log -1 --format='%(trailers:key=Heartbeat,valueonly)' <branch>
```

---

## 5. Multi-Repo Support

### 5.1 Single-Repo (Default)

The `.mcagent/` directory lives inside the coordination repo (e.g., `bitswell/bitswell`). This repo is the single source of truth for agent state. Most assignments create worktrees from this same repo. The `.mcagent/` directory SHOULD be added to `.gitignore` — it is local operational state, not deliverable code.

### 5.2 External Repos via `.repos/`

When agents need to work on external repositories, those repos are added as git submodules under `.repos/`:

```
bitswell/bitswell/                   # Source of truth
  .mcagent/
    agents/
      ratchet/
        identity.md
        1-local-task/
          AGENT.json                 # repo: "bitswell/bitswell"
          worktree/                  # worktree from this repo
        2-fix-acme-frontend/
          AGENT.json                 # repo: "acme/frontend"
          worktree/                  # worktree from .repos/acme/frontend
  .repos/
    acme/
      frontend/                      # git submodule -> acme/frontend
```

The orchestrator adds external repos via `git submodule add` into `.repos/<org>/<repo>/`. Worktrees for external assignments are created from these submodule checkouts. The coordination repo tracks which external repos are registered and at which commit.

### 5.3 Constraints

- `.mcagent/` is always in the coordination repo, never in an external repo
- External repos do not know they are being coordinated — no `.mcagent/` leaks into them
- The `repo` field in AGENT.json identifies the target; the orchestrator resolves it to a `.repos/` submodule path or the local repo

---

## 6. Lifecycle

### 6.1 Create Assignment

The orchestrator:
1. Creates the assignment directory: `.mcagent/agents/<name>/<sequence>-<slug>/`
2. Writes AGENT.json into the assignment directory.
3. Creates a git worktree at `<assignment>/worktree/` from the target repo's `base_ref`.
4. Commits the `Task-Status: ASSIGNED` message to the agent's branch (see schemas.md Section 5.1).
5. Spawns the agent (sync) or commits a dispatch trigger (push-event).

### 6.2 Agent Execution

The agent:
1. Reads `identity.md` for self-reference.
2. Reads `AGENT.json` for assignment config, scope, and budget.
3. Works exclusively within `worktree/`.
4. Commits deliverable changes with required trailers, including `Heartbeat` at least every 5 minutes.
5. Does NOT create or modify any files outside `worktree/` (the orchestrator owns the assignment directory).

### 6.3 Complete Assignment

On completion:
1. The agent's final commit carries `Task-Status: COMPLETED` with required trailers (`Files-Changed`, at least one `Key-Finding`).
2. The orchestrator integrates the agent's branch into the target repo (see protocol.md Section 3.3).
3. The worktree MAY be removed. The assignment directory (with AGENT.json) is retained for audit.
4. The agent's branch is retained per retention policy (default 30 days).

### 6.4 Cleanup

Completed assignment directories are archived, not deleted. The orchestrator MAY compress or relocate them after the retention period. Identity files are never cleaned up.

---

## 7. Conformance Criteria

An implementation conforms to this spec if:

1. **Separation**: AGENT.json is never committed to the agent's code branch. No protocol state files of any kind are present in the worktree.
2. **Identity persistence**: `identity.md` exists before the first assignment and survives after all assignments are cleaned up.
3. **Assignment isolation**: Each assignment has its own directory, its own AGENT.json, and its own worktree. No sharing.
4. **Worktree purity**: The worktree contains only files intended for the target repository. `git status` in the worktree shows no protocol artifacts.
5. **Commit protocol**: Every agent commit includes `Agent-Id`, `Session-Id` trailers. State-transition commits include `Task-Status`. Agents commit `Heartbeat` at least every 5 minutes.
6. **Scope enforcement**: The orchestrator rejects integration of commits modifying files outside `scope.paths_allowed` or inside `scope.paths_denied`.
7. **Naming convention**: Assignment directories follow the `<sequence>-<slug>` pattern. Branches follow `loom/<agent>-<slug>`.
8. **Orchestrator accountability**: Orchestrator commits after terminal state use `chore(loom):` and carry no `Task-Status` trailer.

---

*End of `.mcagent/` directory specification — loom/2 2.0.0-draft.*
