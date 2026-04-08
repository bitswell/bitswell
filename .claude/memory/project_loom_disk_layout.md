---
name: LOOM disk layout conventions
description: .loom/agents/ for agent state+worktrees, repos/ for submodules only, naming conventions
type: project
---

Agent orchestration state lives under `.loom/`, not `.mcagent/` (superseded).

**Layout:**
```
.loom/agents/<agent-name>/
  identity.md
  worktrees/
    <org>_<repo>_<descriptive-branch>/   ← git worktree

repos/<org>/<repo>/   ← submodule checkout (main branch only)
```

**Naming:** Worktree dirs use `<org>_<repo>_<branch-description>` (underscores separate segments). Repos dir uses `<org>/<repo>` (slash-separated).

**AGENT.json:** No longer a file — config lives in ASSIGNED commit trailers (LOOM v2 protocol).

**Why:** Clean separation between code repos (submodules) and agent workspace state. Agents own their worktrees. The structure dogfoods what loom-tools will eventually manage programmatically.

**How to apply:** When dispatching agents, create worktrees under `.loom/agents/<name>/worktrees/`. Never put worktrees in `repos/`. Never put non-submodule content in `repos/`.
