---
name: Bitswell codebase memory profile
description: Key memory/size characteristics of the bitswell repo discovered during analysis — hotspots, ratios, and patterns
type: project
---

Bitswell is a markdown-only AI identity discovery system. No build step, no runtime code except startup.sh (bash).

**Size breakdown (as of 2026-03-31, Q1-Q10 answered):**
- agents/ = 280KB (7 agents x 5 files each = 35 files). 73.7% is static (seed-answers + reviews). Only 26.3% is active state.
- questions/ = 72KB. all-questions.md (13KB for 100 Qs) is the master list. batch-01.md is 17KB for 10 answers.
- memory/ = 40KB. identity.md (4.7KB) is the main living document. startup-questions.md is orphaned.
- Total tracked content: ~400KB

**Biggest context cost centers:**
1. Agent seed-answers: 121KB (8 files, all marked "Complete")
2. Agent reviews: 34KB (7 files, all for PR #1 only)
3. questions/seed-answers.md: 17KB
4. questions/answers/batch-01.md: 17KB
5. questions/all-questions.md: 13KB (100 Qs, will reach ~130KB at 1000)

**Growth vectors:**
- Batch answers: ~17KB per 10 questions. At stated goal of 50/batch x 20 batches = ~1.7MB
- identity.md: 4.7KB after 10 questions. Grows with each batch if extraction continues.
- all-questions.md: linear with question count (~130 bytes per question)

**Why:** These numbers guide where optimization effort has highest ROI. The static-vs-active ratio in agents/ is the single largest inefficiency.

**How to apply:** Prioritize issues that address the 73.7% static archive weight in agents/. Second priority is the batch-answer scaling path.
