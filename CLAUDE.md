# Bitswell — Claude Code Configuration

## Main Agent

**Bitswell** (`.claude/agents/bitswell.md`) is the primary agent for this project — the one that coordinates the team, talks to the user, and works directly when delegation isn't needed.

To invoke bitswell explicitly, use `@bitswell` or launch it as a subagent. Regular Claude Code sessions in this repo are not automatically bitswell — they are Claude, with access to the agent team.

Project identity and values are in `AGENT.md`. Agent identities are in `agents/<name>/identity.md`.

## Agent Team

| Agent | Role | When to use |
|-------|------|-------------|
| **bitswell** | Main agent | Default. Direct work, coordination, user interaction |
| **bitsweller** | Issue finder | Proactively finds optimization opportunities |
| **vesper** | Planner | Decomposes issues into implementation tasks |
| **ratchet** | Writer | Implements tasks — structural, practical |
| **moss** | Writer | Implements tasks — surgical, minimal |
| **drift** | Reviewer | Lateral thinking, intuitive review |
| **sable** | Reviewer | Skeptical, incisive review |
| **thorn** | Reviewer | Stress-testing, adversarial review |
| **glitch** | Reviewer | Chaos testing, breaks things |
| **bitswelt** | Approver | Final sign-off on implementations |

## Development Workflow

- Agents work in git worktrees. Use standard git (branch, commit, push, PR) — no external VCS tools.
- Bitsweller files issues as commits on the `bitsweller` branch.
- Tasks live in `tasks/` (unassigned, assigned, done).
- Agent identities live in `agents/<name>/identity.md`. Not all agents have discovered identities yet — bitsweller and bitswelt are pending.

## Pipeline Visibility

Three git-native mechanisms track work from issue through merge:

- **`refs/notes/pipeline`** — YAML notes attached to bitsweller issue commits. Track status (`filed → planned → assigned → in-review → shipped → abandoned`), implementation PR, reviewers, and retro link. Written by bitsweller (filed), vesper (planned), bitswell (assigned), bitswelt (shipped). Fetch with `git fetch origin refs/notes/pipeline:refs/notes/pipeline`, view with `git log bitsweller --show-notes=pipeline`.
- **`retros` branch** — Orphan branch, one commit per merged PR. 5-heading template (What worked / What surprised us / What we'd do differently / Follow-ups filed / Signal for future planning). Written by bitswelt at approval time. Ceiling: 15 lines.
- **`Bitsweller-Issue: <sha>` trailer** — Added to merge commits in implementation repos (loom-tools, memctl, etc.) to close the reverse link from PR back to issue. Makes the backlink grep-exact.

These three form a closed loop: issue → note → PR → merge trailer → issue, with retro linked from the note.
