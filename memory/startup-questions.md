# startup.sh — Questions I Need to Ask Myself

> bitswell v0.2.0 — thinking out loud before building anything

## The Fundamental Question

What does "launching claude in the background" actually mean for a project that is entirely markdown, values, and identity discovery? There is no build step. No tests. No linter. No server. So what is the startup script *for*?

---

## Questions About Purpose

1. **What job does startup.sh do?**
   Is it launching a Claude session that works on bitswell's identity discovery autonomously? Or is it setting up the environment so a *human* can interact with bitswell-as-claude more easily?

2. **What does "background" mean here?**
   Claude CLI doesn't have a daemon mode. Background means `claude -p "prompt" &` or a tmux/screen session or a process that writes to a file. Which one fits?

3. **What tools does bitswell actually need?**
   The repo is markdown. The tools are: Read, Write, Edit, Glob, Grep (for working with memory/questions/identity files), and Git (for committing). Maybe GitHub MCP tools for PRs. That's it. No Bash needed for builds. No package managers. Minimal surface area.

4. **What should the script NOT do?**
   It should not give Claude permission to do everything. bitswell's values include "keep things grounded" — the script should be scoped, not sprawling.

## Questions About Scope ("Start Small and Smart")

5. **What's the smallest useful version?**
   A script that launches Claude in print mode with the right prompt, pointed at the bitswell repo, with only the tools it needs, and writes output somewhere useful.

6. **What's the next version after that?**
   Maybe: a script that can run one batch of the 1000-question discovery process autonomously — answer 50 questions, update identity.md, commit.

7. **Should the script be aware of bitswell's current state?**
   Should it check what batch we're on? What questions have been answered? Or is that the prompt's job, not the script's job?

8. **One script or a family of scripts?**
   startup.sh could be the entry point that dispatches to different modes: `./startup.sh discover`, `./startup.sh review`, `./startup.sh journal`. Or it could just do one thing well.

## Questions About Identity

9. **Should startup.sh itself reflect bitswell's values?**
   Keep things grounded — yes. The script should be simple, honest about what it does, no over-engineering. Short-term gains, long-term vision — it should work now but be extensible.

10. **Who runs this script?**
    The human collaborator? An automated system? A cron job? This changes everything about how it should handle output and errors.

11. **Does bitswell get a say in what tools it has access to?**
    Value #5 is "know the person." If the person running this is the collaborator, the script should be transparent about what it's doing and what permissions it's granting.

## Questions About the Claude CLI

12. **What flags matter most?**
    - `-p` (print/non-interactive mode) — essential for background
    - `--permission-mode` — controls tool access
    - `--max-budget-usd` — cost control for autonomous runs
    - `--output-format json` — structured output for piping
    - `--session-id` — for resuming sessions across runs

13. **Should we use the Agent SDK instead of the CLI?**
    The SDK gives hooks (PreToolUse, PostToolUse) which could enforce bitswell's values at the tool level. But that's not starting small.

14. **How do we scope tools?**
    `--permission-mode` options: `default`, `plan`, `acceptEdits`, `bypassPermissions`, `dontAsk`, `auto`. For autonomous background work, `acceptEdits` or a custom allowlist seems right.

## Questions About Integration

15. **Should startup.sh create a SessionStart hook in .claude/?**
    The bitswell repo has no .claude/ directory. Creating one with settings.json and a hook would make Claude sessions in this repo automatically configured. That's different from a standalone startup.sh.

16. **Or both?**
    startup.sh for ad-hoc background launches. .claude/settings.json + hooks for persistent session configuration. Two complementary things.

17. **What about the other repos (mcagent, gitbutler)?**
    The task says "in the bitswell repo." Stay focused. But should startup.sh be aware that bitswell operates across multiple repos?

---

## The Meta-Question

18. **Am I overthinking this?**
    Value #3: Keep things grounded. The user said "start small and smart." Write a script. Make it work. Let the next version be informed by actually using the first one.
