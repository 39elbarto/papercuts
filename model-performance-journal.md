# Model performance journal — papercuts

## 2026-07-09 - grok-4.5-fast-xhigh via cursor - adversarial design review (plan r1)

Command and run: `delegate --group papercuts-plan-review cursor safe --prompt-file _scratch/plan-review-prompt.md`; alias `cursor-1`; mode/isolation: safe/isolated copy.

Task and expectation: adversarial review of the papercuts design doc against the rust-agent-cli skill, 7 named hunt areas; expected a ranked findings list.

Outcome and verification: excellent. 2 true blockers (ID-determinism/doctor contradiction; torn-last-line unspecified) + ~14 majors/minors, near-zero noise. Notably resourceful: safe isolation blocked the repo's skill symlink, and the lane hunted down the canonical skill path on its own instead of reviewing blind. Coordinator triage accepted ~80% of findings; rejections were genuine judgment calls, not errors.

Performance observations: ~7 min wall. Findings arrived well-tabled with severities (its known flat-intensity weakness was mostly absent). One background search hung at the end without affecting output.

Routing assessment: confirms Cursor/Grok-4.5 as the fleet's first-choice design reviewer; use again unhesitatingly. Confidence: high.

## 2026-07-09 - gpt-5.6 codex (sol, xhigh) via codex - adversarial design review (plan r2)

Command and run: `delegate --group papercuts-plan-review codex safe --model sol --reasoning-effort xhigh --prompt-file _scratch/plan-review-prompt-r2.md`; alias `codex-3` (runs codex-1/codex-2 failed pre-model: work-account quota exhaustion, then an expired restored personal token — harness failures, no model signal).

Task and expectation: fresh-eyes review of the once-amended r2 doc, hunt areas aimed at the amendments themselves.

Outcome and verification: outstanding — 1 blocker + 12 majors + 1 minor, and triage accepted every single one (unprecedented zero-reject round). The blocker (content-addressed ID ≠ retry-idempotency; ts changes on retry) was a real reasoning flaw both the coordinator and the first reviewer missed. Caught doc-internal inconsistencies (doctor --fix ghosts in two sections, sort-order contradiction between synopsis and normative fold) and deep contract holes (write() vs write_all partial-write poisoning, lock-hang with no timeout policy, missing-file semantics). Checked local rustc File::lock docs before opining on locking.

Performance observations: ~12 min wall at xhigh. Findings precise, each with concrete failure scenario + 1-2 line fix; report format followed exactly. Zero fabrication; zero scope creep.

Routing assessment: Sol xhigh is a frontier-grade design reviewer — for contract-dense specs it outperformed the same doc's first-round review on depth (different axis: precision holes vs design contradictions; the two rounds were complementary, validating the two-family sequence). Use Sol xhigh for judgment-dense spec gates; keep Cursor for artifact-probing reviews. Confidence: high.
