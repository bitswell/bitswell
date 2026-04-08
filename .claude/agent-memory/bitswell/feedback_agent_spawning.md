---
name: Agent spawning via Claude CLI
description: Correct flags and patterns for spawning LOOM agents as child claude processes from an orchestrator session
type: feedback
---

When spawning claude agents as child processes from an orchestrator session:

1. **Use `--dangerously-skip-permissions`**, not `--yes` (which doesn't exist in claude 2.1.92+). The `--permission-mode acceptEdits` works for file writes but blocks Bash without explicit `--allowed-tools`.

2. **Use `dangerouslyDisableSandbox: true`** on the parent Bash call. The parent session's sandbox blocks child processes from writing to `~/.claude/session-env/`, which the child's Bash tool requires.

3. **Pass prompts as inline argument strings**, not via stdin redirection (`< file`). The `$(cat /path/to/prompt.txt)` expansion works; `< /path/to/prompt.txt` with `run_in_background` does not.

4. **Don't spawn more than ~2 agents simultaneously**. Concurrent launches sometimes fail silently with "no stdin data received in 3s" warning and produce no output. Stagger launches or run solo for reliability.

5. **Always include `cd /path/to/worktree &&`** in the same Bash command as the claude invocation. Working directory doesn't reliably persist across separate `run_in_background` calls.

**Why:** Discovered through multiple failed attempts during the 8-PR LOOM plugin conversion (2026-04-05). The `--yes` flag, stdin redirection, and sandbox issues each caused silent agent death with no error output.

**How to apply:** Any time LOOM dispatches agents via `claude -p` from an orchestrator session. The `loom-spawn` script in `loom/bin/` should use `--dangerously-skip-permissions` (PR 5 already implements this).
