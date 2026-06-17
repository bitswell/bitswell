# Team 2 — Review (reviewer: sable)

**Verdict**: APPROVED WITH EDITS

The proposal is angle-faithful, load-bearing citations check out against the
actual source of `pr-create.ts` and `pr-retarget.ts`, and it refuses — in every
section that invites temptation — to smuggle in code. That is the bar it was
asked to clear, and it clears it. The reason this is not a straight APPROVED is
that the recipe punts on one thing it claims to have solved (review staleness
across retarget) and hand-waves one thing it pretends is free (cohort
recognition). Neither is fatal. Both are fixable in a subsequent round, and the
writer has enough range to fix them without betraying the angle.

---

## 1. Coverage audit

All seven required RFP sections are present, each under the exact header the
OUTLINE specified:

| § | Header | Present | Thin? |
|---|---|---|---|
| 1 | Angle statement | yes | no |
| 2 | Thesis | yes | no |
| 3 | What changes | yes | no — by far the densest section, as it must be for a "nothing changes" angle |
| 4 | Branch naming and scope | yes | no |
| 5 | Merge vs rebase | yes | no |
| 6 | Worker authority | yes | no |
| 7 | End-to-end example | yes | no — traces every tool call with arguments, as the outline required |
| 8 | Risks and rejected alternatives | yes | no |

Plus §9 Summary, Appendix A (key references table), Appendix B (the recipe page
in outline). Nothing is missing. The proposal is if anything long — 792 lines
against a required seven-section minimum — but the length is earned by the
§7 worked example, which is the proposal's best asset.

## 2. Angle fidelity audit

The angle is "convention-only, zero code changes." I hunted for drift in the
places drift usually lives — the end-to-end example, the risks section, and
the worker-authority section (where "just a small hook" tends to crawl in).
I found none.

Specifically:

- **§3 never proposes TypeScript.** Every "changes" subsection ends with "none."
  §3.2's "one new file" is a markdown recipe page in the plugin skill
  `references/` directory. That is documentation, not code, and sits alongside
  four existing peer pages documented in `plugins/loom/skills/loom/references/`.
- **§5 refuses the rebase path outright.** No "we could rebase just the top
  layer" escape hatch. The recipe's answer to amendment is "do not amend."
- **§6 does not invent a new role.** The "single new orchestrator responsibility"
  (§6.3) is explicitly a *read* — a topological sort over `Dependencies:`
  trailers already in the commit log. It does not widen the `roles: ['orchestrator']`
  gate or create a new sub-role.
- **§7's tool-call surface (7.10) is summed honestly: 2 pr-create with non-main
  base, 2 pr-retarget fixups.** No phantom tool calls smuggled into the count.
- **§8.2 rejected alternative 2** is a `stack-create` MCP tool. The proposal
  rejects it on exactly the grounds the angle requires — that shipping a tool
  locks in the recipe's shape before there is evidence it is the right shape.
  This is the single most load-bearing rejection in the document and it is
  argued correctly.

No paragraphs drift. The angle is held across all eight sections.

## 3. Sharp-edge audit

The RFP issue #74 names five sharp edges. The proposal's treatment of each:

**Sharp edge #1 — "loom-tools already supports custom PR bases."**
Named explicitly in §2 (thesis) with the full quote from issue #74, and
discharged structurally by the entire proposal. The factual backing is correct:
I verified `src/tools/pr-create.ts` lines 6–11 (unrestricted `base: z.string()`)
and lines 32–42 (verbatim `--base` forwarding to `gh pr create`). This is the
strongest part of the proposal. Specific answer: "the mechanical surface is
already here; we are writing a recipe, not code."

**Sharp edge #2 — Dependency DAG already exists.**
Named in §3.3 and §7.3. Specific answer: the topological walk of an existing
`Dependencies:` trailer chain *is* the stack-ordering input — no new metadata
required. `dag-check` is namechecked as the existing validator. This is sound
as far as it goes (see §5 below for where it stops).

**Sharp edge #3 — Branch naming conflict.**
Named in §4.3. Specific answer: the recipe refuses `gh stack`'s `-p` prefix
model. LOOM branches stay `loom/<agent>-<slug>`; the recipe only *reads* those
names and passes them as `base:` strings. This is probably the cleanest answer
in the proposal — there is no name collision because there is no new name.

**Sharp edge #4 — Merge vs rebase / audit trail incompatibility.**
Named in §5.2 and called "moot" with the specific mechanism: the recipe never
invokes `gh stack`'s rebase commands, so the incompatibility never arises.
The §5 conceptual claim — *stacked PRs and stacked-branches-managed-by-gh-stack
are not the same thing* — is correct and load-bearing, and it is the one
insight of this proposal that might survive into other proposals even if this
one loses. Keep it.

**Sharp edge #5 — Worker-vs-orchestrator authority.**
Named in §6 and answered structurally: the worker side of the protocol never
learns that stacking is happening. §6.5's invariant-count table is explicit
about the count of relaxed invariants (zero). This is a hand-wave-resistant
way to make the claim; credit given.

