# Papercuts Fork Project Plan

Status: phase-1 hardened contract accepted; phase-2 implementation authorized

Date: 2026-07-12

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
- automatic `cwd` and repository-path omission in the private profile, with an
  explicit committed compatibility projection;
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

### 6.1 Accepted storage direction

The storage and read-only decision is recorded in
[`SAFE_STORAGE_PROFILES_ADR.md`](SAFE_STORAGE_PROFILES_ADR.md). In summary:

- `private` is the hardened default;
- inside a validated Git working tree, private state lives at
  `GIT_COMMON_DIR/papercuts/log.jsonl`, outside every worktree;
- outside validated Git, private mutation requires an explicit file instead of
  silently mixing directories in a global journal;
- `committed` explicitly preserves upstream repository-visible discovery;
- a monotonic read-only guard denies actual appends but does not pretend to
  infer conversational authorization;
- existing committed journals are never auto-migrated or merged.

Implemented by `x30.7`. The fork still withholds a hardened-release claim until
the remaining compatibility, adversarial, documentation, and release gates
pass.

### 6.2 Accepted path and project-identity direction

The path and identity decision is recorded in
[`PATH_AND_PROJECT_IDENTITY_ADR.md`](PATH_AND_PROJECT_IDENTITY_ADR.md). In
summary:

- private records automatically store no filesystem or derived path identity;
- contract-1 fields remain parseable as `cwd: "."` and `repo: null`, labeled
  with `path_policy: omitted`;
- committed compatibility explicitly retains legacy absolute paths;
- private output redacts path-bearing contract-1 records without rewriting
  source bytes;
- strict Git metadata validation replaces `.git.exists()` discovery;
- project names are external operator aliases in an allowlist, not record
  fields, path hashes, remote-derived IDs, or automatic basenames.

Implemented by `x30.8`, including mixed-journal private projection and strict
native Git metadata resolution. Remaining release gates still apply.

### 6.3 Accepted sensitive-data guardrail direction

The content policy decision is recorded in
[`SENSITIVE_DATA_GUARDRAIL_ADR.md`](SENSITIVE_DATA_GUARDRAIL_ADR.md). In
summary:

- one bounded, versioned, offline scanner covers cut text, tags, persisted agent
  names, and resolution notes before append-side I/O;
- `balanced` is the private-profile floor: high-confidence credential shapes
  refuse while medium-risk identifiers, paths, and config-like context append
  with category-only audit metadata;
- `strict` is the committed-profile floor and refuses both levels;
- policy can be strengthened but not weakened below the profile floor;
- an override requires an operator-controlled environment gate plus exact
  repeated category flags, has no wildcard, and is persisted as category-only
  audit metadata;
- refused values are never echoed, hashed, logged, or written by Papercuts;
- policy version 1 deliberately has no entropy detector, recursive decoding,
  Unicode normalization, network lookup, or exhaustive-safety claim;
- new cut and resolve events carry `content_policy`; old events remain
  `legacy-unscanned` without rewrite.

Implemented by `x30.9`, including bounded stdin and persisted-field validation,
policy-v1 scanning, exact overrides, category-only refusal diagnostics,
contract-2 audit objects, synthetic corpus coverage, and release benchmark
evidence. Consolidated compatibility (`x30.10`) and the independent adversarial
acceptance suite (`x30.11`) remain pending.

### 6.4 Accepted consolidated contract

[`HARDENED_CONTRACT_ADR.md`](HARDENED_CONTRACT_ADR.md) is the normative
phase-2 contract. It selects machine contract 2, reconciles evaluation and
side-effect order across the three policy ADRs, defines exact records,
metadata, error codes, compatibility and rollback, and separates Rust
mechanism from repository/harness authorization policy.

Implementation remains dependency-ordered through Beads. Closing the
architecture gate authorizes `br-hardened-papercuts-fork-x30.7` first; it does
not mean the current binary implements contract 2.

## 7. Planning decisions

All eight original planning decisions are resolved or deliberately gated:

| Decision | Resolution |
|---|---|
| default profile | private; committed is explicit |
| warn versus refuse | private balanced; committed strict |
| deterministic content checks | bounded offline policy version 1; no exhaustive claim |
| automatic paths | omitted in private; labeled legacy absolute in committed |
| private target | validated Git common directory; explicit file outside Git |
| package/binary name | retained only through exact-SHA isolated pilot; later rename gate |
| upstream boundary | generic contract-1 fixes isolated; fork policy stays fork-only |
| cross-project review | deferred to `x30.17` after pilot, with explicit allowlist/no-scan boundary |

The deferred cross-project surface is not part of the first single-project
contract and does not block phase-2 implementation.

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
- balanced warnings and deliberate overrides persist the original accepted
  input; their category-only metadata is not redaction.

## 9. Compatibility and upstream strategy

- The detailed maintainer contract is
  [`UPSTREAM_SYNC_AND_RELEASE_RUNBOOK.md`](UPSTREAM_SYNC_AND_RELEASE_RUNBOOK.md).
- Preserve `upstream` and periodically fetch it without rewriting public fork
  history.
- Maintain a small compatibility matrix for upstream tags and fork releases.
- Separate generic bug fixes into focused commits that can be submitted
  upstream.
- Keep fork-only workflow adapters isolated and clearly labeled.
- Keep the GitHub repository name `papercuts`. The Cargo package and binary may
  keep that name through development and an exact-SHA isolated pilot only.
- Before non-isolated public distribution, rename the package and binary if any
  fork-only default or machine-contract behavior remains. Retain the upstream
  name beyond the pilot only with explicit namespace authority and upstream-
  compatible behavior.
- Do not publish to crates.io as `papercuts`; upstream 0.1.0 occupies that
  namespace and this fork has no verified publishing authority. The first fork
  release is GitHub-only unless a later release gate explicitly authorizes a
  distinct package namespace.
- Integrate upstream on dated sync branches and merge through reviewed fork
  pull requests. Never rebase, reset, or force-push public `main`; revert a bad
  public integration with a normal rollback pull request.
- The first hardened machine contract is exactly `2`; schema and compatibility
  tests must pin it, while contract-1 journal lines remain readable.
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

- Keep repository-visible v0.1 storage/projection reachable through the
  committed profile, but do not describe it as unchecked v0.1 behavior. Exact
  unchecked rollback requires the explicitly selected upstream v0.1 binary.
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

Status: complete through `docs/HARDENED_CONTRACT_ADR.md` and propagated Beads.

### Phase 2 — Safe record creation

Status: in progress; `x30.7` profile/storage policy, `x30.8` path projection,
and `x30.9` sensitive preflight are implemented. `x30.10` consolidated
schema/error/compatibility work is the next dependency-ordered slice.

- implement shared profile/target/write/content policy resolution (`x30.7`);
- implement strict Git resolution and path minimization (`x30.8`);
- implement sensitive-data guardrails (`x30.9`);
- update contract-2 schema/errors (`x30.10`) and exhaustive tests (`x30.11`);
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
