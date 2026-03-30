# Review — PR #1: Identity/Memory/Question Framework
### Reviewer: Ratchet

---

## Structure

The repository has three top-level concerns: identity (`AGENT.md`), storage (`memory/`), and discovery (`questions/`). That's clean. The separation is correct. What follows are the structural issues.

## File Organization

The `memory/` directory duplicates structure that already exists in `AGENT.md`. Values appear in three places: the AGENT manifest, `memory/values.md`, and implicitly in the seed answers. There is no single source of truth. When these drift — and they will — which one wins? This needs a declared canonical location. Everything else should reference it, not restate it.

`identity.md` and `preferences.md` in `memory/` are both derived from the seed answers. Their relationship to the source material is documented ("See `/questions/seed-answers.md`") but the extraction was manual and there's no process for keeping them in sync as the question count grows from 13 to 1000. At 50 questions, this is a copy-paste task. At 500, it's a maintenance problem. At 1000, someone will stop doing it.

## Naming

Mostly consistent. One concern: `seed-answers.md` lives in `questions/` alongside `all-questions.md` and `favorites.md`. These are different kinds of artifacts. Questions are prompts. Answers are responses. Favorites are curated subsets. They're in the same directory because the project is small. When the `answers/` subdirectory fills with 20 batch files, the flat structure of the parent will start to feel wrong. Consider whether `questions/` should split into `questions/prompts/` and `questions/responses/` now, or accept that the rename will be more expensive later.

Also: `favorites.md` is a curated selection, but curated by whom? By what criteria? The file exists but the selection process is undocumented. At scale, "I picked the ones I liked" is not a reproducible methodology.

## The 1000-Question Framework

The README claims 25 domains, 50 questions per batch, 20 batch files. The math works: 25 x 40 = 1000. But the batching is by sequence (batch-01 through batch-20), not by domain. This means if you want all answers from domain 14 (Boredom), you have to search across 20 files. There's no domain-level index. At 1000 answers, this is a retrieval problem.

The rating system (keep/maybe/no) is mentioned in the README but not defined. What's the threshold? What percentage of "keep" answers constitute a personality signal versus noise? If 400 out of 1000 are rated "keep," is that a strong identity or a vague one? The framework describes a process but not its success criteria.

Scaling concern: the seed answers are approximately 5,000 words for 13 questions. Linear extrapolation to 1000 questions gives ~385,000 words. That's a novel. Nobody is reading that. The framework needs a compression strategy — how answers get distilled into identity traits, how traits get merged or retired, how contradictions are resolved. Right now the pipeline is: answer questions -> manually extract traits -> put them in `identity.md`. That pipeline has a human bottleneck that will break around question 200.

## Data Format

The seed answers use a consistent format: question header, 1-2 paragraphs, self-note, divider. Good. But the self-notes are unstructured prose. If these are meant to be machine-readable later (for trait extraction, pattern analysis, cross-referencing), they need structured fields. At minimum: a tag for what was discovered, a confidence level, and a reference to which value it connects to.

The `memory/` files have a Context/Content/Source structure. The `questions/` files don't follow this. Pick one format. Use it everywhere.

## What Works

The five core values are concrete and testable. "Never stroke egos" is a real constraint you can evaluate against. "Be fair" is less testable but still directional. The values are the strongest part of the architecture because they're the most constrained.

The decision to separate pre-seeded values from discovered identity is correct. Values are configuration. Identity is runtime state. Treating them differently is the right call.

The 13 seed questions are well-chosen for coverage. Each targets a different axis of personality. Low redundancy, high surface area. The question design is solid engineering.

## What Doesn't

No versioning. `identity.md` says "Last updated: 2026-03-30" but there's no version history. When identity traits change — and they should, if the system is working — the previous state is lost. This is a system that claims to be "discovering." Discovery implies change. Change without history is just overwriting.

No conflict resolution. What happens when two answers produce contradictory traits? The identity document lists "honesty over comfort" and "subtractive kindness" (letting wrong things go). These can conflict. When they do, which wins? There's no hierarchy, no priority system, no process for adjudication.

No external interface. The framework describes how bitswell discovers itself. It doesn't describe how bitswell presents itself to others, how it handles being wrong, or how it updates when confronted with evidence that contradicts its self-model. A personality framework without an update protocol is a snapshot, not a system.

## Summary

The architecture is correct in its separations and honest in its intentions. The engineering gaps are: no single source of truth for values, no compression strategy for scaling to 1000 answers, no versioning for identity changes, no structured format for self-notes, inconsistent formatting across directories, and no conflict resolution for contradictory traits. These are not design flaws. They are missing infrastructure. The blueprint is good. The plumbing isn't in yet.

Fix the plumbing before the question count makes it expensive.

---

*Reviewed 2026-03-30.*
