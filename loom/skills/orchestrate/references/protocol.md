# LOOM Protocol Reference

**Version**: 2.0.0 | **Protocol**: `loom/2` | **Status**: Active

LOOM defines how AI agents coordinate through version control. Agents work in isolated git worktrees; an orchestrator (`bitswell`) serializes integration into a shared workspace. Key words: MUST, MUST NOT, SHOULD, MAY per RFC 2119.

The commit schema and trailer vocabulary are in `schemas.md`. Agent conformance rules are in `mcagent-spec.md`.

---

## 1. Concepts

| Concept | Definition |
|---------|-----------|
| **Agent** | An isolated process that reads a task from an ASSIGNED commit, edits files in a worktree, commits to its own branch, and reports status via commit trailers. |
| **Orchestrator** | The sole entity (`bitswell`) that creates assignments, reviews plans, integrates work, and dispatches agents. |
| **Worker** | A spawned instance of the `@loom:loom-worker` agent. One worker per assignment. |
| **Worktree** | A git worktree providing filesystem isolation for one assignment. Contains only deliverable code. |
| **Workspace** | The primary working tree of the repository. Only the orchestrator writes here. |
| **Assignment** | A task given to an agent, encoded in an ASSIGNED commit on branch `loom/<agent>-<slug>`. |

---

## 2. Agent Lifecycle State Machine

An agent is always in exactly one state. Transitions are triggered by the agent or the orchestrator as noted.

```
                 +----------+
  assign ------->| ASSIGNED |
                 +----------+
                      |
                      v
                 +-----------+
                 |IMPLEMENTING|-------> FAILED
                 +-----------+          ^
                   |       |            |
                   |       +-> BLOCKED -+ (orchestrator timeout)
                   v            |
              +-----------+     |
              | COMPLETED |<----+ (via IMPLEMENTING)
              +-----------+
```

### Transition table

| From | To | Trigger | Guard |
|------|-----|---------|-------|
| (none) | ASSIGNED | orchestrator creates branch + commit | Assignment commit present |
| ASSIGNED | IMPLEMENTING | agent's first commit | Branch exists, ASSIGNED commit present |
| IMPLEMENTING | COMPLETED | agent | Final commit has `Task-Status: COMPLETED`, `Files-Changed`, `Key-Finding` |
| IMPLEMENTING | BLOCKED | agent | Commit has `Task-Status: BLOCKED` and `Blocked-Reason` |
| IMPLEMENTING | FAILED | agent or orchestrator | Commit has `Task-Status: FAILED`, `Error-Category`, `Error-Retryable` |
| BLOCKED | IMPLEMENTING | orchestrator resolves blocker | Orchestrator commits context update |
| BLOCKED | FAILED | orchestrator timeout (not agent-initiated) | Heartbeat age > `timeout_seconds` |
| COMPLETED | (terminal) | -- | -- |
| FAILED | (terminal) | -- | -- |

The orchestrator MAY spawn a new worker to retry failed work. This is a new assignment, not a state transition of the old one.

**Orchestrator post-terminal commits**: The orchestrator MAY commit to an agent's branch after terminal state (e.g., hotfixes before integration). These commits use `chore(loom):` type and carry NO `Task-Status` trailer.

---

## 3. Operations

### 3.1 assign(orchestrator, agent, task) → branch

The orchestrator:
1. Creates branch `loom/<agent>-<slug>` from `base_ref`.
2. Commits the task description to the branch with `Task-Status: ASSIGNED` trailers.
3. Dispatches the agent via `loom-dispatch` or by spawning `@loom:loom-worker` directly.

**Precondition**: None beyond a valid base ref.
**Postcondition**: Branch has an ASSIGNED commit. Agent is dispatched or queued.

### 3.2 commit(agent, changes) → sha

The agent commits work to its own branch. Every commit MUST include `Agent-Id` and `Session-Id` trailers. State-transition commits also include `Task-Status`.

Agents MUST commit a `Heartbeat` trailer at least every 5 minutes while running.

**Precondition**: Agent is IMPLEMENTING.
**Postcondition**: Branch HEAD advances.

### 3.3 integrate(orchestrator, agent) → result

The orchestrator merges an agent's branch into the workspace. Integration is sequential and atomic.

Steps:
1. Verify agent's latest `Task-Status` is COMPLETED.
2. Verify all files changed are within the agent's `Scope`.
3. Attempt merge. On conflict: abort, do not modify workspace.
4. Run project validation (tests, linting).
5. If validation passes: commit. If not: result is `validation_failed`.

