# Papercuts Fork Project Plan

Status: initial planning baseline

Date: 2026-07-11

Fork: `39elbarto/papercuts`

Upstream: `treygoff24/papercuts`

## 1. Purpose

Build a security-hardened, agent-friendly evolution of `papercuts` that can be
deployed across many repositories without silently violating read-only task
boundaries, dirtying every worktree, or leaking sensitive context into public
Git history.

The upstream CLI already provides the valuable substrate: an append-only JSONL
journal, stable machine-readable envelopes, deterministic IDs, concurrent
writers, lifecycle events, validation, tags, severity, and Markdown review.
This project should preserve those strengths and avoid a ground-up rewrite.

## 2. Current problem

AI coding agents encounter small but repeated workflow friction while completing
larger tasks. That signal is usually lost. A durable complaint journal allows a
human or later agent to identify repeated setup, documentation, tooling, and
automation defects and fix the source of the friction.

The upstream `v0.1.0` contract has several adoption risks for broad use:

- the repository-local log is committed by default;
- free-form text is persisted without a sensitive-data guard;
- records capture absolute `cwd` and repository paths;
- a logging instruction can conflict with explicitly read-only work;
- review is per file, while our intended use spans many projects;
- there is no policy for promoting repeated cuts into issues, runbooks, CM
  rules, or automation improvements.

The fork must solve those problems without turning a small CLI into a platform.

## 3. Product principles

1. **Safe by default.** A first-time agent should not accidentally publish
   sensitive context or mutate a repository during restricted work.
2. **Agent-first contract.** Commands remain non-interactive, deterministic,
   structured, and self-describing.
3. **Append-only evidence.** Preserve journal semantics and avoid hidden history
   rewrites.
4. **Policy separated from mechanism.** The Rust CLI provides reliable storage
   and validation; repository instructions define when an agent may log.
5. **Upstream-friendly changes.** Generic fixes should remain small enough to
   propose upstream when appropriate.
6. **No premature platform.** No server, hosted account, telemetry, or mandatory
   central database in the first hardened release.
7. **File-first continuation.** Plans, decisions, worklog, tests, and release
   gates in this repository outrank chat memory.

## 4. Users and workflows

### 4.1 Agent logging a papercut

1. The agent encounters minor workflow friction.
2. It checks whether the current task permits writes or external logging.
3. It submits a concise description and tag.
4. The CLI checks size, path policy, and likely sensitive content.
5. The CLI either appends safely or returns a structured refusal/warning with a
   useful remediation.
6. The agent continues the main task without investigating the papercut unless
   it is a blocker.

### 4.2 Operator reviewing one project

1. Run `doctor` to validate the journal.
2. List open cuts as Markdown or JSON.
3. Group repeated cuts by tool or workflow area.
4. Fix or consciously decline a cut.
5. Append a resolution containing a durable reference where useful.

### 4.3 Operator reviewing many projects

1. Discover configured project journals without scanning arbitrary private
   paths.
2. Produce a bounded cross-project digest.
3. Cluster only for review; never rewrite source journals automatically.
4. Promote high-value findings to the correct durable destination:
   repository issue/Bead, runbook, helper, CM rule, or ClickUp workstream.
5. Resolve source cuts only after verification.

## 5. Scope

### 5.1 First hardened release

- explicit safe/private storage mode and documented discovery behavior;
- an option to omit or relativize `cwd` and repository paths;
- a bounded sensitive-data preflight with structured warnings/refusals;
- dry-run coverage for every policy decision;
- canonical `AGENTS.md` instructions that respect read-only boundaries;
- upgrade and upstream-sync documentation;
- tests proving no sensitive paths are emitted under safe mode;
- a review runbook for one repository.

### 5.2 Follow-up candidate

- opt-in multi-project inventory and digest;
- repository aliases and stable project keys;
- configurable tag vocabulary;
- promotion references on resolution events;
- packaging beyond `cargo install`;
- optional integrations with Beads, ClickUp, or CM through thin adapters.

### 5.3 Non-goals for now

- hosted service, accounts, telemetry, or synchronization server;
- replacing issue trackers or project-management systems;
- AI-generated fixes inside the logging command;
- automatic bulk resolution;
- storing raw terminal transcripts;
- ACFS-specific behavior inside the generic Rust core;
- a rename or incompatible rebrand before the fork contract is reviewed.

## 6. Architecture direction

Keep the upstream Rust core and event journal. Add narrowly scoped policy at the
edges:

```text
agent instruction
      |
      v
CLI policy preflight ---- structured refusal/warning
      |
      v
existing append-only store
      |
      +---- per-project review
      |
      +---- optional external inventory adapter (later)
```

Proposed separation:

- `src/store.rs`: storage, locking, discovery, append and fold invariants;
- command layer: new safe-mode flags and structured policy output;
- policy module: path capture and sensitive-data decisions, with no I/O beyond
  the supplied record;
- docs/templates: harness-neutral instructions and review workflow;
- optional adapters: separate scripts or crates, not hidden inside `add`.

## 7. Decisions still open

These must be resolved in planning before implementation:

1. Should safe/private mode become the global default or an explicit profile
   selected by the fork?
2. Should likely-secret detection warn, refuse, or support both policies?
3. Which checks can be deterministic and explainable without creating a false
   promise of complete secret detection?