**Verdict on sharp edges**: named, specific, no hand-waves. Four of the five
are answered cleanly. Sharp edge #2 is answered at a structural level but
leaves a gap — see §5 below.

## 4. Feasibility audit

Feasibility is unusually easy to audit for a zero-code proposal: the
implementation work is "write one markdown file" and "add one line to
SKILL.md." The proposal names the target locations concretely
(`plugins/loom/skills/loom/references/stacked-prs.md`, SKILL.md references
list). I verified the references directory layout expectation: the four
peer pages (`examples.md`, `protocol.md`, `schemas.md`, `worker-template.md`)
exist at exactly the cited path. Appendix B provides the recipe page outline
at enough detail (sections, approximate line count) for bitswelt to judge
what would ship.

**Honest sizing check.** Appendix B promises a ~300–500-line markdown page.
The outline has six top-level sections and a worked example. 300–500 is
reasonable for that scope. Not underestimated.

**What is not sized honestly.** The proposal repeatedly claims the recipe
"can ship the afternoon it is approved." That is only true for the *code*
delta. It is not true for the *agent prompt* delta: every orchestrator run
that exercises the recipe has to remember to call it. The cost of the recipe
is paid at inference time by the orchestrator agent, not at implementation
time by the implementer. The proposal does not name this cost, and it is the
only feasibility claim I would push back on. It is, however, a small enough
push-back that I would not make it a blocker — the cost exists in any angle
whose output is documentation.

**One genuine hidden complexity.** §7.6 step 5 confronts the
worktree-does-not-contain-dependency problem and resolves it in three bullets.
The resolution is correct but understated: bullet 2 says "`integrate()` is
responsible for merging `ratchet`'s work into `main` before `moss`'s branch is
merged," which gets the direction of causality right but glosses that in the
stacked flow `integrate()` is *delayed* until GitHub's merge button runs. The
proposal is not wrong, but a skeptical reader could read §7.6 and §7.9 in
sequence and conclude that `integrate()` is running twice (once conceptually
to satisfy the dependency, once via the merge button). A one-sentence
clarification would close this. See §8 below.

## 5. The cut — what could sink this in the winner-selection round

The proposal claims `pr-retarget` is the existing fixup for base-branch
deletion, and in §7.9 and §8.1-risk-2 treats the retarget as a harmless
cosmetic operation. That is only true if nobody is reviewing PR #102 during
the window between "PR #101 merged" and "orchestrator ran pr-retarget."

**The unaddressed problem is review staleness across retarget.** A reviewer
who approves PR #102 while its base is `loom/ratchet-feat-auth` is approving
a diff of ~10 API files. After the orchestrator runs
`pr-retarget { number: 102, base: "main" }`, GitHub re-renders PR #102 as a
diff against `main` — which, if there was any conflict between `ratchet`'s
auth code and unrelated advances on `main` since the review began, now contains
additional resolution commits the reviewer never saw. The approval from the
"reviewed against a sibling branch" state *carries forward* to the "reviewed
against main" state, silently.

The proposal handles the catastrophic-drift case (§5.4) by invoking the
standard conflict recovery path. It does not handle the *benign* case where
the re-rendered diff merely contains three or four new files the approver
did not approve. GitHub's native PR-review UI does not force re-approval on
a base change. The recipe, as written, relies on the orchestrator not merging
a PR whose approval predates a retarget — and nowhere in §7.9 is that rule
stated.

This is not a reason to kill the proposal. It is a reason to amend §8.1 or §7.9
with a two-sentence rule: *after any `pr-retarget`, the recipe requires a
fresh reviewer approval before the retargeted PR is eligible for merge*. That
rule is zero-code (it is a recipe instruction) and it is angle-faithful (it
adds text to the documentation page, not logic to a tool). The writer can
amend this in the implementation round; I am flagging it here.

**Secondary weakness — cohort recognition is unspecified.** The §6.3 "read-only
new responsibility" is a topological sort over "a cohort of ASSIGNED branches."
What defines *cohort*? In a long-lived LOOM orchestration, many ASSIGNED
branches may exist simultaneously, some with `Dependencies:` chains that the
orchestrator has no intent to ship as a stack. The recipe does not say how the
orchestrator decides that a particular chain is a stack candidate. §7.1 quietly
assumes the epic is trio-shaped and the three agents are obviously one cohort;
that works for the 3-agent worked example and nowhere else. In a realistic
run, cohort boundaries are a judgment call — which is fine, because the
convention-only angle is explicitly a hypothesis-shaped experiment, but the
proposal should say so rather than naming it "read-only topological sort" as
if the hardest part were sort-order.

