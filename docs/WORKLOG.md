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
