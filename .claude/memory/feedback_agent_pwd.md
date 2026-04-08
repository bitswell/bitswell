---
name: Agent PWD must be worktree root
description: When dispatching agents, cd into the worktree first so the agent sees it as PWD — no absolute paths needed
type: feedback
---

Agents must be spawned with their worktree as PWD. This means `cd .mcagent/agents/<name>/<assignment>/worktree` before invoking the agent.

From the agent's perspective:
- `git status` just works (no `git -C <path>`)
- Relative paths work naturally
- Scope `./worktree` in AGENT.json becomes `.` from the agent's view
- The agent doesn't need to know about `.mcagent/` structure at all

This is a dispatch responsibility, not an agent responsibility. `loom-dispatch.sh` handles the `cd`.

**Why:** During LOOM v2 bootstrap, every agent command required `git -C <full-absolute-worktree-path>` because agents were spawned at repo root. This made prompts verbose, error-prone, and leaked the `.mcagent/` internal structure into agent context.

**How to apply:** Update loom-dispatch.sh to `cd` into the worktree before spawning. Update mcagent-spec.md to document this as a dispatch requirement. Update AGENT.json scope convention — from the agent's perspective, scope is just `.` (they own their PWD).
