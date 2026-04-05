# LOOM

**Lifecycle-Orchestrated Operation Model** — coordinate AI agents through git worktrees and a commit-based protocol.

## What it is

LOOM is a Claude Code plugin that provides multi-agent orchestration primitives. Agents communicate through git commits, work in isolated worktrees, and follow a structured lifecycle (ASSIGNED → IMPLEMENTING → COMPLETED → REVIEWED → APPROVED).

## Loading the plugin

```sh
claude --plugin-dir ./loom
```

## What it provides

| Directory | Purpose |
|-----------|---------|
| `agents/` | Worker agent definitions (e.g. ratchet, moss, sable) |
| `skills/` | Orchestrator skills for dispatching and coordinating agents |
| `hooks/` | Lifecycle hooks — pre/post task, on commit, on review |
| `bin/` | Dispatch scripts and CLI utilities |

## Protocol

- Each task is a commit on a task branch with structured trailers (`Agent-Id`, `Task-Status`, `Session-Id`)
- Agents work in git worktrees scoped to their assignment
- Reviews and approvals are also commits — the git log is the audit trail
