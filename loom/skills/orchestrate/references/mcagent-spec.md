# LOOM Agent Specification

**Version**: 2.0.0 | **Protocol**: `loom/2` | **Status**: Active

Defines agent identity, assignment structure, worktree rules, and conformance criteria for LOOM v2.

---

## 1. Purpose

Separate agent state from the codebase. Agent state lives in commit messages and trailers. Worktrees contain only deliverable code.

---

## 2. Three-Layer Ontology

Every piece of agent state belongs to exactly one layer.

| Layer | Lifecycle | Contents | Location |
|-------|-----------|----------|----------|
| **Identity** | Persistent across all assignments | Agent persona, values, accumulated skill | `agents/<name>/identity.md` (in repo) |
| **Assignment** | Per-task context | Task spec, scope, budget, dependencies | ASSIGNED commit on `loom/<agent>-<slug>` |
| **Worktree** | Ephemeral, deliverable code only | Git worktree of target repo | Created by orchestrator at dispatch time |

Identity outlives any assignment. Assignment context lives in the commit — not in files. A worktree contains nothing that is not intended for the target repository.

---

## 3. Agent Identity

### 3.1 Identity File

`agents/<name>/identity.md` is the agent's persistent persona. It describes traits, voice, and operational tendencies. Read by the orchestrator when dispatching and by the agent for self-reference.

Identity files evolve slowly through deliberate updates. They are NOT generated per-assignment.

### 3.2 Agent Names

Lowercase alphanumeric, kebab-case. Examples: `ratchet`, `moss`, `drift`, `vesper`.

Agent names are stable for the lifetime of the installation.

---

## 4. Assignment Structure

An assignment is encoded in a single ASSIGNED commit on branch `loom/<agent>-<slug>`.

### 4.1 Branch Naming

**Pattern:** `loom/<agent>-<slug>`

- `<agent>`: agent name (kebab-case)
- `<slug>`: task descriptor (kebab-case, no sequence prefix)

Examples: `loom/ratchet-plugin-scaffold`, `loom/moss-fix-auth-middleware`

### 4.2 Assignment Trailers

The ASSIGNED commit body is the full task spec. Required trailers:

| Trailer | Description |
|---------|-------------|
| `Assigned-To` | Agent that will do the work |
| `Assignment` | Assignment slug |
| `Scope` | Allowed paths (glob, relative to worktree root) |
| `Dependencies` | `<agent>/<slug>` refs or `none` |
| `Budget` | Token budget (integer) |

See `schemas.md` for the complete trailer vocabulary and required trailers per state.

### 4.3 Dependency Resolution

Dependencies use format `<agent>/<slug>`. To resolve to a branch: replace `/` with `-`, prepend `loom/`.

Example: dependency `ratchet/plugin-scaffold` → branch `loom/ratchet-plugin-scaffold`.

The dependency graph MUST be a DAG. `loom-dispatch` checks dependencies before spawning.

---

## 5. Worktree Rules

A worktree is a git worktree created for one assignment. It provides filesystem isolation.

**The worktree contains ONLY deliverable code.** No protocol artifacts, no task files, no agent config — nothing that is not intended for the target repository.

Specifically, the worktree MUST NOT contain:
- Any file named `TASK.md`, `PLAN.md`, `STATUS.md`, or `MEMORY.md`
- Agent configuration files
- Protocol state files

All protocol state is encoded in commit messages and trailers.

**PWD convention**: The dispatch mechanism MUST `cd` into the worktree before spawning the agent. The agent sees a normal git checkout. `Scope` paths in the ASSIGNED commit are relative to the worktree root.

---

## 6. Agent Execution

The agent (`@loom:loom-worker`):
1. Reads its identity from `agents/<name>/identity.md`.
2. Reads the ASSIGNED commit from its branch for task spec, scope, and budget.
3. Works exclusively within the worktree.
4. Commits deliverable changes with required trailers, including `Heartbeat` at least every 5 minutes.
5. Does NOT modify files outside the worktree.

The ASSIGNED commit body is the complete task specification. The agent does not need any additional config files.

---

## 7. Orchestrator Identity

The orchestrator is `bitswell`. All orchestrator commits use `Agent-Id: bitswell`.

There is no separate "orchestrator" entity — the orchestrator is an agent with elevated scope (`.**` or full repo access).

**Orchestrator capabilities:**
- Create assignment branches and ASSIGNED commits
- Spawn `@loom:loom-worker` agents via `loom-dispatch` or directly
- Integrate completed agent branches into the workspace
- Write orchestrator post-terminal commits

---

## 8. Conformance Criteria

An implementation conforms to this spec if:

1. **Worktree purity**: The worktree contains only files intended for the target repository. `git status` in the worktree shows no protocol artifacts.
2. **Identity persistence**: `agents/<name>/identity.md` exists before the first assignment and survives after cleanup.
3. **Assignment isolation**: Each assignment has its own branch. No sharing of branches between agents.
4. **Commit protocol**: Every agent commit includes `Agent-Id`, `Session-Id` trailers. State-transition commits include `Task-Status`. Agents commit `Heartbeat` at least every 5 minutes.
5. **Scope enforcement**: The orchestrator rejects integration of commits modifying files outside `Scope` or inside `Scope-Denied`.
6. **Naming convention**: Branches follow `loom/<agent>-<slug>`.
7. **Orchestrator accountability**: Orchestrator commits after terminal state use `chore(loom):` and carry no `Task-Status` trailer.
8. **Dependency enforcement**: `loom-dispatch` checks all dependencies are COMPLETED before spawning.

---

## 9. Multi-Repo Support

### 9.1 Single-Repo (Default)

The coordination repo (`bitswell/bitswell`) is the single source of truth. Most assignments create worktrees from this same repo.

### 9.2 External Repos via `.repos/`

When agents need to work on external repositories, those repos are added as git submodules under `.repos/`:

```
bitswell/bitswell/
  agents/
    ratchet/
      identity.md
  .repos/
    acme/
      frontend/              # git submodule -> acme/frontend
```

Worktrees for external assignments are created from these submodule checkouts. The `Assignment` slug may include the repo identifier for clarity (e.g., `fix-acme-frontend`).

External repos do not know they are being coordinated — no LOOM artifacts leak into them.

---

*End of LOOM Agent Specification v2.0.0.*
