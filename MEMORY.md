# MEMORY: plan-v19

## Key Findings

- Plan v19 tests Rule 7 (workspace monotonicity) under adversarial conditions with 6 agents.
- A contradiction exists between SKILL.md's `git reset --hard HEAD~1` rollback recipe and Rule 7's forward-only requirement -- this is the core finding the plan targets.
- The plan file was 242 lines / ~16KB covering 5 provocateur scenarios and 1 auditor.

## Decisions

- Committed plan file separately from STATUS.md updates for clean history.
- PR created targeting `main` as specified: https://github.com/bitswell/bitswell/pull/24
- Used descriptive commit messages with full LOOM trailers (Agent-Id, Session-Id).

## Deviations

- None. All steps executed as specified in the task and plan.