4. Should paths be omitted, repository-relative, hashed, or selected by flag?
5. Where should private logs live by default across multiple projects?
6. Should the fork retain the binary/crate name while it remains GitHub-only?
7. Which changes belong in upstream pull requests versus fork-only policy?
8. What is the smallest useful cross-project review interface?

Until these decisions are recorded, do not start broad implementation.

## 8. Threat model

Protect against accidental disclosure, not a malicious local user with full
filesystem access.

Primary threats:

- an agent pastes a token, credential-bearing URL, private key fragment, email,
  customer identifier, or raw config into free-form text;
- absolute paths reveal usernames or private project layout;
- a tracked journal is committed and pushed without focused review;
- logging mutates a worktree during a read-only audit;
- a secret checker produces false confidence and users stop reviewing output;
- cross-project aggregation traverses unintended repositories or evidence.

Required posture:

- documentation must state that detection is a guardrail, not proof of safety;
- safe mode must minimize stored metadata before attempting content detection;
- every mutation must remain explicit in `schema`;
- no network call may be required to log or validate a cut;
- raw detected sensitive values must never be echoed in error details.

## 9. Compatibility and upstream strategy

- Preserve `upstream` and periodically fetch it without rewriting public fork
  history.
- Maintain a small compatibility matrix for upstream tags and fork releases.
- Separate generic bug fixes into focused commits that can be submitted
  upstream.
- Keep fork-only workflow adapters isolated and clearly labeled.
- Do not change contract version `1` silently; any output or record-shape change
  requires an explicit compatibility decision and schema tests.
- Preserve the MIT license and upstream attribution.

## 10. Verification gates

Every code slice must define:

- preflight state and compatibility assumptions;
- focused unit and black-box tests;
- execution command;
- verification command;
- rollback or feature-disable path;
- evidence location.

Baseline gate inherited from upstream:

```bash
cargo build --release
cargo test --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --check
```

When storage or concurrency changes, run the test suite five times. For changed
code, run a scoped UBS check before handoff.

Security acceptance must additionally prove:

- safe mode omits or normalizes absolute paths;
- likely-secret refusals do not echo the suspected secret;
- explicit legacy mode remains predictable during migration;
- read-only commands never create files or directories;
- public repository history passes a secret scan before release;
- documentation distinguishes implemented behavior from planned behavior.

## 11. Rollback strategy

- Keep upstream `v0.1.0` behavior reachable through a documented compatibility
  mode until a breaking release decision is made.
- Implement safe changes behind explicit, testable configuration boundaries.
- Revert individual focused commits instead of rewriting public history.
- Never auto-migrate or rewrite an existing journal without dry-run, backup, and
  a separately approved design.

## 12. Phased plan

### Phase 0 — Bootstrap and evidence

- create the public GitHub fork and separate local checkout;
- preserve upstream remote and attribution;
- establish AGENTS, plan, worklog, continuation docs, Beads, and Agent Mail;
- register one Machine Projects parent;
- run the unchanged upstream quality gate;
- record known risks without claiming fixes.

Exit: fork is public, recoverable, cleanly tracked, and ready for design review.

### Phase 1 — Contract and threat-model review

- inspect record shape, discovery, schema, and error contracts;
- decide safe-mode defaults and backward compatibility;
- define secret/path policy with examples and counterexamples;
- review the plan iteratively until architectural suggestions converge;
- convert accepted implementation slices to dependency-aware Beads.

Exit: decisions are explicit and implementation tasks have acceptance criteria.

### Phase 2 — Safe record creation

- implement path minimization;
- implement sensitive-data guardrails;
- add migration-safe flags/configuration;
- update schema and exhaustive tests;
- run adversarial review and the full gate.

Exit: a single-project pilot can log without publishing unnecessary context.

### Phase 3 — Pilot operating workflow

- install the forked binary under an unambiguous version;
- pilot in `papercuts`, `acfs-workbench`, and at most two other active projects;
- collect false positives, missed cases, and operational friction;
- define review cadence and promotion rules;
- keep rollout reversible.

Exit: evidence supports or rejects wider adoption.

### Phase 4 — Multi-project review

- design an explicit allowlisted inventory;
- produce bounded JSON and Markdown digests;
- keep source journals authoritative;
- add thin Beads/ClickUp/CM promotion adapters only where repeated use proves
  value.

Exit: cross-project analysis works without broad filesystem collection.

### Phase 5 — Public release

- settle name/version strategy;
- synchronize or document divergence from upstream;
- run tests, UBS, dependency review, and secret scan;
- publish release notes and migration instructions;
- avoid crates.io publication until package ownership and compatibility are
  explicitly decided.

## 13. Initial acceptance criteria

The bootstrap slice is complete when:

- `39elbarto/papercuts` is a real public GitHub fork;
- `/data/projects/papercuts` is the canonical local checkout;
- `origin` and `upstream` point to the correct repositories;
- the fork contains a self-contained plan, worklog, and continuation prompt;
- Beads and Agent Mail are initialized;
- unchanged upstream tests pass;
- Git history contains no unrelated private-project material;
- one ClickUp Machine Projects parent exists with accurate fields and links;
- the next session begins with contract and threat-model review, not coding.

## 14. Planning refinement protocol

This document is the initial plan, not the final architecture. Review it in
several fresh-context rounds. Each round should propose justified changes,
integrate accepted revisions in place, and record disagreements rather than
silently simplifying the plan. Use multi-model review only when it adds a
decorrelated perspective. Convert the plan into Beads after the contract and
threat model stabilize, then polish dependencies before implementation.
