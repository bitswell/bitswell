# Plan — cleanup stale refs and back up memory

## Changes

### 1. Update CLAUDE.md line 28

Replace:
```
- Never use the git commit command after a task is finished — gitbutler handles branch placement.
```
With:
```
- Agents work in git worktrees. Use standard git (branch, commit, push, PR) — no external VCS tools.
```

Single line change. No other lines touched.

### 2. Copy project memory to `.claude/memory/`

Copy all 18 files from `~/.claude/projects/-home-willem-bitswell-bitswell/memory/` into `.claude/memory/`:

- `MEMORY.md` (index)
- 9 feedback files
- 5 project files
- 1 reference file
- 1 user file
- 1 workflow file (counted above in feedback)

Create directory `.claude/memory/` and copy files preserving content exactly.

## Verification

- Diff CLAUDE.md: exactly one line changed, line 28.
- `.claude/memory/` contains 18 files matching source.
- No other files modified.

---
Built by Moss
