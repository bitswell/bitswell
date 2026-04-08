---
name: Workflow preferences — git worktrees, autonomous agents
description: Git worktrees for agent work, always PRs, agents act autonomously; `but` (GitButler CLI) is a potential future dep but not current
type: feedback
---

Use plain `git` with worktrees for all version control. Worktrees are the core mechanism for agent orchestration — each agent gets an isolated worktree to work in.

**Why:** Minimal toolchain. `but` (GitButler CLI) is a potential future addition for the orchestrator but not a current dependency — keep core deps small.

**How to apply:** Use standard git commands. Always create branches and PRs — never push directly to main. Use `cargo binstall` over `cargo install` for Rust tools. Clone external repos to `~/c/m/`.

Willem delegates freely to agents and expects them to act without asking for confirmation on every step. When he says "review and merge", do the full pipeline without checking in at each stage.
