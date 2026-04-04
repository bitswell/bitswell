# `.mcagent/` Directory Specification

**Version**: 0.1.0 | **Status**: Draft | **Companion to**: LOOM Protocol v1

---

## 1. Purpose

Separate agent state from the codebase. The `.mcagent/` directory is the operational root for all agent activity. It contains identity, assignment metadata, and worktrees — but never protocol files inside deliverable code.

This specification addresses five failures identified in LOOM v1 (see RETRO.md):
1. Protocol files polluting code worktrees
2. Anonymous, stateless agents
3. Single-repo assumption
4. Synchronous orchestrator bottleneck
5. Ceremony overhead for simple tasks

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

Agent names are stable across the lifetime of the `.mcagent/` installation. They are proper nouns -- they refer to a specific agent with a specific identity.

### 3.2 Identity File

`identity.md` is a Markdown file describing the agent's persistent traits, voice, and operational tendencies. It is read by the orchestrator when spawning the agent and by the agent itself for self-reference.

Identity files are NOT generated per-assignment. They evolve slowly through deliberate updates, not through task execution.

### 3.3 Assignment Directory

Named `<sequence>-<slug>` where:

- `<sequence>` is a monotonically increasing integer (1, 2, 3, ...), unique within the agent. No zero-padding.
- `<slug>` is a lowercase kebab-case descriptor. For single-repo work, use a task descriptor (e.g., `1-mcagent-spec`, `2-fix-auth-middleware`). For multi-repo work, include the repo identifier (e.g., `27-bitswell-bitswell`, `42-acme-frontend`).

The sequence provides ordering. The slug provides human readability. Together they form a unique assignment identifier within the agent's namespace.

### 3.4 AGENT.json

Lives inside the assignment directory, **outside** the worktree. This is the critical distinction from LOOM v1, where AGENT.json was committed to the agent's branch alongside deliverable code.

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

Fields carried forward from LOOM v1: `agent_id`, `session_id`, `protocol_version`, `context_window_tokens`, `token_budget`, `dependencies`, `scope`, `timeout_seconds`.

New fields:
- `assignment_id`: Matches the directory name. Allows the agent to identify its own assignment.
- `repo`: The target repository in `<org>/<repo>` format.
- `base_ref`: The branch or commit the worktree was created from.
- `dispatch`: How this agent was triggered. `sync` means the orchestrator spawned it directly. `push-event` means a commit on `trigger_ref` initiated dispatch.

### 3.5 Worktree

A git worktree created via `git worktree add`. Points to the target repository, branching from `base_ref`.

**The worktree contains ONLY deliverable code.** No TASK.md, no PLAN.md, no STATUS.md, no MEMORY.md, no AGENT.json. These artifacts either live in the assignment directory (AGENT.json) or in commit messages (protocol state).

The worktree path is always `.mcagent/agents/<name>/<assignment>/worktree/`. Implementations MUST NOT place the worktree elsewhere.

**PWD convention**: The dispatch mechanism MUST `cd` into the worktree before spawning the agent. From the agent's perspective, its working directory is the worktree root — it sees a normal git checkout. The agent does not need to know about `.mcagent/` structure. Scope paths in AGENT.json (e.g., `"."`) are relative to the worktree root because that is the agent's PWD.

### 3.6 Orchestrator Identity

The orchestrator (typically `bitswell`) MUST have its own agent directory at `.mcagent/agents/<orchestrator-name>/` with:
- `identity.md` -- persistent orchestrator identity
- `orchestrator.json` -- declares orchestrator role and `.mcagent/**` write scope

```json
{
  "agent_id": "<orchestrator-name>",
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

---

## 4. Protocol State in Commits

LOOM v1 encoded protocol state in files (STATUS.md, PLAN.md, MEMORY.md). This spec moves protocol state to commit messages and trailers.

The commit-message protocol schema is defined in a companion specification. This document defines only the directory structure.

Key principle: `git log --format` extracts all protocol state. The worktree diff contains only deliverable changes.

Required commit trailers (carried forward from LOOM v1, extended):
- `Agent-Id`: Agent name
- `Session-Id`: UUID for this invocation
- `Task-Status`: Current lifecycle state (ASSIGNED, PLANNING, IMPLEMENTING, COMPLETED, BLOCKED, FAILED)

Optional trailers for richer state:
- `Files-Changed`: Integer count
- `Key-Finding`: One-line discovery worth preserving
- `Blocked-Reason`: Present when Task-Status is BLOCKED

---

## 5. Multi-Repo Support

A single `.mcagent/` directory can coordinate work across multiple repositories. Each assignment's worktree points to the appropriate repo.

```
.mcagent/agents/ratchet/
  identity.md
  27-bitswell-bitswell/
    AGENT.json           # repo: "bitswell/bitswell"
    worktree/            # git worktree of bitswell/bitswell
  42-acme-frontend/
    AGENT.json           # repo: "acme/frontend"
    worktree/            # git worktree of acme/frontend