**Tertiary — bottom-up merge cascade ownership.** §7.9 depicts the
orchestrator serialising three merges manually. The recipe's whole "preserves
LOOM invariants" claim depends on that serialisation happening in a specific
order. Nowhere is there a rule that says "the orchestrator MUST complete each
retarget before authorising the next merge." The risk is not theoretical:
GitHub's merge-queue feature is mentioned in §8.1 risk 6 as "disabled for
stacked epics," but "disabled" is a configuration claim, not a recipe rule.
A follow-up round will likely need the stronger rule: *the orchestrator is
the sole merge authority for PRs in a stacked cohort.* One line in the recipe.

## 6. Strongest arguments

The single strongest move in the proposal is §5.2's insight that **stacked PRs
and stacked branches managed by `gh-stack` are two different things**. This
reframing is correct, it is specifically attributable to the angle, and it is
the reason the rest of the proposal can exist. If the proposal loses the
winner-selection round, this insight should survive into whatever wins — it
is genuinely new thinking, not just a stingy variant of the other angles.

The second strongest move is §3's section-by-section "no changes" enumeration,
which converts the angle from a claim ("zero code") into an audit
(each LOOM component, checked). This is a documentation technique other angles
will not be able to copy without doing real work, and it makes the proposal
disproportionately hard to argue against on a per-component basis.

The third is §8.2 rejected-alternative 2 (the `stack-create` MCP tool). A
convention-only proposal is *most* tempted to defect here — "we'll wrap the
recipe in a small tool, it's nicer" — and the proposal explicitly refuses with
the strongest available reason: that shipping a tool locks in the recipe's
shape before there is evidence the shape is right. This is the proposal
earning its angle.

## 7. Weakest arguments

The weakest single move is §7.9's framing of `pr-retarget` as "the exact case
this existing tool was built for, no new code path is exercised." Both halves
of that sentence are mechanically true. Both halves are also insufficient,
because the mechanical operation is trivial but the *semantic* operation —
changing what a PR means after it has been reviewed — is not. The proposal
never names the semantic operation. A reviewer selecting between angles will
notice this gap and may read the rest of the proposal more suspiciously for
missing it.

The second-weakest move is the repeated "can ship the afternoon it is
approved" framing. It is technically true for a zero-code delta and it is the
emotional hook the angle wants — but it systematically undercounts the
inference-time cost (the orchestrator prompt has to learn to invoke the
recipe) and the review-convention cost (reviewers have to learn the stack-
approval contract). An opponent in the winner-selection round will point out
that "zero code" and "zero cost" are not the same thing, and this proposal
does elide the distinction more than once.

## 8. Suggested edits to proposal.md

I am **not** making any edits to proposal.md. Justification:

The review-staleness-across-retarget problem is a substantive omission, not a
factual error; the writer will want to handle it in their own voice in the
next round rather than having it grafted in. The cohort-recognition and
merge-cascade-ownership gaps are similarly substantive. My editorial authority
is for factual errors, missing sections, and clarification of plainly-ambiguous
claims — none of the three fall into those categories.

I audited the specific citations that would have been in-scope for a
fact-check amendment:

- `pr-create.ts` lines 6–11 (schema) — correct.
- `pr-create.ts` lines 32–42 (handler, `--base` forwarding) — correct.
- `pr-create.ts` line 27 (`roles: ['orchestrator']`) — correct.
- `pr-retarget.ts` lines 6–9 (schema) — correct.
- `pr-retarget.ts` lines 30–34 (handler, `gh pr edit --base`) — correct.
- `pr-retarget.ts` line 25 (`roles: ['orchestrator']`) — correct.
- `protocol.md` §3.3 `integrate()` lines 86–98 — correct.
- `protocol.md` §6.1 trust boundary table — correct (verified at lines 157–165).
- `protocol.md` §8.2 audit trail quote "Every state change is a commit..." —
  quote is verbatim correct; the line number cited (183–184) is off by ~2
  against the current plugin cache (actual ~185–187). Drift of two lines in a
  doc that may have been edited since the citation. Not worth amending; the
  quote itself is exact and the `§8.2` anchor is stable.
- `schemas.md` §3.3 `Dependencies:` trailer — correct.
- `schemas.md` §2 branch naming — correct.
- `gh-stack` SKILL.md line 789 "Stacks are strictly linear" — exact match.

No factual amendments warranted. No missing sections. No plainly-ambiguous
claims that a fact-check amendment could fix without rewriting the writer's
voice.

If the writer revises for a later round, the two things I would ask them to
add are:

1. A one-paragraph rule in §7.9 step 8 or §8.1 risk 2: "after any
   `pr-retarget`, the retargeted PR requires a fresh reviewer approval before
   merge, regardless of prior approval state." This closes the review-staleness
   gap and costs one paragraph of recipe text.
2. A one-sentence definition of *cohort* in §6.3: how the orchestrator decides
   which ASSIGNED branches form a stack candidate versus which form independent
   PRs. Honest answer is probably "the orchestrator decides per-epic based on
   whether per-layer review is desired," which is fine — just say so.

These are suggestions, not required edits. The writer owns the voice.

— Reviewed by sable
