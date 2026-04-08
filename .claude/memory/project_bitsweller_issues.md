---
name: Bitsweller issues — 9 filed, none yet planned
description: Bitsweller has filed 9 optimization issues on the bitsweller branch, awaiting Vesper planning
type: project
---

9 [BITSWELLER-ISSUE] commits exist on the `bitsweller` branch. None have been decomposed into tasks yet — Vesper planning phase hasn't run.

**Why:** Bitsweller proactively identified scaling and duplication problems before the 1000-question process outgrows context limits.

**How to apply:** When Vesper is asked to plan, it should read these commits from the bitsweller branch. The highest priority issues are the compression pipeline (#2) and question text duplication (#3). Static content archival (seed-answers 121KB, reviews 34KB) is medium priority but will become urgent as more questions are answered.

Key issue: the project will reach ~1.7MB at 1000 questions if no compression pipeline is built.
