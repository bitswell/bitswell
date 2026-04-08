---
name: LOOM plugin conversion
description: LOOM is being converted from .claude/skills/loom/ into a standalone Claude Code plugin at loom/ in repo root. 8 sub-issues (#41-#48) under parent #40. Branch-based state replaces .mcagent/ directory.
type: project
---

LOOM is being converted from a skill (`.claude/skills/loom/`) into a Claude Code plugin at `loom/` in the repo root.

**Why:** Triple identity problem (agents/, .mcagent/, .claude/agents/), v1/v2 doc contradictions, portability. Plugins can provide agents, skills, hooks, and bin scripts as a self-contained unit.

**How to apply:** All LOOM work goes through issues #41-#48. State lives on branches (commit trailers), not .mcagent/ directory. When reviewing LOOM changes, verify they target the plugin structure, not the old skill location.

**Key decisions:**
- No .mcagent/ directory — all state on loom/* branches via commit trailers
- Bitswell team agents stay in .claude/agents/ (project-specific)
- Plugin provides generic loom-worker agent with `isolation: "worktree"`
- Plugin provides orchestrator skill, lifecycle hooks, dispatch bin scripts

**GitHub project:** "bitswell" project board has Blocked column. Issues #41-#48 tracked there.

**Agent assignments:**
- Ratchet: #41 scaffold, #44 skill, #46 hooks, #48 cleanup
- Vesper: #42 docs, #47 branch-state
- Moss: #43 worker agent, #45 dispatch scripts
