# Worklog

## 2026-07-11 — Public fork bootstrap

### Outcome

- Created public GitHub fork `39elbarto/papercuts` from
  `treygoff24/papercuts` with upstream history and MIT attribution intact.
- Established canonical checkout at `/data/projects/papercuts`.
- Configured `origin` for the fork and `upstream` for the original repository.
- Initialized Codex trust, runtime handoff scaffolding, Beads, and Agent Mail.
- Added the initial fork plan and continuation protocol.
- Created the top-level ClickUp Machine Projects task `papercuts` with status
  `In Progress`, build mode, and `Soon` urgency.
- Kept product implementation unchanged; proposed hardening remains planning.

### Evidence

- Fork: `https://github.com/39elbarto/papercuts`
- Upstream: `https://github.com/treygoff24/papercuts`
- Agent Mail project: `/data/projects/papercuts`
- Initial reporter: `ChartreuseHawk`
- ClickUp: `https://app.clickup.com/t/86ey8k1ay`

### Papercut observed

`gh repo fork --clone` created the remote fork but treated an attempted explicit
destination argument as another repository, so local clone failed. Recovery was
to verify the remote fork and run an explicit `git clone` into the canonical
path. The event is also recorded in `.papercuts.jsonl` as a dogfood check.

### Next step

Run a focused contract and threat-model review of `docs/PROJECT_PLAN.md`. Decide
safe storage defaults, path handling, secret detection behavior, compatibility,
and upstream contribution boundaries before writing implementation Beads.

## 2026-07-11 — Beads conversion and polishing

### Outcome

- Converted the complete project plan into one active epic and 21 child Beads.
- Added 36 dependency edges covering decision gates, implementation, adversarial
  testing, documentation, pilot evidence, multi-project review, optional
  promotion adapters, and public release.
- Added structured acceptance criteria and time estimates to every Bead.
- Added explicit design/file-ownership notes to implementation Beads so later
  parallel agents can avoid overlapping Rust surfaces.
- Added the tag vocabulary and review lifecycle workstream that was present in
  the plan's follow-up scope but missing from the first conversion pass.
- Required the hardened-contract ADR to copy final decisions into downstream
  Beads before it can close, keeping execution self-contained after planning.

### Graph verification

- `br dep cycles --json`: zero cycles.
- `br ready --json`: exactly one actionable task,
  `br-hardened-papercuts-fork-x30.1`.
- `bv --robot-plan`: the upstream contract inventory is the highest-impact
  unblocker and opens four parallel decision tracks.
- `bv --robot-alerts`: no warning or critical alerts.
- Completeness audit: no missing descriptions, acceptance criteria, estimates,
  or duplicate titles.

### Next step

Execute `br-hardened-papercuts-fork-x30.1`: inventory the upstream `v0.1.0`
contract and all four findings in upstream issue #1 without changing product
behavior. That evidence unlocks the storage, path, sensitive-data, and upstream
strategy decisions.

## 2026-07-11 — Upstream v0.1 compatibility audit

### Outcome

- Completed `br-hardened-papercuts-fork-x30.1` without changing product code.
- Added `docs/UPSTREAM_V0_1_COMPATIBILITY_AUDIT.md` with the complete CLI,
  storage, record, error, side-effect, locking, fold, test, and versioning
  inventory.
- Confirmed `upstream/main` at `ffba2bd` and release `v0.1.0` at `5d8b827`.
- Confirmed and classified all four findings in upstream issue #1: sensitive
  committed data, missing MSRV declaration, dangling absolute skill symlink,
  and `.git.exists()` repository detection.
- Verified that the fork has no product-code diff from upstream under `src/`,
  `tests/`, `Cargo.toml`, or `Cargo.lock`.

### Verification

- Release build: pass.
- Tests: 30 passed.
- Clippy with warnings denied: pass.
- Formatting check: pass.
- `papercuts doctor`: healthy, two journal lines.
- Disposable live probes: schema, dry-run/no-create, virtual-empty list and
  doctor all matched the documented contract.

### Next step

The audit unlocks four decision Beads that can proceed independently:

