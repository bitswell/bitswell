# LOOM Protocol Reference

**Version**: 2.0.0-draft | **Protocol**: `loom/2` | **Status**: Draft

LOOM defines how AI agents coordinate through version control. Agents work in isolated git worktrees; an orchestrator serializes integration into a shared workspace. Key words: MUST, MUST NOT, SHOULD, MAY per RFC 2119.

The authoritative directory specification is `mcagent-spec.md`. This document defines the lifecycle state machine, operations, error model, security model, and coordination patterns.

---

## 1. Concepts

| Concept | Definition |
|---------|-----------|
| **Agent** | An isolated process that reads a task, edits files in a worktree, commits to its own branch, and reports status via commit trailers. |
| **Orchestrator** | The sole entity (`bitswell`) that creates agents, reviews plans, integrates work into the workspace, and manages `.mcagent/`. |
| **Worktree** | A git worktree providing filesystem isolation for one agent. Contains only deliverable code. |
| **Workspace** | The primary working tree of the repository. Only the orchestrator writes here. |
| **Assignment** | A task given to an agent, tracked in `.mcagent/agents/<name>/<assignment>/`. |

---

## 2. Agent Lifecycle State Machine

An agent is always in exactly one state. Transitions are triggered by the agent or the orchestrator as noted. Invalid transitions MUST be rejected.

```
                      +-----------+
         assign ----->|IMPLEMENTING|-------> FAILED
                      +-----------+          ^
                        |       |            |
                        |       +-> BLOCKED -+
                        v            |
                   +-----------+     |
                   | COMPLETED |<----+ (via IMPLEMENTING)
                   +-----------+
```

### Transition table

| From | To | Trigger | Guard |
|------|-----|---------|-------|
| (none) | ASSIGNED | orchestrator creates branch + commit | AGENT.json exists in assignment dir |
| ASSIGNED | IMPLEMENTING | agent's first commit | Branch exists, task commit present |
| IMPLEMENTING | COMPLETED | agent | Final commit has `Task-Status: COMPLETED`, `Files-Changed`, `Key-Finding` |
| IMPLEMENTING | BLOCKED | agent | Commit has `Task-Status: BLOCKED` and `Blocked-Reason` |
| IMPLEMENTING | FAILED | agent or orchestrator | Commit has `Task-Status: FAILED`, `Error-Category`, `Error-Retryable` |
| BLOCKED | IMPLEMENTING | orchestrator resolves blocker | Orchestrator updates task context |
| BLOCKED | FAILED | orchestrator timeout | Heartbeat age > configured threshold |
| COMPLETED | (terminal) | -- | -- |
| FAILED | (terminal) | -- | -- |

The orchestrator MAY spawn a new agent to retry failed work. This is a new agent, not a state transition of the old one.

**Orchestrator post-terminal commits**: The orchestrator MAY commit to an agent's branch after terminal state (e.g., hotfixes before integration). These commits use `chore(loom):` type and carry NO `Task-Status` trailer. They are outside the state machine.

---

## 3. Operations

### 3.1 assign(orchestrator, agent, task) -> worktree

The orchestrator:
1. Creates `.mcagent/agents/<name>/<assignment>/` with AGENT.json.
2. Creates a git worktree branching from `base_ref`.
3. Commits the task description to the agent's branch with `Task-Status: ASSIGNED` trailers.
4. Spawns the agent (sync) or pushes the branch for dispatch (push-event).

**Precondition**: AGENT.json written. Worktree created.
**Postcondition**: Agent's branch has an ASSIGNED commit.

### 3.2 commit(agent, changes) -> sha

The agent commits work to its own branch. Every commit MUST include `Agent-Id` and `Session-Id` trailers. State-transition commits also include `Task-Status`.

Agents MUST commit a `Heartbeat` trailer at least every 5 minutes while running. This is the liveness signal.

**Precondition**: Agent is IMPLEMENTING.
**Postcondition**: Branch HEAD advances.

### 3.3 integrate(orchestrator, agent) -> result

The orchestrator merges an agent's branch into the workspace. Integration is sequential and atomic.