```

The orchestrator is responsible for ensuring the correct repository is cloned and available before creating the worktree. The `.mcagent/` directory itself is not inside any target repo -- it is a standalone coordination root.

---

## 6. Lifecycle

### 6.1 Create Assignment

The orchestrator:
1. Creates the assignment directory: `.mcagent/agents/<name>/<sequence>-<slug>/`
2. Writes AGENT.json into the assignment directory.
3. Creates a git worktree at `<assignment>/worktree/` from the target repo's `base_ref`.
4. Spawns the agent (sync) or commits a dispatch trigger (push-event).

### 6.2 Agent Execution

The agent:
1. Reads `identity.md` for self-reference.
2. Reads `AGENT.json` for assignment config, scope, and budget.
3. Works exclusively within `worktree/`.
4. Commits deliverable changes with required trailers.
5. Does NOT create or modify any files outside `worktree/` (the orchestrator owns the assignment directory).

### 6.3 Complete Assignment

On completion:
1. The agent's final commit carries `Task-Status: COMPLETED`.
2. The orchestrator integrates the agent's branch into the target repo (per LOOM v1 Section 5.3).
3. The worktree MAY be removed. The assignment directory (with AGENT.json) is retained for audit.
4. The agent's branch is retained per LOOM v1 retention policy (default 30 days).

### 6.4 Cleanup

Completed assignment directories are archived, not deleted. The orchestrator MAY compress or relocate them after the retention period. Identity files are never cleaned up.

---

## 7. Conformance Criteria

An implementation conforms to this spec if:

1. **Separation**: AGENT.json is never committed to the agent's code branch. Protocol files (TASK.md, PLAN.md, STATUS.md, MEMORY.md) are never present in the worktree.
2. **Identity persistence**: `identity.md` exists before the first assignment and survives after all assignments are cleaned up.
3. **Assignment isolation**: Each assignment has its own directory, its own AGENT.json, and its own worktree. No sharing.
4. **Worktree purity**: The worktree contains only files intended for the target repository. `git status` in the worktree shows no protocol artifacts.
5. **Commit protocol**: Every agent commit includes `Agent-Id`, `Session-Id`, and `Task-Status` trailers.
6. **Scope enforcement**: The orchestrator rejects integration of commits modifying files outside `scope.paths_allowed` or inside `scope.paths_denied`.
7. **Naming convention**: Assignment directories follow the `<sequence>-<slug>` pattern.

---

## 8. Migration from LOOM v1

### 8.1 Agent Identity

Move agent identities from project-specific locations (e.g., `agents/<name>/identity.md`) to `.mcagent/agents/<name>/identity.md`. The content is unchanged.

### 8.2 Protocol Files

LOOM v1 protocol files (TASK.md, PLAN.md, STATUS.md, MEMORY.md) are no longer committed to agent branches. Their contents are encoded in commit messages and trailers. Existing branches with protocol files are valid under v1 but non-conformant under this spec.

### 8.3 AGENT.json

Move from worktree root to `.mcagent/agents/<name>/<assignment>/AGENT.json`. Add the new fields (`assignment_id`, `repo`, `base_ref`, `dispatch`). Existing fields are backward-compatible.

### 8.4 Worktree Layout

LOOM v1 worktrees were standalone directories. Under this spec, worktrees are nested inside assignment directories. Existing worktrees can be adopted by creating the assignment directory structure around them.

### 8.5 Coexistence

During migration, both conventions MAY coexist. The orchestrator SHOULD detect which convention an agent branch uses by checking for the presence of protocol files in the worktree root. If present, treat as v1. If absent, treat as v2.

---

*End of `.mcagent/` directory specification v0.1.0.*