- `br-hardened-papercuts-fork-x30.2` — storage profiles;
- `br-hardened-papercuts-fork-x30.3` — path minimization;
- `br-hardened-papercuts-fork-x30.4` — sensitive-data guardrail;
- `br-hardened-papercuts-fork-x30.6` — upstream and release strategy.

## 2026-07-11 — Upstream, naming, and release strategy

### Outcome

- Completed `br-hardened-papercuts-fork-x30.6` as a documentation and operating-
  policy slice; product code remains unchanged.
- Added `docs/UPSTREAM_SYNC_AND_RELEASE_RUNBOOK.md` with exact preflight, fetch,
  merge, upstream PR, conflict, verification, rollback, and evidence procedures.
- Defined immutable public-history rules: upstream is fetch-only, sync happens
  on dated branches through fork pull requests, and published mistakes are
  reverted rather than erased.
- Classified current upstream candidates versus fork-only behavior and defined
  how accepted, rejected, and inactive upstream proposals are carried.
- Resolved the package and binary decision: retain `papercuts` only through
  source development and an exact-SHA isolated pilot; rename before non-isolated
  distribution if fork-only behavior remains.
- Prohibited publication to crates.io as `papercuts`. The official sparse index
  shows upstream package 0.1.0, while this fork has no verified namespace
  authority.
- Preserved the GitHub fork name, MIT license, upstream attribution, and an
  explicit “not an upstream release” release-note contract.

### Live evidence

- `origin/main`: `6e7dd774778866821e6969779772d02e18d572c1` at pre-change snapshot.
- `upstream/main`: `ffba2bd453ab0faeadf4f923fc727586958c8d6f`.
- Upstream release `v0.1.0`:
  `5d8b827abbd054f5f506d26be865f5b7f573a298`.
- Relationship: fork was four commits ahead and zero behind; merge base was
  exactly `upstream/main`.
- GitHub confirmed the fork parent, MIT license, no fork release, and upstream
  issue #1 still open without comments.
- The official crates.io sparse index returned `papercuts` 0.1.0. The owners API
  returned HTTP 503, so owner identity was deliberately treated as unverified.
- The local `upstream` push URL is now the `DISABLED` sentinel; a dry-run push
  failed locally before contacting GitHub, while fetch remains configured to
  the original repository.

### Verification

- Release build: pass.
- Tests: 30 passed.
- Clippy with warnings denied: pass.
- Formatting check and `git diff --check`: pass.
- `papercuts doctor`: healthy, five journal lines after the three dogfood events
  recorded during this slice.
- Gitleaks: no leaks found across 12 commits.
- UBS: skipped because this slice changed only planning/docs, Beads, and the
  append-only dogfood journal; no code, script, hook, or executable
  configuration changed.

### Papercuts observed

- crates.io's API returned repeated HTTP 503 responses during namespace-owner
  verification; the sparse index supplied package existence and version, but
  publication remains prohibited without an authoritative owner readback.
- A probe assumed `target/release/papercuts`, while this host uses a shared Cargo
  target directory. The journal records the safer instruction to use
  `cargo run` or build and resolve the actual target directory first.
- The destructive-command guard interpreted an angle-bracket path placeholder
  inside a Beads note as shell redirection and blocked the complete metadata
  command before execution. The safe retry used plain-language placeholders.

### Next step

Proceed with one of the three remaining independent contract decisions:
storage profiles, path minimization, or sensitive-data guardrails. The hardened
contract ADR will combine those choices with this release strategy before any
broad implementation begins.

## 2026-07-11 — Safe storage and read-only semantics decision

### Outcome

- Completed the planning contract for
  `br-hardened-papercuts-fork-x30.2`; product code remains unchanged.
- Added `docs/SAFE_STORAGE_PROFILES_ADR.md` with exact profile, precedence,
  first-run, no-write, migration, rollback, permissions, schema, error, and test
  behavior.
- Selected `private` as the hardened default and `committed` as the explicit
  upstream-compatible profile.
- Selected `GIT_COMMON_DIR/papercuts/log.jsonl` as private per-project state so
  normal and linked worktrees share history without dirtying a worktree.
- Required explicit storage outside validated Git instead of mixing unrelated
  directories in an implicit global journal.
- Added a separate monotonic read-only guard; it can deny append commands but
  cannot infer conversational scope or grant write authority.