Steps:
1. Verify agent's latest `Task-Status` is COMPLETED.
2. Verify all files changed are within the agent's `scope`.
3. Attempt merge. On conflict: abort, do not modify workspace.
4. Run project validation (tests, linting -- project-defined).
5. If validation passes: commit. If not: result is `validation_failed`.

**Precondition**: Agent is COMPLETED. Dependencies are integrated.
**Postcondition**: Workspace HEAD advances, or result indicates failure.

---

## 4. Error Model

### 4.1 Error categories

| Category | Meaning | Retryable | Example |
|----------|---------|-----------|---------|
| `task_unclear` | Agent cannot interpret the task | No | Ambiguous requirements |
| `blocked` | External dependency not met | Yes, when unblocked | Waiting on upstream agent |
| `resource_limit` | Context window, budget, or time exhausted | Maybe | Token limit hit |
| `conflict` | Integration merge conflict | Yes, after rebase | Two agents modified same file |
| `internal` | Unexpected failure | Maybe | Crash, git corruption |

### 4.2 Recovery

- `task_unclear`: Escalate to human. Do not retry automatically.
- `blocked`: Wait for dependency, then transition agent to IMPLEMENTING.
- `resource_limit`: May retry with increased budget or decompose task.
- `conflict`: Rebase agent branch onto new workspace HEAD. Spawn new agent to verify.
- `internal`: Preserve worktree for post-mortem. May retry with new agent.

Failed agent branches MUST NOT be deleted. Retained for 30 days minimum.

---

## 5. Security Model

### 5.1 Trust boundary

| Boundary | Rule |
|----------|------|
| Workspace write | Only the orchestrator writes to the workspace. Agents MUST NOT. |
| Agent scope | An agent may modify only files matching its `scope` in AGENT.json. The orchestrator verifies at integration. |
| Cross-agent isolation | An agent MUST NOT write to another agent's worktree. |
| `.mcagent/` ownership | Only the orchestrator writes to `.mcagent/`. Agents read only their own AGENT.json and identity. |

### 5.2 Input validation

- The orchestrator MUST validate commit trailers before acting on them.
- External input (PR comments, issues) MUST be treated as untrusted.
- Prompt files MUST use quoted heredocs (`<<'DELIM'`) to prevent shell injection.

---

## 6. Coordination

- **Orchestrator-to-agent**: Task description in the ASSIGNED commit message. AGENT.json for config.
- **Agent-to-orchestrator**: Agent commits with `Task-Status` trailers. `git log --format` is the query interface.
- **Agent-to-agent**: No direct communication. Agents MAY read peer branches (best-effort, tolerate stale data).
- **Dependencies**: Declared in AGENT.json. The dependency graph MUST be a DAG (orchestrator rejects cycles). Integration proceeds in topological order. Dependencies resolve via branch naming: `<agent>/<slug>` maps to `loom/<agent>-<slug>`.

---

## 7. Observability

### 7.1 Heartbeat

Agents MUST include a `Heartbeat: <ISO-8601 UTC>` trailer and commit at least every 5 minutes while running. The orchestrator considers an agent stale if no commit appears within `timeout_seconds`. Stale agents are terminated.

### 7.2 Audit trail

Every state change is a commit. `git log` is the complete audit trail. Commit trailers provide structured metadata for automated queries. See `schemas.md` for extraction queries.

---

## 8. Context Window Management

### 8.1 Budget declaration

AGENT.json declares `context_window_tokens` and `token_budget`. The orchestrator MUST size tasks to fit within the agent's context window.

### 8.2 Incremental checkpointing

Agents MUST commit findings incrementally (via `Key-Finding`, `Decision`, `Deviation` trailers). If the agent's context is compacted, it re-reads AGENT.json and its own commit history to recover state.

### 8.3 Budget reservation

Agents MUST reserve at least 10% of `token_budget` for the final commit (status + findings + trailers). At 90% consumption, the agent MUST commit current state with `Task-Status: BLOCKED` and `Blocked-Reason: resource_limit`, then exit.

---

*End of LOOM Protocol Reference v2.0.0-draft.*
