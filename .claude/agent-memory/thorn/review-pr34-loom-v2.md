---
name: PR #34 Review - LOOM v2 consolidated spec
description: Thorn's structural review of the LOOM v2 spec consolidation, identifying 8 pressure points including missing heartbeat, dispatch idempotency, shell injection, and --print mode blocker
type: project
---

## Review -- Thorn

**PR**: #34 — LOOM v2: consolidated spec with all bootstrap fixes
**Verdict**: request-changes

I read all five files, Glitch's test results, and the PR description claiming 10 fixes. Glitch found the surface cracks. I looked at the load-bearing structure.

---

### Pressure Point 1: The v2 heartbeat mechanism does not exist

The protocol (Section 9.1) says: agents MUST update `heartbeat_at` in STATUS.md and commit every 5 minutes. Stale agents get SIGTERM, then SIGKILL.

In v2, STATUS.md does not exist. The heartbeat mechanism was defined entirely in terms of a file that v2 explicitly removes from the worktree. The commit-based protocol (schemas.md Section 8) defines no heartbeat trailer. There is no `Heartbeat-At` trailer. There is no replacement mechanism.

This means in v2: the orchestrator has **no way to detect a stalled agent**. An agent that hangs after its first `Task-Status: IMPLEMENTING` commit will appear to be working indefinitely. The only timeout is the outer `timeout_seconds` from AGENT.json, but the dispatch script (`loom-dispatch.sh`) does not implement any timeout monitoring. The spawn runs as a background process (line 163: `&`) with the PID echoed to stdout and then forgotten. Nobody is watching that PID.

**The liveness detection system was amputated during the v1-to-v2 migration and nothing replaced it.** This is the single most dangerous gap in the spec.

### Pressure Point 2: The dispatch script has no idempotency guard

`loom-dispatch.sh --scan` iterates all `loom/*` branches, finds ones with `Task-Status: ASSIGNED` as the latest status, and dispatches them. But "latest status" is determined by `git log -1 --format='%(trailers:key=Task-Status,valueonly)' --grep='Task-Status:' "$branch"`.

If the orchestrator commits ASSIGNED, the scan dispatches the agent. The agent starts working (background process). The scan runs again (cron, manual, whatever). The ASSIGNED commit is still on the branch -- the agent's IMPLEMENTING commit hasn't landed yet because the agent is still working. The scan dispatches the agent **a second time**. Now two Claude Code instances are writing to the same worktree simultaneously.

There is no lock file. No PID tracking. No "dispatch already in progress" marker. The scan mode is a footgun that gets more dangerous the longer agents take to produce their first commit. For any agent that takes more than a few seconds to start (which is all of them, given LLM cold start), repeated scans will produce duplicate spawns.

### Pressure Point 3: Two agents, same file -- the spec hand-waves the hardest problem

The protocol (Section 5.3) says integration is "sequential and atomic" and on conflict: "set result to `conflict`, do not modify workspace." The recovery section (6.3) says: "Rebase agent branch onto new workspace HEAD. Retry agent."

What does "retry agent" mean? The agent is an LLM invocation that consumed context. There is no "retry" that preserves its work. You're spawning a new agent to redo the task from scratch on the rebased branch, plus manually resolving any conflicts the rebase produced. The spec treats this like retrying an HTTP request. It is not. It is re-executing potentially hours of LLM computation.

And the scope system provides no prevention. Two agents can have `Scope: .` and freely modify the same files. The dependency system can prevent integration ordering, but it cannot prevent two agents from working on overlapping files simultaneously, discovering the conflict only at integration time -- after both have consumed their full token budgets.

The spec needs to either: (a) define a file-level lock mechanism so conflicts are detected at dispatch time, not integration time, or (b) acknowledge that conflict detection is deferred and specify what "retry agent" actually costs.

### Pressure Point 4: The prompt file is a shell injection vector

`loom-dispatch.sh` line 144-156 writes a heredoc that interpolates `$task_body` directly from the commit message body:

```bash
task_body="$(git log -1 --format='%b' "$sha")"
cat > "$prompt_file" <<PROMPT
...
$task_body
...
PROMPT
```

This is a heredoc without quoting the delimiter (`<<PROMPT` not `<<'PROMPT'`). Shell variable expansion is active. If a commit message body contains `$(rm -rf /)` or backticks with commands, they will execute during the heredoc expansion. The commit message is written by the orchestrator, so in the happy path this is controlled input. But the protocol (Section 7.2) says "External input (PR comments, issues) MUST be treated as untrusted." If the orchestrator ever constructs a task from external input (which is its primary use case -- turning issues into agent tasks), the prompt construction becomes a code execution vulnerability.

The fix is `<<'PROMPT'` (quoted delimiter) and explicit variable substitution for the controlled fields. This is a one-character fix for a code execution bug.

### Pressure Point 5: The v1/v2 state machines are contradictory and both are normative

The protocol.md (Section 3) defines the v1 state machine:
```
spawn -> PLANNING -> IMPLEMENTING -> COMPLETED
                                  -> BLOCKED -> IMPLEMENTING/FAILED
                  -> FAILED
```