- Required migration refusal when only a legacy journal exists, followed by an
  explicit copy-and-verify transition with selection-only rollback.

### Evidence

- Current v0.1 discovery and side-effect behavior was rechecked in `src/store.rs`,
  all command dispatchers, schema/errors, and the relevant black-box tests.
- A disposable Git probe confirmed that a main checkout and linked worktree
  resolve one common Git directory, while a submodule resolves a distinct
  common directory.
- Product files under `src/`, `tests/`, `Cargo.toml`, and `Cargo.lock` still
  match `upstream/main` before this decision is committed.

### Verification

- Release build: pass.
- Tests: 30 passed.
- Clippy with warnings denied: pass.
- Formatting and `git diff --check`: pass.
- `papercuts doctor`: healthy, eight journal lines.
- Gitleaks: no leaks found across 13 commits.
- UBS: skipped because this slice changed only planning/docs, Beads, and the
  append-only dogfood journal; no code, script, hook, or executable
  configuration changed.
- Beads graph: no cycles; `bv --robot-next` selected
  `br-hardened-papercuts-fork-x30.3`.

### Papercuts observed

- An expected failing `git rev-parse` inside a `set -e` command substitution
  produced empty status evidence; the corrected probe used an explicit branch.
- zsh reserves `status` as a read-only variable; the corrected portable snippet
  used `rc` for the exit code.
- One documentation patch missed because its expected context split a wrapped
  paragraph differently; the retry inspected exact numbered lines and applied
  a narrower patch.

### Next step

Complete the independent path-minimization and sensitive-data decisions. The
consolidated ADR must then reconcile all three contracts before implementation
of storage resolution begins.

## 2026-07-12 — Path minimization and project identity decision

### Outcome

- Completed the planning contract for
  `br-hardened-papercuts-fork-x30.3`; Rust product code remains unchanged.
- Added `docs/PATH_AND_PROJECT_IDENTITY_ADR.md` with exact safe/legacy records,
  read projection, diagnostics, strict Git resolution, symlink/non-UTF-8 rules,
  project aliases, migration, rollback, and test requirements.
- Selected automatic path omission for private records while retaining
  contract-1 parser compatibility through sentinel fields.
- Kept legacy absolute capture only in the explicit committed profile.
- Selected external operator aliases for later multi-project identity; rejected
  automatic path hashes, remote-derived IDs, random UUIDs, and basenames.
- Required safe projection of stored legacy records in all private outputs
  without rewriting append-only source bytes.
- Refined the storage ADR so explicit file selection changes the target but not
  the active private/committed profile or path policy.

### Evidence

- Rechecked all path-bearing Rust surfaces in record construction, storage,
  list/resolve/doctor output, schema, errors, and black-box tests.
- A disposable symlink probe confirmed logical shell cwd can differ from the
  physical Git root/common directory; private output will expose neither.
- A disposable non-UTF-8 probe confirmed current v0.1 JSON uses Unicode
  replacement through lossy path conversion.
- The exact example ID was generated with the existing path-independent ID
  algorithm and is identical for safe and legacy examples.
- The unchanged v0.1 binary parsed the proposed safe record, returned the cwd
  sentinel and null repo, and passed doctor ID validation; compatibility is
  proven as parsing, not contract-2 semantic awareness.

### Verification

- Release build: pass.
- Tests: 30 passed.
- Clippy with warnings denied: pass.
- Formatting and `git diff --check`: pass.
- `papercuts doctor`: healthy, nine journal lines.
- Gitleaks: no leaks found across 14 commits.
- Proposed contract-2 safe record: unchanged v0.1 list and doctor passed.
- UBS: skipped because this slice changed only planning/docs, Beads, and the
  append-only dogfood journal; no code, script, hook, or executable
  configuration changed.
- Beads graph: no cycles; `bv --robot-next` selected
  `br-hardened-papercuts-fork-x30.4`.

### Papercut observed

A broad angle-bracket placeholder check matched the legitimate Rust type
`Option<String>` and stopped validation early. The corrected check avoids
treating language generics as shell redirection placeholders.

### Next step

Complete the sensitive-data guardrail decision. The consolidated ADR can then
integrate storage, paths, input policy, compatibility, and release strategy
before implementation begins.
