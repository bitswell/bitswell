# LOOM v1 Retrospective

**Date**: 2026-04-04
**Run**: 21 LOOM agents creating PRs for evaluation plans
**Stats**: 42 agent spawns, 21 worktrees, 21 PRs (#8-#28), ~280KB of plans

---

## What Worked

- **Parallel worktree creation** -- 21 worktrees from `main` in one bash loop, no conflicts
- **Parallel agent spawns** -- 21 agents in a single message, all completed successfully
- **Commit trailers** -- 100% compliance across all 21 branches (verified at plan gate)
- **Scope isolation** -- Zero cross-contamination between agents
- **Git as audit trail** -- Every state change is a commit, fully inspectable after the fact

## What Didn't Work

### 1. Two-phase spawn is wasteful for simple tasks

**Problem**: 42 agent spawns for 21 tasks. The planning phase added zero value -- every PLAN.md said the same thing ("I will commit the file, push, create PR"). The plan gate was a rubber stamp.

**Cost**: ~$15+ in API tokens, ~15 minutes of wall time, purely for ceremony.

**Insight**: The two-phase spawn makes sense when plans might conflict (overlapping scopes, complex dependencies). For independent, well-specified tasks, a single agent should plan and implement in one shot.

### 2. Protocol files pollute the worktree

**Problem**: Each agent branch has TASK.md, AGENT.json, PLAN.md, STATUS.md, MEMORY.md committed alongside the actual deliverable (the plan file). That's 5 protocol files for 1 content file. These get merged into main if integrated.

**Impact**: Protocol artifacts leak into the codebase. Reviewers see noise. The PR diff is mostly protocol boilerplate.

### 3. Orchestrator does too much file-writing

**Problem**: The orchestrator had to write TASK.md + AGENT.json into 21 worktrees, commit them, then spawn agents to read those files. This is a file-based RPC pattern -- write a file, spawn a process to read it.

**Insight**: The task description can be the agent's prompt. The agent metadata can live outside the code worktree entirely.

### 4. No dispatch mechanism

**Problem**: The orchestrator must stay alive to spawn agents, wait for them, run the plan gate, re-spawn them. It's a synchronous coordinator bottleneck.

**Insight**: If agent tasks were encoded in commits on a known branch, a push-event trigger could dispatch agents asynchronously. The orchestrator writes intent; the system handles execution.

### 5. Agents are anonymous

**Problem**: Agents are `plan-v0`, `plan-v1`, etc. -- disposable IDs with no identity, no memory across runs, no persona. Every agent starts cold from a generic worker template.

**Insight**: Named agents with persistent identity (like the bitswell team: ratchet, moss, drift) would accumulate skill, have consistent voice, and be addressable across conversations.

---

## Proposed Architecture: `.mcagent/`

### Core Idea

Separate agent state from the codebase. Agent metadata lives in `.mcagent/`, not in the code worktree. Protocol information lives in commit messages, not files.

### Directory Structure

```
.mcagent/
  agents/
    <agent-name>/
      identity.md              # Persistent agent identity
      <pr_#>-<org>-<repo>/     # Per-assignment workspace
        AGENT.json             # Agent config for this assignment
        worktree/              # Git worktree of the TARGET repo
```

**Example:**

```
.mcagent/
  agents/
    ratchet/
      identity.md
      27-bitswell-bitswell/
        AGENT.json
        worktree/              # -> git worktree of bitswell/bitswell
    moss/
      identity.md
      28-bitswell-bitswell/
        AGENT.json
        worktree/
    drift/
      identity.md
      42-acme-frontend/        # Agent working on a DIFFERENT repo
        AGENT.json
        worktree/              # -> git worktree of acme/frontend
```

### Key Changes from LOOM v1

| LOOM v1 | Proposed |
|---------|----------|
| Protocol files (STATUS.md, PLAN.md, MEMORY.md) in worktree | Protocol state in commit messages + trailers |
| TASK.md written to worktree | Task embedded in dispatch commit message |
| AGENT.json in worktree | AGENT.json in `.mcagent/agents/<name>/<assignment>/` |
| Anonymous agents (`plan-v0`) | Named agents with persistent identity (`ratchet`, `moss`) |
| Orchestrator spawns agents synchronously | Push-event dispatch triggers agents |
| Single-repo worktrees only | Multi-repo: one `.mcagent/` manages worktrees from any repo |
| Two-phase spawn mandatory | Single-phase for simple tasks, two-phase optional |
| Plan gate always required | Plan gate only when scopes overlap or deps exist |

### Commit-Based Protocol

Instead of STATUS.md, encode state in commit messages:

```
task(ratchet): create PR for LOOM eval plan v5

Assigns ratchet to create and push a PR for the error recovery
marathon evaluation plan.

Agent-Id: ratchet
Session-Id: f6b392f8-cd1c-47a3-a9fc-c43a4944dfe9
Task-Status: ASSIGNED
Scope: tests/loom-eval/plans/plan-v5.md
Dependencies: none
Budget: 50000
```

The agent reads this commit, does the work, and commits:

```
feat(eval): add LOOM eval plan v5 -- error recovery marathon

Agent-Id: ratchet
Session-Id: f6b392f8-cd1c-47a3-a9fc-c43a4944dfe9
Task-Status: COMPLETED
Files-Changed: 1
Key-Finding: sandbox blocks network; orchestrator assist needed for push
```

**No files created.** `git log --format` extracts all protocol state. The worktree contains only deliverable code.

### Push-Event Dispatch

```
Orchestrator                    Dispatch System              Agent
    |                               |                         |
    |-- create branch + commit ---->|                         |
    |   (task in commit msg)        |                         |
    |                               |-- push event triggers ->|
    |                               |                         |-- creates worktree
    |                               |                         |-- reads AGENT.json
    |                               |                         |-- implements
    |                               |                         |-- commits + pushes
    |                               |<-- push event ----------|
    |<-- notification --------------|                         |
    |                               |                         |
```

The orchestrator doesn't need to stay alive. It creates a commit describing the task, pushes, and walks away. The dispatch system (GitHub Actions, webhook, cron) picks it up and spawns the agent.

### Multi-Repo Support

Because `.mcagent/` is separate from the target repo, one agent hub can coordinate work across multiple repos:

```
.mcagent/agents/ratchet/
  27-bitswell-bitswell/worktree/    # worktree of bitswell/bitswell
  42-acme-frontend/worktree/        # worktree of acme/frontend
  55-acme-api/worktree/             # worktree of acme/api
```

The agent's identity persists across repos. Its AGENT.json per-assignment configures scope and budget for that specific task.

---

## Action Items

1. Design the `.mcagent/` directory spec
2. Define commit-message protocol schema (trailers, required fields per state)
3. Build dispatch trigger (GitHub Actions workflow or Claude Code remote trigger)
4. Migrate bitswell agent team identities into `.mcagent/agents/`
5. Test with a single-agent single-repo flow before scaling