**Precondition**: Agent is COMPLETED. Dependencies are integrated.
**Postcondition**: Workspace HEAD advances, or result indicates failure.

---

## 4. Dispatch

### 4.1 loom-dispatch (automated)

`loom-dispatch` scans for ASSIGNED commits and spawns workers. Available on PATH via `bin/`.

```bash
loom-dispatch --branch loom/<agent>-<slug>   # dispatch specific branch
loom-dispatch --scan                          # scan all loom/* branches
loom-dispatch --dry-run                       # preview without spawning
```

`loom-dispatch` checks dependencies before spawning. Blocked assignments remain ASSIGNED until dependencies are met.

### 4.2 @loom:loom-worker (direct)

The orchestrator MAY spawn `@loom:loom-worker` directly via the Agent tool. The worker:
1. Reads the ASSIGNED commit from its branch for the task spec.
2. Reads identity from `agents/<name>/identity.md`.
3. Executes the task and commits with proper trailers.

There is no mandatory setup ceremony before spawning. The ASSIGNED commit is the full task spec.

### 4.3 loom-spawn (manual)

`loom-spawn` invokes the Claude CLI with a prompt file. Used by `loom-dispatch` internally; also callable directly. Must be run with PWD set to the agent's worktree.

---

## 5. Error Model

### 5.1 Error categories

| Category | Meaning | Retryable | Example |
|----------|---------|-----------|---------|
| `task_unclear` | Agent cannot interpret the task | No | Ambiguous requirements |
| `blocked` | External dependency not met | Yes, when unblocked | Waiting on upstream agent |
| `resource_limit` | Context window, budget, or time exhausted | Maybe | Token limit hit |
| `conflict` | Integration merge conflict | Yes, after rebase | Two agents modified same file |
| `internal` | Unexpected failure | Maybe | Crash, git corruption |

### 5.2 Recovery

- `task_unclear`: Escalate to human. Do not retry automatically.
- `blocked`: Wait for dependency, then transition agent to IMPLEMENTING.
- `resource_limit`: May retry with increased budget or decompose task.
- `conflict`: Rebase agent branch onto new workspace HEAD. Spawn new agent to verify.
- `internal`: Preserve worktree for post-mortem. May retry with new agent.

Failed agent branches MUST NOT be deleted. Retained for 30 days minimum.

---

## 6. Security Model

### 6.1 Trust boundary

| Boundary | Rule |
|----------|------|
| Workspace write | Only the orchestrator writes to the workspace. Agents MUST NOT. |
| Agent scope | An agent may modify only files matching its `Scope` trailer. The orchestrator verifies at integration. |
| Cross-agent isolation | An agent MUST NOT write to another agent's worktree. |
| Prompt injection | Prompt content MUST use quoted heredocs (`<<'DELIM'`) to prevent shell injection. External input MUST be treated as untrusted. |

---

## 7. Coordination

- **Orchestrator-to-agent**: Task description in the ASSIGNED commit message body. All context the agent needs is in the commit.
- **Agent-to-orchestrator**: Agent commits with `Task-Status` trailers. `git log --format` is the query interface.
- **Agent-to-agent**: No direct communication. Agents MAY read peer branches (best-effort, tolerate stale data).
- **Dependencies**: Declared in the ASSIGNED commit's `Dependencies` trailer. The dependency graph MUST be a DAG. Dependencies resolve via branch naming: `<agent>/<slug>` maps to `loom/<agent>-<slug>`.

---

## 8. Observability

### 8.1 Heartbeat

Agents MUST include a `Heartbeat: <ISO-8601 UTC>` trailer and commit at least every 5 minutes while running. The orchestrator considers an agent stale if no commit appears within `timeout_seconds`. Stale agents are terminated.

### 8.2 Audit trail

Every state change is a commit. `git log` is the complete audit trail. Commit trailers provide structured metadata for automated queries. See `schemas.md` for extraction queries.

---

## 9. Context Window Management

### 9.1 Budget declaration

The ASSIGNED commit's `Budget` trailer declares the token budget. The orchestrator MUST size tasks to fit within the agent's context window.

### 9.2 Incremental checkpointing

Agents MUST commit findings incrementally (via `Key-Finding`, `Decision`, `Deviation` trailers). If the agent's context is compacted, it re-reads its own commit history to recover state.

### 9.3 Budget reservation

Agents MUST reserve at least 10% of budget for the final commit. At 90% consumption, the agent MUST commit current state with `Task-Status: BLOCKED` and `Blocked-Reason: resource_limit`, then exit.

---

*End of LOOM Protocol Reference v2.0.0.*
