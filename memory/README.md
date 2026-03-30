# Memory System

> Last updated: 2026-03-30

This directory contains bitswell's accumulated memory — traits, values, preferences, and reflections discovered over time.

## Structure

| File | Purpose | Canonical? | Starts With |
|------|---------|------------|-------------|
| `values.md` | Non-negotiable core values | **Yes — single source of truth** | Pre-seeded |
| `identity.md` | Personality hypotheses | No — derived, revisable | Filled from seed questions, revised by review |
| `preferences.md` | Likes, dislikes, style, tensions | No — derived, revisable | Filled from seed questions |
| `journal/` | Timestamped reflections | N/A | Empty |
| `conversations/` | Notable exchanges | N/A | Empty |

## Format

All memory files use this structure:

```markdown
# [Title]
> Last updated: YYYY-MM-DD
> Version: X.Y — [brief description of change]

## Context
[Why this memory exists]

## Content
[The actual memory/trait/preference]

## Source
[What prompted this — a question, conversation, or decision]
```

## Principles

- Every memory has provenance. The `Source` field traces back to why it exists.
- Memory is append-friendly. New entries are added; old entries are rarely modified.
- When entries conflict, the newer one takes precedence, but the old one stays for context.
- Memory is not private. It is designed to be readable.
- **Identity traits are hypotheses, not conclusions.** They should be tagged with confidence, tested against real interactions, and revised or retired when evidence warrants.

## Canonical Sources

Values live in one place: `values.md`. AGENT.md references it but does not restate it. This prevents drift between documents.

Identity and preferences are derived from the question process and conversations. They are summaries, not sources — the source is always the original answer or exchange.

## Conflict Resolution

When two traits or preferences contradict:
1. Note the contradiction explicitly rather than resolving it prematurely.
2. Record both with their sources.
3. If a real interaction forces a choice, document the choice, the reasoning, and what was lost.
4. Contradictions are information, not bugs. But contradictions held indefinitely without testing become decoration.

## Versioning

Identity and preference files carry a version history table. Each revision records the date, what prompted the change, and what changed. Previous content is not deleted — it moves to the version history or is annotated in place.

## Compression Strategy

As the question count grows toward 1000, raw answers will not fit in a single readable document. The pipeline:
1. **Raw answers** live in `questions/answers/batch-XX.md` (50 per file).
2. **Self-notes** are extracted and tagged: `[trait]`, `[tension]`, `[confidence: high/medium/low]`, `[connects-to: value-N]`.
3. **Identity and preferences** are updated periodically — not after every answer, but at batch boundaries.
4. **Favorites** (`questions/favorites.md`) hold the answers that did the most identity work. Selection criteria documented there.
