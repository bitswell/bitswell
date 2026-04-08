---
name: Bitsweller issues filed to date
description: Index of all improvement issues filed on the bitsweller branch, with status and priority
type: project
---

10 issues filed as of 2026-04-06 on the `bitsweller` branch as commits:

Note: Issues 1-9 were on a previous bitsweller branch that no longer exists.
Issue 10 is on the current bitsweller branch (created 2026-04-06 from main).

| # | Title | Priority | Category |
|---|-------|----------|----------|
| 1 | Values restated in 3+ locations creating drift risk | Medium | Duplication/drift |
| 2 | No compression pipeline for answer-to-identity extraction at scale | High | Scaling |
| 3 | Question text duplicated across batch answers and all-questions.md | High | Duplication |
| 4 | Agent review files are static artifacts consuming 34KB | Medium | Archive lifecycle |
| 5 | Orphaned startup-questions.md in memory/ (4.6KB) | Low | Orphaned artifact |
| 6 | No agent manifest/index — 35 files require full traversal | Medium | Context efficiency |
| 7 | Inline "What I Found" sections duplicate extracted identity | Medium-high | Duplication/drift |
| 8 | startup.sh reads task file with no size guard | Low | Resource safety |
| 9 | Agent seed-answers are 121KB static content dominating agents/ | Medium | Archive lifecycle |
| 10 | Add loom-context tool for worktree project dir lifecycle | High | New tool / observability |

**Why:** Prevents filing duplicate issues in future sessions.

**How to apply:** Before filing a new issue, check this list. Update when issues are resolved or new ones are filed.
