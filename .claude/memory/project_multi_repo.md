---
name: Multi-repo via .repos/ with git submodules
description: bitswell/bitswell is single source of truth; other repos are submodules under .repos/
type: project
---

bitswell/bitswell is the single source of truth for agent coordination. Multi-repo support uses `.repos/` with git submodules:

```
bitswell/bitswell/
  .mcagent/agents/...          # Agent state lives here
  .repos/
    <org>/
      <repo>/                  # git submodule -> external repo
```

Start single-repo first. `.repos/` comes later when needed. Worktrees for external repos are created from the submodule checkouts.

**Why:** Willem wants this repo to be the coordination hub. Submodules keep external repos versioned and trackable. No "standalone coordination root" — it lives inside this repo.

**How to apply:** Update mcagent-spec.md Section 5 (Multi-Repo) to describe `.repos/` + submodules instead of "standalone coordination root". Single-repo is the default; multi-repo is an extension.
