# Team 5 — Review: Stack-Mode Replaces `--no-ff` for Opted-In Epics

**Verdict**: APPROVED WITH EDITS

---

## 1. Verdict

**APPROVED WITH EDITS.** The proposal stays true to its angle, covers all
seven RFP sections with specificity, and commits cleanly to replacement
rather than hedging. It has three real weaknesses — an over-attributed
protocol citation, a missing content-equivalence check in `stack-land`,
and a seam where `gh stack submit --auto` races `stack-land.ts`'s wait
loop — but none of them puncture the angle, and all are fixable without
re-architecting. One factual amendment was made to proposal.md (§8).

---

## 2. Coverage audit

Seven RFP-required sections, each checked against proposal.md:

| RFP section | proposal.md home | Status |
|---|---|---|
| 1. Angle statement | §1, §2.3 | Present. One-sentence claim plus explicit "replacement not augmentation" paragraph. |
| 2. What changes | §3 (13 sub-sections) | Present and exhaustive. Names every affected file, including four "no change" declarations for falsifiability. |
| 3. Branch naming and scope | §4 (six sub-sections) | Present. `loom/*` retention, `--adopt`, two-checkpoint scope enforcement, cross-stack exclusion. |
| 4. Merge vs rebase | §5 (eight sub-sections) | Present and is the strongest section. Itemizes the five guarantees of `--no-ff` and rules each one either REPLACED or DROPPED. |
| 5. Worker authority | §6 (six sub-sections) | Present. Invariant table in §6.4 is the clearest single artifact in the document. |
| 6. End-to-end example | §7 (four sub-sections) | Present. Side-by-side merge-mode / stack-mode trace, contrast table, conflict path. |
| 7. Risks and rejected alternatives | §8 (eight risks + five rejected alternatives) | Present. Exceeds the "two rejected alternatives" floor by 2.5x. |

No section is missing. No section is thin by RFP standards. §2 "Thesis"
and §9 "Summary" are additions beyond the seven required sections, but
they strengthen rather than pad.

---

## 3. Angle fidelity audit

**Team 5's angle**: "stack-mode replaces `--no-ff` merges for opted-in
epics" — no preservation, no parallel projection, no hedging.

The proposal stays on-angle throughout. Concrete evidence of fidelity:

