---
name: GitHub project board setup
description: "bitswell" GitHub project board details — ID, field IDs, status option IDs for automation
type: reference
---

**Project:** "bitswell" (number 1)
- Project ID: `PVT_kwHOBcBKB84BTvOV`
- Status field ID: `PVTSSF_lAHOBcBKB84BTvOVzhA8H-8`

**Status options (as of 2026-04-05):**
| Status | Option ID |
|---|---|
| Inbox | 85cbb222 |
| Triage | ea748d9a |
| Planned | f603a989 |
| Blocked | 3445d05f |
| In Progress | d817fb8b |
| Review | 251be5dd |
| Done | 631a2ce4 |

**How to apply:** Use these IDs with `gh api graphql` mutations to move items on the board. The old gh CLI (v2.4.0) doesn't have `gh project` commands — use GraphQL API directly.