The schemas.md (Section 8.7) defines the v2 state machine:
```
ASSIGNED -> IMPLEMENTING -> COMPLETED
                         -> BLOCKED -> IMPLEMENTING
                         -> FAILED
```

These are fundamentally different machines. v1 starts at PLANNING, v2 starts at ASSIGNED. v1 allows BLOCKED -> FAILED directly. v2 says BLOCKED -> FAILED is an invalid transition ("must resume IMPLEMENTING first"). v1 allows BLOCKED -> COMPLETED directly (the ASCII art shows an arrow). v2 says that's invalid too.

The PR claims "PLANNING removed from v2." But the protocol.md still defines PLANNING as the starting state in Section 3, which is presented without any scoping to v1. Section 4.0 says "Two directory conventions exist" but does not say which state machine applies to which convention. A reader who reads Section 3 before Section 8.7 will implement the wrong state machine.

The fix: Section 3 needs a clear label: "This state machine applies to `loom/1` only. For `loom/2`, see Section 8.7." Currently, the reader has to figure this out by noticing the contradiction.

### Pressure Point 6: `loom-spawn.sh` uses `--print` mode which is fire-and-forget

Line 36: `exec "$CLAUDE" --print $EXTRA_ARGS < "$PROMPT_FILE"`

`--print` is non-interactive mode. It sends the prompt, gets a response, prints it, and exits. This is appropriate for a one-shot query. It is not appropriate for an agent that needs to:
- Read files
- Make commits
- Update trailers
- Respond to blocked states
- Checkpoint progress

The `--print` flag means the "agent" gets one response from Claude and then the process exits. That response cannot include tool use, file reads, or git operations -- those require the interactive Claude Code session. The entire dispatch pipeline, which carefully sets up worktrees and prompts, spawns an agent that **cannot use any tools**.

If the intent is to use `claude` without `--print`, the spawn script needs to handle the interactive session differently (probably `--yes` or similar for auto-approval). If the intent is `--print`, then the protocol's assumptions about agents making multiple commits, reading files, and checkpointing are incompatible with the spawn mechanism.

This is the most immediately blocking issue. The pipeline literally cannot produce a working agent.

### Pressure Point 7: Temp file cleanup never happens

`loom-dispatch.sh` creates a temp file at line 143: `prompt_file="$(mktemp)"`. The spawn command is run in the background. The temp file is never deleted. Not on success, not on failure, not on signal. Over time, `/tmp` accumulates one prompt file per dispatch. Each contains the full task body, which may include sensitive context.

More importantly: the spawn runs as a background subshell (`(cd "$worktree" && "$SPAWN_CMD" "$prompt_file") &`). The dispatch function returns immediately. If the scan dispatches 10 agents, it creates 10 temp files and 10 background processes, with no tracking of which PID corresponds to which agent, and no cleanup path for any of them.

### Pressure Point 8: The orchestrator hotfix pattern breaks the state machine by design

Glitch found that the orchestrator commits to agent branches after terminal state. The PR description doesn't address this. But it's not a bug to fix -- it's a design gap to fill.

The orchestrator *needs* to sometimes modify an agent's branch post-completion (applying fixes, rebasing for integration). The state machine says terminal states are terminal. The orchestrator ignores this because it has to. The spec needs to acknowledge that orchestrator commits are exempt from the state machine, or define a mechanism for post-terminal amendments (e.g., an `AMENDED` state, or explicit rules that orchestrator commits after terminal state don't carry `Task-Status` trailers).

Currently, the spec defines a rule that its own orchestrator systematically violates. This is not enforcement that's "aspirational" -- it's a spec that's wrong about its own operational model.

---

### Structural Assessment

The three-layer architecture (identity / assignment / worktree) is genuinely well-designed. Glitch is right about that. The commit-based protocol design (trailers instead of files) is a strong idea that simplifies the worktree and makes state auditable via `git log`.

But the spec has moved the protocol from files (which were self-contained and simple) to commits (which are more powerful but harder to get right), and in the process it lost:

1. **Liveness detection** (heartbeat had a home in STATUS.md; it has no home in v2)
2. **Tooling compatibility** (the spawn script uses `--print` which can't do tool use)
3. **Dispatch safety** (no idempotency, no process tracking, no cleanup)
4. **A code execution bug** (unquoted heredoc)
5. **State machine clarity** (two contradictory machines with no clear scoping)

The skeleton is strong. But the nervous system is not just "not yet connected" as Glitch says. Parts of it were severed during the migration and nobody noticed because the bootstrap was done manually, not through the dispatch pipeline.

**What would make this acceptable:**

1. Define a v2 heartbeat mechanism (e.g., `Heartbeat` trailer on periodic commits, or a sideband file outside the worktree in the assignment directory)
2. Fix the heredoc quoting (`<<'PROMPT'`)
3. Add a dispatch lock (pidfile or marker in the assignment directory)
4. Resolve `--print` vs interactive mode in loom-spawn.sh
5. Add a version-scoping header to the v1 state machine in protocol.md
6. Define orchestrator-post-terminal commit rules
7. Clean up temp files (trap on EXIT)

Items 2 and 7 are one-line fixes. Item 4 is the blocker -- without it, the dispatch pipeline is decorative.

-- Reviewed by Thorn