- §1 paragraph 3 explicitly refuses parallel projection: "we explicitly
  do NOT keep `--no-ff` merges as a parallel projection on top of the
  stack (that is team 1's angle, and we reject it in §8)."
- §2.3 "Why replacement, not parallel projection" argues the case
  affirmatively and names team 1's angle as the rejected sibling.
- §5.1–§5.7 itemizes what `--no-ff` carried and decides each item
  either REPLACED or DROPPED. Nothing is PRESERVED-AND-STACK-TOO.
- §6.4's invariant table marks "Integration produces a merge commit" as
  **broken (by design)** and "`git log --first-parent main` = epic list"
  as **broken (by design)**. The proposal does not pretend these
  invariants survive.
- §8.2 Rejected Alternative 2 is exactly the parallel-projection hedge,
  and the rejection is unambiguous.

**No drift found.** No paragraph softens the replacement claim or
suggests merge-mode epics get any of stack-mode's benefits without
opting in. The angle contract is honored.

One place the angle could have been even sharper: §5.8 claims "linear
is strictly better" for the target regime but does not nail down what
"small-to-medium" means operationally. The proposal leaves this
decision to humans at opt-in time, which is defensible but slightly
fuzzy. Not a drift — the angle is about replacement, not about when to
opt in — so this reads as a deliberate out-of-scope.

---

## 4. Sharp-edge audit

The five RFP sharp edges, each checked against the proposal's response:

**Sharp edge 1 — loom-tools already supports custom PR bases.**
Named in §3.7 (pr-create.ts, no change) and §3.8 (pr-retarget.ts, no
change). The proposal's answer is that these existing tools are simply
not called on the stack-mode path — `gh stack submit --auto` creates
and bases the PRs instead. **Specific, non-hand-wavy.** The proposal
does not claim credit for reusing these tools; it cleanly sidesteps
them in stack-mode, which is consistent with its replacement angle.

**Sharp edge 2 — Dependency DAG exists.** Named in §3.5, §4.2, §7.2.
The proposal uses the existing `dag-check.ts` topo-sort output as the
topological order input to `gh stack init --adopt` (§4.2, §7.2 step 2).
This is correct reuse. It then proposes a NEW check inside `dag-check.ts`
to reject fan-out DAGs for stack-mode epics (§3.5). The proposal is
honest that this is added code, not an existing capability.

**Sharp edge 3 — Branch naming conflict (loom/ vs gh-stack prefix).**
Named in §4.1–§4.3. The resolution: keep `loom/<agent>-<slug>` on
workers, orchestrator uses `gh stack init --adopt` with full branch
names (no `-p` prefix), and no `stack/*` namespace is created.
**Concrete and falsifiable.** The proposal explicitly states "no new
refs are created" and explains why dispatch continues to scan `loom/*`
unchanged.

**Sharp edge 4 — Merge vs rebase.** This is §5, the core section, and
it is the strongest part of the proposal. Each of the five guarantees
`--no-ff` carried is named, and each is either REPLACED (a, b, e) or
DROPPED with documented mitigation (c, d). No hand-wave.

**Sharp edge 5 — Worker-vs-orchestrator PR authority.** Named in §6.
The answer: workers NEVER run `gh stack` commands; the orchestrator
runs all of them in its dedicated integration worktree. The one new
worker obligation is stamping `Epic-Id`, which is a trailer-only
change. §6.4's invariant table is the cleanest articulation of what is
preserved, strengthened, or broken.

**All five sharp edges are named and each has a specific answer.** No
hand-waves detected.

---

## 5. Feasibility audit

Implementation work, by component, sized against my reading of the
existing loom-tools codebase:

**Realistic:**

- `pr-merge.ts` short-circuit (§3.1): trivial. Read the root ASSIGNED
  commit, check a trailer, return `err` early. An hour of work plus
  tests.
- `commit.ts` / `trailer-validate.ts` adds (§3.6): two new trailers,
  one "root-only" rule. The existing trailer-validate machinery handles
  similar rules today. A half-day of work plus tests.
- `stack-submit.ts` (§3.2): the body is essentially three `exec` calls
  (`gh stack init --adopt`, `gh stack rebase`, `gh stack submit --auto`)
  plus JSON parsing and error translation. Maybe one day of work. The
  existing tool framework makes this straightforward.
- Worker template one-line add (§3.10): trivial.
- Integration recipe branch (§3.9): a top-level `if`, a day.

**Plausible but underestimated:**

- `stack-land.ts` wait loop (§3.3). The proposal glosses over a race:
  `gh stack submit --auto --draft` creates PRs as drafts, then a human
  marks each ready, then GitHub auto-merges (or a human merges) in
  order. Meanwhile `stack-land.ts` is polling `gh stack sync` + `gh
  stack view --json`. What happens if the human *retargets* a PR's
  base manually in the GitHub UI during this window? What happens if
  the human closes a PR instead of merging? The proposal names a
  30-minute default timeout but does not say what "timeout" does
  beyond return an error — does the stack get torn down? Are the
  `loom/*` branches left in a consistent state for a retry? The
  recovery story here is thinner than the rest of the proposal.
- Post-rebase scope re-check (§3.4). The proposal says "walk the
  branch back to the pre-rebase checkpoint, which the orchestrator
  snapshot keeps for the duration of integration." There is no such
  snapshot mechanism described anywhere else in the proposal. The
  orchestrator needs to *capture* the pre-rebase branch SHAs before
  calling `gh stack rebase` and keep them reachable until post-rebase
  check completes. This is a genuine new piece of orchestrator-side
  state management that §3.4 does not call out as such. Call it
  another day of work plus careful recovery handling, not the
  "trivial additional check" tone §3.4 implies.
- Content-equivalence hash (§8.2 R2, "content hash comparison: the
  combined rebased diff should match the topo-merged pre-rebase diff
  byte-for-byte modulo formatting"). This is listed as mitigation but
  is not implemented anywhere in §3. Either drop it from R2 or add a
  §3.x entry describing where the hash lives and how it's computed.
  As-is, §8.2 R2 promises a safety net that §3 doesn't build.

**Honestly scoped losses:**

- `stack-revert.ts` is named as a helper with a clear behavior
  description (§5.5). The proposal correctly does NOT claim it
  restores atomic-revert semantics; it is explicitly "operationally
  atomic, not topologically atomic." The honesty is on-angle.
- `epic-list.ts` shim (§5.7) is one `git log --first-parent` union'd
  with `git for-each-ref refs/tags/stack-landed/`. Two-hour tool.

**Overall**: the proposal is roughly honest about scope. The
stack-land wait loop and the pre-rebase snapshot are the two places
where complexity is understated. Neither is fatal. Total work is
realistically one to two weeks for an experienced implementer, not the
"surgical change" framing §1.4 hints at.

---

## 6. Strongest arguments

The proposal's three biggest wins:

1. **§5 is the best answer to sharp edge 4 in the field.** By
   itemizing the five guarantees `--no-ff` provides and ruling on each
   one explicitly, the proposal forces the reader to count exactly
   what is lost and exactly what is replaced. No other angle in this
   RFP gets to be this precise because no other angle commits to
   replacement.

2. **The invariant table in §6.4** is the single clearest artifact in
   the proposal. Naming two invariants as "broken (by design)" and
   defending every other one as preserved or strengthened is exactly
   the kind of accounting the RFP asked for. Reviewers scanning for
   hand-waves will find none here.

3. **The rejected-alternatives list (§8.2) is the strongest defense
   of the angle.** It does not just list sibling angles; it names
   team 1's approach specifically (Rejected Alt 2) and explains why
   adopting it would hedge the replacement. The rejection of
   integration-time opt-in (Rejected Alt 1) correctly identifies the
   "rewriting COMPLETED commits changes their SHAs" problem, which is
   a real protocol-level objection, not a stylistic preference.

---

## 7. Weakest arguments

The one thing that could sink this proposal in the winner-selection
round:

**The operational cost of losing atomic revert is understated.**
§5.5 is careful and honest, but the tone of "non-trivial loss" doesn't
match the blast radius. `git revert -m 1 <merge-sha>` is the LOOM
orchestrator's emergency brake for a bad epic landing. A reviewer in
the winner-selection pass who has ever had to actually use that brake
at 2am will read §5.5 and think: "so the recovery story for a bad
stack-mode epic is 'hope `stack-revert.ts` exists and works, or revert
eight commits by hand.'" The proposal's answer is "don't opt into
stack-mode for risky epics," which is correct but reduces the
addressable market for stack-mode to "epics whose landing never needs
to be undone" — a narrower slice than §2.2's "small-to-medium feature
epics" framing implies. If the winner-selection pass weights
operational resilience heavily, this is the wound that will decide
against replacement in favor of team 1's projection angle.

Secondary weakness: the pre-rebase snapshot mechanism (§3.4) is
assumed, not specified. This is a real gap in the scope-check story
and should have been its own sub-section.

---

## 8. Suggested edits to proposal.md

**Amendments made**:

**One factual amendment** was made to proposal.md to fix a misattribution:

- §2.1 opening (line 47–54 of the previous version) attributed the
  `--no-ff` merge integration model specifically to `protocol.md §3.3`.
  In fact, `protocol.md §3.3` describes integration generically
  ("Attempt merge. On conflict: abort…") and does not mention `--no-ff`
  or `--first-parent`. The `--no-ff` prescription actually lives in
  `loom/SKILL.md` (lines 43–44 and 95) — the loom skill's integration
  recipe, not the protocol reference. I amended §2.1 to cite both
  sources accurately so the proposal does not overclaim what protocol.md
  prescribes. This correction does not change any argument in the
  proposal — the operational reality is that pr-merge.ts calls
  `gh pr merge --merge` (server-side merge commit) and the workspace
  integration recipe does a local `git merge --no-ff`. Both together
  are what the proposal replaces. It just needed to say so precisely.

**Edits considered but NOT made** (out of scope for factual/completeness
amendment, left for the writer's voice):

- §3.4 does not describe the pre-rebase snapshot mechanism. This is
  a design gap, not a factual error, and rewriting §3.4 to fill it
  would be exceeding my authority. Flagged in §5 (feasibility audit).
- §8.2 R2's content-hash mitigation is not implemented in §3. Same
  treatment — flagged, not edited.
- §5.4 claim that reflog "expires within ~90 days" is correct for
  git's default `gc.reflogExpireUnreachable`, and the proposal is
  correct to mark this as an approximate window. No change.
- §5.5 tone around operational revert-cost. Style/emphasis, not
  factual. Not edited — flagged in §7.
- §4.2 mentions "SKILL.md line ~420" for `--adopt`. Verified at
  SKILL.md line 420 — correct.
- §3.5 / §7 R4 cites "SKILL.md line ~789" for the strict-linearity
  limitation. Verified at SKILL.md line 789 — correct.
- §6.5 says `gh stack rebase` exit code 3 is rebase conflict. Verified
  against SKILL.md exit-code table (line 780) — correct.

**Edits explicitly rejected**:

- I did not soften the "replacement" language anywhere. That would
  change the angle.
- I did not add caveats to §5.5 about atomic revert being a bigger
  loss than the proposal admits. The writer made a considered choice
  about tone there; flagging it in §7 of this review is the correct
  channel.
- I did not rewrite §2.1 beyond the citation correction. The
  substance of §2.1 (what `--no-ff` buys LOOM today) is accurate.

---

*End of team 5 review for RFP #74.*
