---
name: .mcagent architecture direction
description: Post-LOOM-v1 retro — redesign separating agent state from codebase, commit-based protocol, push-event dispatch, multi-repo support
type: project
---

After running 21 LOOM agents (42 spawns) to create PRs, Willem proposed a redesign:

**Key principles:**
- Agent metadata in `.mcagent/agents/<name>/<pr_#>-<org>-<repo>/` — separate from code
- Protocol state (status, plan, memory) encoded in commit message trailers, not files
- Named agents with persistent identities (ratchet, moss, drift) instead of anonymous IDs
- Push-event dispatch instead of synchronous orchestrator spawning
- Multi-repo: one `.mcagent/` hub manages worktrees from multiple target repos
- Single-phase spawn for simple tasks; two-phase optional when scopes overlap

**Why:** LOOM v1 created 5 protocol files per 1 content file, required 42 spawns for 21 tasks, polluted PRs with protocol noise, and needed the orchestrator to stay alive throughout.

**How to apply:** This is the next-gen direction for agent coordination in bitswell. The retro and design live at `tests/loom-eval/RETRO.md`. Action items include specifying the `.mcagent/` directory structure, commit-message schema, and dispatch triggers.
