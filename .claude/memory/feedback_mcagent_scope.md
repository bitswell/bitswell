---
name: .mcagent scope and orchestrator identity
description: Scope paths in AGENT.json must be relative to the assignment directory, using ./worktree/ prefix explicitly
type: feedback
---

Scope paths in AGENT.json must reference `./worktree/` explicitly since AGENT.json lives alongside the worktree, not inside it. Example:

```json
"scope": {
  "paths_allowed": ["./worktree/.claude/skills/loom/**"],
  "paths_denied": []
}
```

NOT `".claude/skills/loom/**"` (ambiguous — relative to what?) and NOT `".mcagent/**"` (would block the agent's own worktree).

The orchestrator (bitswell) needs its own identity at `.mcagent/agents/bitswell/` with explicit `.mcagent/**` write access declared in `orchestrator.json`.

**Why:** During LOOM v2 bootstrap, agents couldn't write to their worktrees because (a) scope was ambiguous about what it's relative to and (b) there was no declared permission for who owns `.mcagent/`.

**How to apply:** Always use `./worktree/` prefix in AGENT.json scope paths. Update mcagent-spec.md Section 3.4 to document this convention.
