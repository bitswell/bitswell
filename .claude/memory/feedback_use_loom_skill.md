---
name: Actually use the loom skill — don't reinvent it
description: Use the loom skill's worker template, two-phase spawn, plan gate — don't hand-craft agent prompts
type: feedback
---

When orchestrating LOOM work, use the actual loom skill infrastructure. Don't reinvent it inline.

**Why:** The whole point is dogfooding. Hand-crafting agent prompts and skipping the plan gate defeats the purpose. The skill exists to be used.

**How to apply:**
1. Read `references/worker-template.md` and substitute `{{WORKTREE_PATH}}`, `{{AGENT_ID}}`, `{{SESSION_ID}}`
2. Always run the two-phase spawn: planning first, then implementation
3. Always run the plan gate — review PLAN.md before approving implementation
4. Use the skill's command patterns for worktree creation, assignment commits, integration
5. Don't skip steps because the task "seems simple enough"
