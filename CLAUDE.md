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

- Never use the git commit command after a task is finished — gitbutler handles branch placement.
- Bitsweller files issues as commits on the `bitsweller` branch.
- Tasks live in `tasks/` (unassigned, assigned, done).
- Agent identities live in `agents/<name>/identity.md`. Not all agents have discovered identities yet — bitsweller and bitswelt are pending.
- LOOM orchestration is available via plugin: `claude --plugin-dir ./loom`
