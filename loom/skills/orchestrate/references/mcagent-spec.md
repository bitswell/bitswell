# LOOM State Model

**Version**: 2.0.0-draft | **Protocol**: `loom/1` | **Status**: Draft

> Supersedes the `.mcagent/` directory specification (v1.0.0-draft). All agent state now lives on branches. The filesystem holds only identity and deliverable code.

---

## 1. Purpose

LOOM's state model answers three questions for every agent assignment:

1. **Who is the agent?** (Identity)
2. **What is the assignment?** (Assignment config and task)
3. **Where does the work happen?** (Worktree)

In v1, each answer lived in the filesystem: identity in `.mcagent/agents/<name>/identity.md`, config in `.mcagent/agents/<name>/<assignment>/AGENT.json`, and the worktree at `.mcagent/agents/<name>/<assignment>/worktree/`. The `.mcagent/` directory was local operational state — not committed, not distributed, not part of the protocol.

In v2, the filesystem holds only identity (committed as part of the repo) and deliverable code (committed on the agent's branch). Assignment config travels in commit trailers. Worktrees are ephemeral, created on demand, and managed by the Claude Code agent runtime. **There is no `.mcagent/` directory.**

The shift is not cosmetic. Git commits are immutable, distributed, and auditable. Storing assignment config in trailers means the full assignment record — task description, scope, budget, dependencies — survives push, clone, and CI without any local filesystem state. The state machine was already commit-based; now the config is too.

---

## 2. Three-Layer Ontology

Every piece of agent state belongs to exactly one layer.

| Layer | Lifecycle | Contents | Location |
|-------|-----------|----------|----------|
| **Identity** | Persistent across all assignments | Agent persona, values, voice, operational tendencies | `agents/<name>/identity.md` in the repo |
| **Assignment** | Per-task, survives restarts and re-dispatch | Task description, scope, budget, dependencies, and status | ASSIGNED commit body + trailers on `loom/<agent>-<slug>` |
| **Worktree** | Ephemeral — created for a session, torn down after integration | Git worktree containing only deliverable code | Created by Claude Code `isolation: "worktree"` at dispatch time |

The layers are strictly nested. Identity outlives any assignment. An assignment outlives any single worktree session. A worktree contains nothing that is not intended for the target repository.

---

## 3. Identity Layer

### 3.1 Location

Agent identity lives in the repository, not in local operational state.

**Team agents** (named members of the bitswell agent team):
```
agents/<name>/identity.md
```

**Generic loom-worker** (stateless plugin agent, ships with LOOM):
```
<loom-plugin-dir>/agents/loom-worker/identity.md
```

The orchestrator finds identity by looking for `agents/<name>/identity.md` in the repository root. If not found there, it searches the LOOM plugin directory. If identity is absent entirely, the agent runs without it (the assignment commit body is the only context).

### 3.2 Properties

Identity files are Markdown. They describe the agent's persistent traits, voice, and operational tendencies. They are NOT generated per-assignment. They evolve slowly through deliberate updates, not through task execution.

Identity files ARE committed to the repository. They are versioned, diff-able, and part of the codebase. This is a deliberate departure from v1, where they lived outside the worktree in `.mcagent/`.

### 3.3 Agent Names

Lowercase alphanumeric, kebab-case. Examples: `ratchet`, `moss`, `drift`, `vesper`. Agent names are stable — they are proper nouns referring to a specific identity.

---

## 4. Assignment Layer

### 4.1 The ASSIGNED Commit as Configuration

In v2, there is no `AGENT.json`. All assignment configuration lives in the ASSIGNED commit — body for human-readable task description, trailers for machine-readable config.

```
task(<agent-id>): <short task description>

<Full task description. This is the task body.
Include objective, context, and acceptance criteria.
This replaces what was previously written to TASK.md or AGENT.json.>

Agent-Id: bitswell
Session-Id: <orchestrator-session-id>
Task-Status: ASSIGNED
Assigned-To: <agent-id>
Assignment: <assignment-slug>
Scope: <allowed paths, space-separated globs>
Scope-Denied: <denied paths, omit if none>
Dependencies: <agent/slug refs, comma-separated, or "none">
Budget: <integer token budget>
```

The commit body is the task description. The trailers are the config. Together they constitute the full assignment record.

### 4.2 Assignment Trailers

| Trailer | Type | Required | Description |
|---------|------|----------|-------------|
| `Assigned-To` | string | yes | Agent-id of the assignee |
| `Assignment` | string | yes | Assignment slug (kebab-case, unique per agent) |
| `Scope` | string | yes | Allowed paths (space-separated globs, relative to worktree root). `"."` means full access. |
| `Scope-Denied` | string | no | Denied paths. Omit if none. Deny takes precedence over allow. |
| `Dependencies` | string | yes | Comma-separated `<agent>/<slug>` refs, or `"none"` |
| `Budget` | integer | yes | Token budget for this assignment |

### 4.3 Reading Assignment Config

The dispatcher reads assignment config by querying the ASSIGNED commit trailers — there is no file to open.

```bash
# Read all assignment config from an ASSIGNED commit
git log -1 --format='%(trailers)' <sha>

# Individual fields
git log -1 --format='%(trailers:key=Assigned-To,valueonly)' <sha>
git log -1 --format='%(trailers:key=Assignment,valueonly)' <sha>
git log -1 --format='%(trailers:key=Scope,valueonly)' <sha>
git log -1 --format='%(trailers:key=Budget,valueonly)' <sha>
git log -1 --format='%(trailers:key=Dependencies,valueonly)' <sha>

# Task description (commit body)
git log -1 --format='%b' <sha>
```

### 4.4 Assignment State

Assignment state is the latest `Task-Status` trailer on the branch. The state machine is unchanged from v1.

```bash
# Current status of an assignment
git log -1 \
  --format='%(trailers:key=Task-Status,valueonly)' \
  --grep='Task-Status:' \
  loom/<agent>-<slug>
```

Valid states: `ASSIGNED`, `IMPLEMENTING`, `COMPLETED`, `BLOCKED`, `FAILED`.

---

## 5. Worktree Layer

### 5.1 Creation

Worktrees are created by the Claude Code agent runtime when `isolation: "worktree"` is set in the agent's frontmatter. The orchestrator does not create worktrees directly — it dispatches an agent, and the runtime provisions the worktree.

In v1, the orchestrator created the worktree at a fixed path (`.mcagent/agents/<name>/<assignment>/worktree/`) and stored that path in AGENT.json. In v2, the runtime manages the worktree path. The agent's PWD is the worktree root — this invariant is unchanged.

### 5.2 Contents

The worktree contains ONLY deliverable code. No protocol artifacts — no TASK.md, no PLAN.md, no STATUS.md, no MEMORY.md. `git status` in the worktree shows only files intended for the target repository.

The task description is in the ASSIGNED commit body (readable via `git log`), not in a file on disk. The assignment config is in commit trailers, not in AGENT.json. The agent's status is in commit trailers on its branch.

### 5.3 Cleanup

Worktrees are ephemeral. The Claude Code runtime cleans them up when the agent session ends, or when the branch has no changes. The branch itself (and all commits on it) persists per the branch retention policy (default: 30 days after terminal state).

---

## 6. Discovery

The orchestrator discovers all assignments by scanning `loom/*` branches. No filesystem scan is needed.

### 6.1 List All Assignments

```bash
git branch --list 'loom/*' --format='%(refname:short)'
```

### 6.2 Get Assignment Status

```bash
# Status of a specific assignment
git log -1 \
  --format='%(trailers:key=Task-Status,valueonly)' \
  --grep='Task-Status:' \
  loom/<agent>-<slug>

# Status of all assignments (tabular)
for b in $(git branch --list 'loom/*' --format='%(refname:short)'); do
  s=$(git log -1 \
    --format='%(trailers:key=Task-Status,valueonly)' \
    --grep='Task-Status:' "$b" 2>/dev/null | head -1 | xargs)
  printf '%-40s %s\n' "$b" "${s:-<no-status>}"
done
```

### 6.3 Find Undispatched Assignments

An ASSIGNED branch is one where the latest `Task-Status` commit still says `ASSIGNED` — no agent has yet committed `IMPLEMENTING`.

```bash
for b in $(git branch --list 'loom/*' --format='%(refname:short)'); do
  s=$(git log -1 \
    --format='%(trailers:key=Task-Status,valueonly)' \
    --grep='Task-Status:' "$b" 2>/dev/null | head -1 | xargs)
  [[ "$s" == "ASSIGNED" ]] && echo "$b"
done
```

### 6.4 Check Dependency Completion

```bash
# Dependency "ratchet/commit-schema" maps to branch "loom/ratchet-commit-schema"
dep="ratchet/commit-schema"
branch="loom/${dep//\//-}"
s=$(git log -1 \
  --format='%(trailers:key=Task-Status,valueonly)' \
  --grep='Task-Status:' "$branch" 2>/dev/null | head -1 | xargs)
[[ "$s" == "COMPLETED" ]] && echo "met" || echo "not met"
```

### 6.5 Read Assignment Config from Branch

The ASSIGNED commit is always the first commit on the branch (written by the orchestrator). To read it:

```bash
# The ASSIGNED commit — first commit after the branch diverged from base
sha=$(git log --format='%H' --grep='Task-Status: ASSIGNED' loom/<agent>-<slug> | tail -1)
git log -1 --format='%(trailers)' "$sha"
git log -1 --format='%b' "$sha"    # task body
```

---

## 7. Dispatch Model

### 7.1 Overview

The dispatcher (e.g., `loom-dispatch.sh`) reads ASSIGNED commits and spawns agent sessions. In v2, the dispatcher no longer reads AGENT.json or manages worktree paths — all config comes from commit trailers, and the agent runtime manages the worktree.

### 7.2 Dispatch Steps

1. **Discover**: Scan `git branch --list 'loom/*'` for branches where the latest `Task-Status` is `ASSIGNED`.
2. **Read config**: Extract `Assigned-To`, `Assignment`, `Scope`, `Dependencies`, `Budget` from the ASSIGNED commit trailers. Extract the task description from the commit body.
3. **Check dependencies**: For each dependency `<agent>/<slug>`, verify `loom/<agent>-<slug>` has `Task-Status: COMPLETED`.
4. **Find identity**: Look up `agents/<assigned-to>/identity.md` in the repo. Optional — proceed without it if absent.
5. **Build prompt**: Compose the agent prompt with identity content and task description. Include the branch name, session ID, and commit protocol instructions.
6. **Spawn**: Invoke the Claude Code CLI or Agent SDK with `isolation: "worktree"`. The runtime creates the worktree and sets PWD.

### 7.3 Idempotency

In v2, idempotency is checked via branch state, not lock files. If the latest `Task-Status` on the branch is `IMPLEMENTING` or beyond, the assignment has already been dispatched. Skip it.

```bash
s=$(git log -1 \
  --format='%(trailers:key=Task-Status,valueonly)' \
  --grep='Task-Status:' loom/<agent>-<slug> | head -1 | xargs)
# ASSIGNED = dispatch; anything else = skip
[[ "$s" == "ASSIGNED" ]] || { echo "already dispatched ($s)"; exit 0; }
```

This is strictly more reliable than lock files: lock files can be orphaned (crash, disk wipe, git clone); branch state cannot.

### 7.4 Session ID

In v2, the agent's session ID is generated fresh at dispatch time — it is not stored in AGENT.json. The orchestrator's session ID is carried in the ASSIGNED commit's `Session-Id` trailer. The agent uses a new UUID for its own `Session-Id` trailer on all subsequent commits.

---

## 8. Branch Naming

**Pattern:** `loom/<agent>-<slug>`

The branch name encodes the agent and assignment. There is a one-to-one mapping between assignments and branches.

**Examples:**
- Agent `ratchet`, assignment `commit-schema` → `loom/ratchet-commit-schema`
- Agent `moss`, assignment `migrate-identities` → `loom/moss-migrate-identities`
- Agent `vesper`, assignment `pr-7-branch-state` → `loom/vesper-pr-7-branch-state` (this branch)

**Dependency resolution:** Dependencies in ASSIGNED trailers use `<agent>/<slug>` format. To resolve to a branch name: replace `/` with `-`, prepend `loom/`. Example: `ratchet/commit-schema` → `loom/ratchet-commit-schema`.

**Constraints:**
- Kebab-case: `[a-z0-9]+(-[a-z0-9]+)*`
- Maximum length: 63 characters (git ref component limit)
- One agent per branch, one branch per assignment
- The orchestrator creates the branch at assignment creation time
- Agents MUST NOT push to or modify any other branch

---

## 9. Multi-Repo Support

### 9.1 Single-Repo (Default)

The coordination repo (`bitswell/bitswell`) is the single source of truth. All `loom/*` branches live here. Agent identities live in `agents/` here. Most assignments work on files in this same repo.

### 9.2 External Repos via `.repos/`

When agents need to work on external repositories, those repos are added as git submodules under `.repos/<org>/<repo>/`. The agent's worktree is created from the submodule checkout.

The ASSIGNED commit's `Scope` trailer uses paths relative to the worktree root, which is the external repo checkout. The `repo` field is implicit in the branch context — the orchestrator knows which external repo corresponds to which assignment by convention or by a `Repo` trailer (future extension).

External repos do not carry any LOOM state. No `loom/*` branches, no `agents/` dirs in the external repo. Coordination state lives entirely in the coordination repo.

---

## 10. Backward Compatibility

### 10.1 Existing `loom/*` Branches

All existing `loom/*` branches created under v1 remain fully readable. The commit trailer format is unchanged. State extraction queries are unchanged.

The only v1 artifact that becomes obsolete is `AGENT.json`. Existing branches that were dispatched under v1 may have an AGENT.json in `.mcagent/` on disk — this file is now ignored by the protocol. It can be deleted or left in place without affecting branch state.

### 10.2 State Queries Are Unchanged

All `git log --format` queries from v1 continue to work:

```bash
# These queries work identically against v1 and v2 branches
git log -1 --format='%(trailers:key=Task-Status,valueonly)' loom/<agent>-<slug>
git log -1 --format='%(trailers:key=Heartbeat,valueonly)' loom/<agent>-<slug>
git log --format='%(trailers:key=Key-Finding,valueonly)' loom/<agent>-<slug> | grep -v '^$'
```

### 10.3 The `.mcagent/` Directory

If `.mcagent/` exists on disk from a v1 deployment, it can be safely deleted after all v1 assignments reach terminal state. It MUST be in `.gitignore` (it was never committed). No LOOM v2 code reads from `.mcagent/`.

---

## 11. Conformance Criteria

An implementation conforms to this spec if:

1. **No `.mcagent/` directory**: No LOOM code creates, reads, or requires `.mcagent/`. If the directory exists from a prior deployment, it is ignored.
2. **No AGENT.json**: Assignment config lives entirely in ASSIGNED commit trailers. No AGENT.json file is written or read.
3. **Identity in repo**: Agent identity files live at `agents/<name>/identity.md` in the repository (or the LOOM plugin directory for generic agents).
4. **Assignment commit protocol**: ASSIGNED commits carry all required trailers (`Assigned-To`, `Assignment`, `Scope`, `Dependencies`, `Budget`) and the full task description in the body.
5. **Worktree via runtime**: Worktrees are created by the agent runtime (`isolation: "worktree"`), not by dispatcher scripts.
6. **Discovery via git**: All assignment discovery uses `git branch --list 'loom/*'` and `git log` queries. No filesystem scan of a state directory.
7. **Idempotency via branch state**: Dispatch idempotency is enforced by checking the branch's latest `Task-Status` — not by lock files.
8. **Worktree purity**: The worktree contains only files intended for the target repository. No protocol artifacts in the worktree.
9. **Commit protocol**: Every agent commit includes `Agent-Id` and `Session-Id` trailers. State-transition commits include `Task-Status`. Agents commit `Heartbeat` at least every 5 minutes.
10. **Scope enforcement**: Integration rejects commits modifying files outside `Scope` (from ASSIGNED trailer) or inside `Scope-Denied`.
11. **Branch naming**: Branches follow `loom/<agent>-<slug>`. Dependencies resolve via `<agent>/<slug>` → `loom/<agent>-<slug>`.
12. **Orchestrator accountability**: Orchestrator post-terminal commits use `chore(loom):` type with no `Task-Status` trailer.

---

## Appendix: Migration from v1

| v1 Location | v2 Location | Notes |
|-------------|-------------|-------|
| `.mcagent/agents/<name>/identity.md` | `agents/<name>/identity.md` | Now committed to repo |
| `.mcagent/agents/<name>/<assignment>/AGENT.json` | ASSIGNED commit trailers | No file equivalent |
| `.mcagent/agents/<name>/<assignment>/worktree/` | Managed by `isolation: "worktree"` | Path not fixed |
| `.mcagent/agents/<name>/<assignment>/.dispatch-lock` | Branch `Task-Status` trailer | State replaces lock file |
| AGENT.json `session_id` | Generated fresh at dispatch | No persistent session ID |
| AGENT.json `base_ref` | Branch creation point (implicit) | Not stored explicitly |
| AGENT.json `repo` | Coordination repo or `.repos/` submodule | Not stored in trailers (v1) |
| AGENT.json `context_window_tokens` | Not stored | Runtime responsibility |
| AGENT.json `timeout_seconds` | Not stored | Heartbeat monitoring via `git log` |
| AGENT.json `dispatch.mode` | Not stored | Runtime concern, not protocol |

---

*End of LOOM State Model v2.0.0-draft.*
