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

## 2026-07-12 — Deterministic sensitive-data guardrail decision

### Outcome

- Completed the planning contract for
  `br-hardened-papercuts-fork-x30.4`; Rust product code remains unchanged.
- Added `docs/SENSITIVE_DATA_GUARDRAIL_ADR.md` with exact modes, category
  catalog, size bounds, override controls, dry-run behavior, redacted error
  shape, record audit, performance budget, limitations, and synthetic corpus.
- Selected `balanced` as the private-profile floor and `strict` as the
  committed-profile floor; callers may strengthen policy but not weaken it
  below the active profile.
- Selected a two-key, category-specific override with no wildcard: a controlling
  environment gate and exact repeated flags must both be present.
- Required scanning of every persisted caller-controlled string before journal
  creation, open, lock, read, duplicate lookup, or event construction.
- Rejected generic entropy scoring, recursive decoding, mandatory external
  scanners, network catalogs, value echo, and one-flag force bypasses.
- Required new cut and resolve events to carry category-only `content_policy`
  audit while retaining old events as append-only `legacy-unscanned` history.

### Evidence

- Rechecked the current add, resolve, CLI, record, error, schema, and black-box
  test surfaces. Current v0.1 scans nothing, reads stdin without a pre-read
  bound, limits only cut text, and persists agent, tag, and resolution-note
  strings without equivalent bounds.
- Sourced initial vendor-prefix categories from official GitHub, AWS, Slack, and
  Stripe documentation, without relying on undocumented fixed token lengths.
- Chose the Rust regex crate only as an implementation option with fixed
  patterns, compiled-size and input caps, and documented worst-case bounds; the
  ADR does not require runtime downloads or caller-supplied regexes.
- Performed security-honesty, compatibility/side-effect-order, and downstream-
  executability review passes; material findings were incorporated into the
  final ADR.

### Verification

- Release build: pass.
- Tests: 30 passed.
- Clippy with warnings denied: pass.
- Formatting and `git diff --check`: pass.
- `papercuts doctor`: healthy, twelve journal lines.
- Gitleaks: no leaks found across 15 commits and the working tree.
- Product files under `src/`, `tests/`, `Cargo.toml`, and `Cargo.lock` still
  match `upstream/main`.
- Beads graph: 22 nodes, 27 edges, no cycles; sensitive-policy decisions copied
  into every affected implementation, schema, test, docs, pilot, digest, and
  release bead.
- UBS: skipped because this slice changed only planning/docs, Beads, and the
  append-only dogfood journal; no code, script, hook, or executable
  configuration changed.

### Papercuts observed

- Source inspection assumed a nonexistent `src/model.rs`; record types actually
  live in `src/lib.rs`.
- A direct `jq` query assumed a top-level issue array, while this `br list`
  version returns an object containing `issues`.
- Overlapping `sed` ranges made headings appear duplicated and caused one
  failed cleanup patch; exact numbered context corrected the review.

### Next step

Publish the consolidated hardened-contract ADR in
`br-hardened-papercuts-fork-x30.5`, reconciling storage, path, content policy,
schema version, compatibility, and release naming before implementation begins.

## 2026-07-12 — Hardened contract v2 architecture gate

### Outcome

- Completed `br-hardened-papercuts-fork-x30.5` as the phase-1 architecture
  gate; Rust product code remains unchanged.
- Added `docs/HARDENED_CONTRACT_ADR.md` as the normative, self-contained
  contract for phase-2 implementation.
- Selected machine contract exactly `2` and reconciled private/committed
  profiles, target and policy precedence, read-only behavior, command-relevant
  environment, lazy clock resolution, migration, path projection, sensitive
  preflight, schema/errors, compatibility, rollback, and distribution.
- Resolved or explicitly gated all eight original planning questions. The
  multi-project interface is deferred behind pilot evidence and no longer
  blocks single-project implementation.
- Updated `docs/PROJECT_PLAN.md` from planning baseline to implementation-ready
  state and refreshed the continuation prompt so resumed sessions begin with
  the first unblocked implementation Bead.
- Rewrote the phase-2 Bead outcomes, scopes, designs, tests, and acceptance
  criteria with exact contract-2 terminology, then copied final decisions into
  every affected implementation, acceptance, documentation, pilot, inventory,
  release, tag, and adapter Bead.

### Review findings integrated

Six focused passes covered usability, security honesty, determinism,
compatibility, migration/rollback, and Bead self-containment/dependencies.
Material corrections included:

- changed the override example to strict policy because an email is only a
  warning under balanced mode and would make the override unused;
- made schema independent of ambient environment and limited every other
  command to environment values that can affect it;
- kept `PAPERCUTS_FILE` and `HOME` in native path encoding while retaining
  UTF-8 requirements for textual policy, agent, and clock values;
- required parser, config, ID, and since errors to omit rejected argv/env
  values instead of forwarding raw Clap messages;
- fixed deterministic sorting for warnings and set-like arrays;
- removed premature alias/digest acceptance requirements from the
  single-project `x30.11` gate;
- clarified that committed preserves repository-visible storage/projection but
  is not an unchecked v0.1 or scanner-bypass mode;
- limited implicit legacy migration detection to private profile-default
  targets, leaving explicit file selection deliberate and independent.

The final self-containment term audit passed for `x30.7` through `x30.21`, and
the dependency graph remained acyclic.

### Verification

- Release build: pass.
- Tests: 30 passed.
- Clippy with warnings denied: pass.
- Formatting and `git diff --check`: pass.
- `papercuts doctor`: healthy, fifteen journal lines.
- Gitleaks: no leaks found across 16 commits and the working tree.
- Product files under `src/`, `tests/`, `Cargo.toml`, and `Cargo.lock` still
  match `upstream/main`.
- Exact fixed-clock record ID: `pc_94f5df71022d`, matching both contract-2 path
  examples.
- Beads graph: 22 nodes, 27 edges, zero cycles; `x30.7` is the next direct
  implementation unblocker after this gate closes.
- UBS: skipped because this slice changed only planning/docs, Beads, and the
  append-only dogfood journal; no code, script, hook, or executable
  configuration changed.

### Papercuts observed

- `skill-finder` could not search or auto-load because its required meta-skill
  MCP tools were not exposed; installed planning and Beads skills were used
  directly.
- A long-document patch coupled unrelated section anchors, so one absent anchor
  rejected all intended corrections; section-local patches were reliable.
- An exact ID check initially omitted `PAPERCUTS_NOW`, producing a legitimate
  different timestamp-derived ID; the fixed-clock retry matched the contract.

### Next step

Begin `br-hardened-papercuts-fork-x30.7`: implement the shared contract-2 typed
policy and storage-resolution seam without widening into path projection or the
scanner catalog owned by later Beads.

## 2026-07-12 — Safe profile and storage resolution implementation

### Outcome

- Completed `br-hardened-papercuts-fork-x30.7` as the first phase-2 Rust
  implementation slice.
- Added one typed policy context that centrally resolves profile, target,
  monotonic write policy, sensitive-policy floor and override inputs, agent
  source, path policy, and lazy clock access.
- Made `private` the default profile and selected one shared journal at
  `GIT_COMMON_DIR/papercuts/log.jsonl` for ordinary and linked worktrees.
- Kept repository-visible `.papercuts.jsonl` and the non-Git HOME target behind
  the explicit `committed` compatibility profile.
- Added exact flag/environment/profile target precedence, command-relevant
  environment reads, native path-valued environment handling, and static
  schema behavior that ignores ambient environment.
- Added a monotonic read-only guard that refuses real add/resolve operations
  before semantic input, clock, repository, storage, stdin, or journal I/O;
  dry-run remains non-mutating and available where target semantics permit it.
- Added private non-Git `storage_required`, legacy-only
  `migration_required`, dual-journal warnings, no fallback, user-only Unix
  creation modes, insecure-permission mutation refusal, and doctor reporting.
- Added strict common-Git-directory resolution for ordinary repositories and
  linked worktrees, nearest-invalid-marker refusal, and private final/implicit
  directory symlink rejection while preserving committed symlink behavior.
- Kept append, fold, lock, tear-heal, and duplicate semantics unchanged.
- Published an intentionally transitional schema that describes the storage
  seam exactly and explicitly marks legacy path projection as pending `x30.8`
  and sensitive-content scanning/enforcement as pending `x30.9`; the fork still
  makes no hardened-release claim.

### Review findings integrated

The first fresh-eyes cross-review found three blockers, all corrected and
re-reviewed:

- private explicit and implicit journals could follow final symlinks;
- whitespace `--agent` validation ran before `writes_disabled`;
- `schema record` and the all-schema metadata description did not exactly match
  the transitional record and success metadata surfaces.

The focused re-review passed all three fixes and found no regression in
profile/target precedence, relevant environment reads, common-dir resolution,
migration state, permissions, or append/lock invariants.

### Verification

- Release build: pass.
- Unit tests: 6 passed.
- Black-box CLI tests: 35 passed, including 11 new contract-2 scenarios.
- Full test suite repeated five times after the final `store.rs` change: all
  five runs passed with 35 CLI tests each.
- Clippy with warnings denied: pass.
- Formatting and `git diff --check`: pass.
- Gitleaks: no leaks found across 17 commits and the working tree.
- Current committed journal doctor: healthy, sixteen lines.
- UBS `--diff`: completed but returned its known noisy Rust heuristic result:
  four critical labels were test-only `panic!` assertions; remaining warnings
  were test `unwrap`/assert inventory and pre-existing safe indexing/cast or
  clone heuristics. Clippy, build, tests, manual review, and cross-review found
  no corresponding production defect. The exact report is retained at
  `/tmp/papercuts-x30.7-ubs-final.log` for this host session only.

### Papercut observed

`cargo test` accepts only one positional `TESTNAME` filter. A command that
passed two test names failed before running either test. The corrected approach
used separate invocations, and the event was recorded in the dogfood journal as
`pc_9fc4bf4fbb25`.

### Rollback

Revert the focused implementation commit through normal Git history. Existing
private and committed journals remain append-only and are not deleted, merged,
or rewritten. To inspect the retained committed source explicitly, select the
`committed` profile; do not silently fall back or move files.

### Next step

Closing `x30.7` unlocked both `x30.8` (privacy-preserving path/project
metadata) and `x30.9` (local sensitive-data preflight). The graph remains a DAG
with zero cycles, and `bv --robot-next` selects `x30.8` as the next highest-
impact ready slice. Keep the two responsibilities separate despite their later
shared schema and acceptance dependencies.

## 2026-07-12 — Privacy-preserving path and project metadata implementation

### Outcome

- Completed `br-hardened-papercuts-fork-x30.8` as the second phase-2 Rust
  implementation slice.
- Added typed `path_policy` and `path_encoding` record fields without changing
  cut-ID inputs. New private records write the exact omitted sentinels; the
  committed compatibility profile labels retained absolute paths as UTF-8 or
  lossy UTF-8.
- Projected contract-1 and legacy-absolute cuts on every private add, duplicate,
  list, Markdown, resolve, already-resolved, and doctor response without
  rewriting source journal bytes.
- Implemented one native strict-Git resolver for ordinary repositories, linked
  worktrees, and submodule-shaped gitdir files. It validates the nearest marker,
  `HEAD`, common `objects` and `config`, native non-UTF-8 metadata on Unix,
  symlinks, and exact LF/CRLF line grammar without invoking Git.
- Preserved operating-system symlink and parent-traversal semantics until
  canonicalization, so metadata such as `route/../admin` cannot be redirected
  by premature lexical normalization.
- Added private-safe parser, repository, journal, lock, OS, and doctor
  diagnostics with opaque location codes and no `meta.file`; committed output
  keeps its explicitly warned compatibility behavior.
- Updated the transitional contract-2 schema and status docs. Sensitive-content
  scanning remains explicitly pending `x30.9`, and the fork still makes no
  hardened-release claim.

### Review findings integrated

The fresh-eyes cross-review found two high-severity issues and one diagnostic
issue, all corrected and re-reviewed:

- Git metadata paths were lexically collapsing `..` before `canonicalize`,
  which changes semantics when an earlier component is a symlink;
- private doctor findings could repeat an unknown enum, kind, or ID from a
  malformed source record;
- invalid nearest-Git metadata used a journal location code instead of the
  opaque `repository_marker` code.

The bounded re-review passed the fixes and their unique-sentinel regression
tests with no new finding. Less central platform combinations such as native
non-UTF-8 `commondir` and explicit journal names remain candidates for the
exhaustive adversarial suite owned by `x30.11`.

### Verification

- Unit tests: 6 passed.
- Black-box CLI tests: 47 passed, including exact private/committed records,
  identical IDs, mixed-journal projection, private diagnostic sentinels,
  worktree/submodule/bare/malformed repositories, native non-UTF-8 metadata,
  symlink traversal, parser errors, and append-only readback.
- Full test suite repeated five times after the final `store.rs` change: all
  five runs passed with 47 CLI tests each.
- The upstream `v0.1.0` binary successfully listed and diagnosed a new omitted
  record. The journal SHA-256 was identical before and after both old-client
  reads.
- Clippy with warnings denied and formatting: pass.
- Gitleaks: no leaks found across 18 commits and the working tree.
- UBS `--diff`: completed with the known noisy Rust heuristics. Its critical
  labels are test-only `panic!` and byte-comparison assertions; the remaining
  inventory is predominantly test `unwrap`/assert usage and pre-existing
  indexing, cast, clone, and allocation heuristics. No corresponding production
  defect remained after Clippy, tests, manual review, or cross-review. The full
  host-local report is `/tmp/papercuts-x30.8-ubs-final.log`.

### Papercuts observed

- `pc_956892a253f7`: a compatibility script was blocked because cleanup used
  `rm -rf`; the check was split from cleanup.
- `pc_12eb71985fb9`: this host redirects Cargo artifacts to a shared target
  directory, so a repo-local `target/debug/papercuts` assumption failed; the
  check switched to `cargo run`.
- `pc_25d1e62997b9`: `zsh` reserves the read-only variable `status`; verification
  wrappers now capture exit codes in `rc` or `exit_code`.

### Rollback

Revert the focused implementation commit through normal Git history. Existing
private and committed journals remain append-only; do not delete, merge, move,
or reconstruct them during rollback. Contract-1 readers continue to ignore the
new optional record fields.

### Next step

Proceed to `br-hardened-papercuts-fork-x30.9`, the isolated sensitive-content
preflight slice. Keep scanner decisions ahead of event/path assembly, preserve
the stable ID contract, and leave consolidated schema/error finalization to
`x30.10` and adversarial expansion to `x30.11`.

## 2026-07-12 — Bounded sensitive-data preflight implementation

### Outcome

- Completed the implementation scope of
  `br-hardened-papercuts-fork-x30.9` as the third phase-2 Rust slice.
- Added a pure, offline policy-v1 scanner with eleven bounded static patterns
  covering all eight high-confidence and four medium-risk categories. Regex
  compilation is process-local, size-limited, and independent of runtime
  configuration, filesystem state, clocks, subprocesses, and network access.
- Enforced the private `balanced` and committed `strict` floors already
  resolved by the central policy seam. Category overrides now require the
  existing environment gate, must exactly cover every refusing category, and
  reject wildcard, partial, flag-only, and unused preauthorization paths.
- Bounded text, stdin, notes, tags, tag count, agent names, and total scan
  payload before event construction or journal open. Exact 10,000-byte LF and
  CRLF stdin payloads are accepted after trailing-newline removal; oversize and
  invalid UTF-8 inputs remain structured exit-65 failures.
- Added `content_policy` to new cut and resolve events and to materialized
  resolution output. Contract-1 events remain readable as legacy-unscanned and
  are never rewritten; audit metadata remains outside the cut ID.
- Added `sensitive_input` exit 65 with category/field-only details. Committed
  sensitive refusals also suppress `meta.file`, preventing the selected target
  from leaking through the shared error wrapper.
- Preserved dry-run and duplicate semantics while moving the scanner before
  clock access, event/ID construction, duplicate lookup, directory creation,
  journal open, lock, read, or append.
- Updated the transitional contract-2 schema, README status, and project plan.
  Full compatibility wording and independent adversarial acceptance remain
  owned by `x30.10` and `x30.11`; the fork still makes no hardened-release
  claim.

### Corpus and regression coverage

- Unit corpus covers every private-key family, mixed-case Bearer/Basic headers,
  HTTP/SSH/database credential URLs, YAML/dotenv/export/JSON assignments, every
  GitHub/Slack/Stripe prefix, paired AKIA/ASIA material across fields, all
  personal-identifier labels, Unix/macOS/Windows/UNC paths, and LF/CRLF config
  blocks.
- Negative controls cover every exact placeholder, shell variable references,
  hashes, UUIDs, Bead IDs, publishable Stripe keys, unpaired AWS access IDs,
  short prefixes, relative paths, single assignments, Unicode prose, encoded
  material, and deliberately split vendor tokens.
- Black-box tests cover warning persistence, strict refusal, two-key override,
  no-echo/no-write sentinels, committed target suppression, invalid-clock and
  duplicate ordering, dry-run parity, text/tag/agent/resolution-note fields,
  exact and plus-one bounds, and unchanged journal bytes after refusal.

### Performance evidence

- Added `examples/policy_bench.rs`, which runs at least 10,000 release-mode
  scans of the maximum legal add payload (11,152 bytes) while exercising all
  twelve categories under an exact strict override.
- Recorded evidence in
  `docs/evidence/x30.9-sensitive-preflight-benchmark-2026-07-12.md` with host,
  dirty source state, release binary SHA-256, exact command, and raw JSON.
- Observed p50 0.023756 ms, p95 0.043114 ms, and maximum 0.125133 ms. The
  acceptance budgets are p95 <= 5 ms and maximum <= 20 ms.

### Verification

- `cargo build --release`: pass.
- `cargo test --all-features`: 15 unit tests and 50 black-box CLI tests pass.
- Because the resolution fold in `store.rs` gained the resolve-event audit,
  the full suite was repeated five times; all five runs passed with the same
  15/50 counts.
- `cargo clippy --all-targets --all-features -- -D warnings`: pass.
- `cargo fmt --check` and `git diff --check`: pass.
- Gitleaks initially identified twelve synthetic private-key, AWS, and Stripe
  fixtures. Their source literals were split without changing runtime values;
  the final full working-tree scan reports no leaks.
- Scoped UBS completed across all changed Rust source, test, and benchmark
  files. Its non-zero inventory is noisy Rust heuristics: test-only
  `unwrap`/`assert`/`panic`, comparisons of fixture labels containing `token`,
  bounds-guarded indexing, and the pre-existing native Git path join required
  by the accepted path ADR. UBS also independently reports clean formatting,
  Clippy, cargo check, test build, unsafe usage, runtime regex inputs, and
  resource lifecycle. No production defect remained after review.

### Papercuts observed

- `pc_05f369a3f132`: Cargo accepts only one positional test-name filter per
  `cargo test` invocation.
- `pc_68a933e2fea9`: repository-local dogfood logging is blocked by the pending
  legacy-journal migration and needs an approved explicit private target.
- `pc_68f9f458a3de`: this host uses a shared `CARGO_TARGET_DIR`, so
  repo-relative release artifact paths are not reliable.

### Rollback

Revert the focused implementation through normal Git history. Existing journal
bytes remain append-only: do not delete, rewrite, strip `content_policy`, or
retroactively label legacy records. Selecting the upstream binary removes the
guardrail and must be treated as an explicit loss of protection, not as a
compatible hardened mode.

### Next step

Proceed to `x30.10` for consolidated schema, errors, and compatibility
surfaces. Keep the already-implemented policy-v1 behavior stable and leave the
independent adversarial expansion to `x30.11`.

## 2026-07-12 — Contract-2 schema, diagnostics, and compatibility

### Outcome

- Completed the implementation scope of
  `br-hardened-papercuts-fork-x30.10` after the storage, path, and content
  shapes stabilized in `x30.7` through `x30.9`.
- Replaced the transitional flat schema with a static structured contract that
  publishes every command and annotation, global flag, relevant environment
  variable, precedence rule, evaluation order, profile floor, storage and
  migration state, strict repository grammar, path projection, metadata shape,
  warning meaning, ID input/exclusion, content category/bound/limitation,
  diagnostic rule, compatibility boundary, and rollback rule.
- Added canonical private cut, committed cut, and resolve examples using the
  ADR-owned `pc_94f5df71022d` ID and exact serialized field order. Tests parse
  the examples into runtime record types and recompute the ID.
- Kept error codes and category names runtime-sourced through
  `ERROR_CONTRACT` and `SensitiveCategory::ALL`. Added a separate exact exit
  dictionary so shared exits such as 65, 77, and 78 no longer inherit whichever
  error description happened to be inserted last.
- Added persisted `content_policy` invariant validation to doctor. Version,
  decision, mode, category, field, sort, and dedup drift now produces the safe
  `content_policy_mismatch` finding.
- Added `legacy_unscanned_records:N` to add/duplicate, resolve, list, and doctor
  projections without changing or synthesizing source audit fields.
- Added black-box proof that schema ignores invalid ambient profile, target,
  clock, agent, read-only, and content-policy environment. Rejected argv,
  environment, ID, since, and category values are absent from errors, and
  suggested fixes never recommend weakening write or content policy.

### Compatibility evidence

- Contract tests deserialize exact new cut and resolve examples through mirror
  structs containing only the unchanged upstream v0.1 fields; unknown
  contract-2 fields are ignored and the private sentinels remain `cwd: "."`
  and `repo: null`.
- Built the actual unchanged `upstream/main` source in an isolated temporary
  checkout and created a new private contract-2 cut with the current binary.
- The v0.1 binary listed the new cut and reported doctor healthy with exit 0.
  Journal SHA-256 remained
  `0f25c215424b4026f5b8790ad216897f3e19a121e465117a49013dd6fac38c09`
  before and after both reads. This is parse/read compatibility only; the old
  output intentionally drops unknown audit fields and remains contract 1.

### Verification

- Unit tests: 17 passed.
- Black-box CLI tests: 53 passed.
- Canonical schema targets `all`, `record`, `error`, and `exit-codes` all emit
  contract exactly 2 with static metadata.
- `cargo build --release`, `cargo test --all-features`,
  `cargo clippy --all-targets --all-features -- -D warnings`, and
  `cargo fmt --check`: pass.
- `git diff --check`: pass.
- Gitleaks scanned the full working tree with no leaks found.
- The current binary diagnosed the four-entry private dogfood journal as
  healthy with no findings and exit 0.
- Scoped UBS completed across the changed command, error, policy, and test
  files. Its non-zero inventory is the known Rust heuristic noise: test-only
  `unwrap`/`assert`/`panic`, ordinary warning-string comparisons mislabeled as
  secret comparisons, and bounds-guarded indexing. UBS independently reports
  clean formatting, Clippy, cargo check, test build, unsafe usage, runtime-regex
  inputs, and resource lifecycle. No production defect remained after review.

### Rollback

Revert the focused implementation through ordinary Git history. Do not rewrite
journals, remove audit fields, or retroactively label legacy records. Older
v0.1 readers remain selectable only with the documented loss of contract-2
semantics and protections.

### Next step

Proceed to `x30.11` for independent adversarial unit and real-binary acceptance
across every public surface. Keep `schema` static and treat any discovered
runtime/schema drift as a defect rather than changing the contract silently.

## 2026-07-12 — Adversarial contract-2 security acceptance

### Outcome

- Completed `br-hardened-papercuts-fork-x30.11` without changing production
  behavior.
- Added `tests/security_acceptance.rs`, a 10-group real-binary suite covering
  every high- and medium-risk category, every persisted input field,
  cross-field AWS detection, override gates, false-positive controls, dry-run
  decisions, storage fallbacks, submodule identity, private path projection,
  invalid UTF-8, parser/config redaction, and mixed/malformed journals.
- Every refusing content fixture now proves both no echo and no write: stdout
  and stderr omit its unique sentinel; missing target parents remain absent;
  existing journal bytes remain identical.
- Added `scripts/security-acceptance.sh`. It records bounded evidence under the
  ignored Cargo target tree and sanitizes repo/shared-target paths before they
  reach the retained log.
- Added `docs/SECURITY_ACCEPTANCE_MATRIX.md`, mapping the consolidated ADR
  requirements to the focused suite and the pre-existing unit/CLI tests.
  Multi-project alias, inventory, and digest behavior remains explicitly
  deferred.

### Verification

- Focused sanitized runner: 10/10 groups pass.
- Full suite: 17 unit, 53 existing CLI, and 10 acceptance tests pass.
- Release build, Clippy with warnings denied, formatting, shell syntax, and
  `git diff --check`: pass.
- Full working-tree Gitleaks: no leaks. An initially detected synthetic
  assignment fixture was source-split without changing its runtime bytes.
- Scoped UBS was non-zero only for test-harness panic/assert heuristics,
  bounded JSON indexing, and disposable-fixture allocations. Its independent
  format, Clippy, check/test-build, unsafe, command-construction, secret, async,
  and resource checks were clean; no production code changed.
- The release binary resolved through Cargo's shared target directory reported
  the then-19-line dogfood journal healthy with no findings. Its legacy path
  and unscanned warnings remained accurate and no journal bytes were rewritten.

### Papercuts observed

- The repo-relative `target/release/papercuts` path failed because this host
  uses a shared Cargo target directory. This is already recorded as
  `pc_68f9f458a3de`, so no duplicate was appended; the verification used
  `cargo metadata --format-version 1 --no-deps` to resolve the binary.
- Markdown hard-break trailing spaces caused the first staged diff check to
  fail. The evidence formatting was corrected and the friction was appended as
  `pc_ec1b21dae2ca`.

### Rollback

Revert the focused acceptance commit. Ignored runner artifacts may be removed
independently; product behavior and append-only journal bytes are unaffected.

### Next step

Proceed automatically to `x30.12` for verified safe single-project agent
instructions and the operator review runbook, then run the independent release
gate in `x30.13`.

## 2026-07-12 — Safe single-project instructions and operator runbook

### Outcome

- Completed `br-hardened-papercuts-fork-x30.12` with a canonical copy-ready
  `AGENTS.md` block and a standalone operator runbook.
- The agent policy preserves task authority: read-only/audit/no-write work
  cannot invoke add or resolve, and autonomous agents cannot enable the
  sensitive-category gate or flag.
- The runbook covers isolated exact-SHA installation, contract preflight,
  deterministic precedence, private/committed/non-Git targets, review and
  resolve cadence, explicit copy-and-verify migration, known scanner/path
  limitations, observable failure modes, and selection-only rollback.
- Reworked README's inherited upstream wording: the hardened default is now
  correctly private, committed is an explicit exposure lane, and installation
  no longer points at the upstream registry package.
- Added `scripts/verify-single-project-runbook.sh`, which executes the
  documented lifecycle in disposable ordinary Git, linked-worktree, migration,
  committed, and non-Git fixtures and emits only a sanitized pass/fail line.
- Remote exact-SHA installation exposed stale schema status and an incorrect
  draft preflight assumption. Updated `adversarial_acceptance` to
  `implemented by x30.11`, used the real `path_projection` key and value form,
  and pinned the status in the CLI contract test.

### Verification

- Disposable runbook lifecycle: pass.
- Full suite: 17 unit, 53 CLI, and 10 security acceptance tests pass.
- Release build, Clippy with warnings denied, formatting, shell syntax,
  `git diff --check`, and full working-tree Gitleaks: pass.
- Dogfood journal doctor: healthy, 20 lines, no findings; retained legacy and
  unscanned warnings remain accurate.
- Scoped UBS was non-zero only for existing test panic/assert heuristics,
  warning-label secret-comparison false positives, bounded JSON indexing, and
  fixture allocations. Its independent build, lint, unsafe, shell, secret, and
  resource checks were clean.

### Papercuts observed

- The destructive-command guard's temporary-cleanup behavior is already
  recorded as `pc_956892a253f7`; no duplicate was added.
- The crates.io availability failure is covered by the existing registry
  availability papercut `pc_d8c79d4b2a1d`; the cached offline install provided
  the bounded verification path without duplicating the entry.

### Rollback

Revert the focused documentation/status commit. Do not rewrite journals;
rollback remains a profile/binary selection decision.

### Next step

Proceed automatically to `x30.13` and run the hardened single-project release
gate against the committed SHA before planning any pilot rollout.

## 2026-07-12 — Hardened single-project release gate

### Verdict

`GO` for exact SHA `804d2b17e65edd865f3dc6e0ec05939aa65cf1ee` in an
isolated-path, allowlisted single-project pilot only. General release,
publication, PATH shadowing, and multi-project rollout remain blocked.

### Evidence

- Fork identity, disabled upstream push, GitHub parent, remotes, merge base,
  and exact SHAs verified; fork is 16 ahead and 0 behind upstream.
- Full 17 unit + 53 CLI + 10 acceptance suite, release build, Clippy, fmt,
  runbook lifecycle, sanitized security runner, and five repetitions of all
  three eight-way race tests passed.
- 10,000-iteration maximum-payload benchmark passed with p95 0.036310 ms and
  maximum 0.802956 ms against 5 ms / 20 ms budgets.
- Gitleaks scanned all 24 public commits with no leaks. The focused artifact
  contained no local absolute paths or acceptance sentinels.
- Locked licenses had no missing declarations or duplicate versions. Isolated
  `cargo-audit` reported zero vulnerabilities and no warnings.
- Dogfood journal remained healthy at 20 lines; compatibility, migration, and
  selection-only rollback evidence remained valid.

Full command, SHA, benchmark, dependency, residual-risk, and rollback evidence
is in `docs/evidence/x30.13-single-project-release-gate-2026-07-12.md`.

### Next step

Proceed to `x30.14` to design the bounded pilot. Planning must name at most four
allowlisted repositories and must not modify them without a separate execution
decision.

## 2026-07-12 — Reversible allowlisted pilot design

- Designed a 14-day pilot for exactly `/data/projects/papercuts` and
  `/data/projects/acfs-workbench`; no target repository was modified.
- Pinned the gated SHA and isolated install root, private/balanced policy,
  exact-path invocation, read-only/no-override rules, activation checks, review
  days, sanitized metrics, stop conditions, 30-day receipt retention, and
  selection-only rollback in `docs/PILOT_PLAN.md`.
- Execution remains isolated in `x30.15` and must stop rather than infer around
  migration, permission, disclosure, contract, or binary-identity failures.

## 2026-07-12 — Pilot activation preflight stopped safely

- Installed and verified the exact gated binary in its isolated SHA-derived
  root; binary SHA matched the release gate.
- No-write probes succeeded in both allowlisted repositories.
- Stopped before activation because `papercuts` requires an explicit legacy
  journal migration decision and `acfs-workbench` has extensive pre-existing
  changes including an already modified `AGENTS.md`.
- No target repository file or journal changed and the 14-day clock did not
  start. Evidence: `docs/evidence/x30.15-activation-preflight-2026-07-12.md`.

## 2026-07-12 — Papercuts pilot repository activated

- With explicit operator approval, removed tracked `.papercuts.jsonl` from the
  current tree without rewriting Git history.
- Did not copy its 20 development dogfood cuts into pilot storage.
- Initialized a fresh empty private journal under the Git common directory with
  directory/file modes `700/600`; doctor is healthy with zero lines.
- Added exact gated-binary, read-only, no-override, and non-sensitive logging
  rules to this repository's `AGENTS.md`.
- The combined 14-day clock remains stopped until `acfs-workbench` is clean and
  separately activated.

## 2026-07-12 — Combined allowlisted pilot activated

- Verified that the earlier `acfs-workbench` `AGENTS.md` commit contained CM
  guidance rather than the required Papercuts pilot marker; the clock remained
  stopped instead of accepting an imprecise commit check.
- Added and pushed only the canonical exact-binary pilot block in
  `acfs-workbench`; all unrelated pre-existing worktree changes remained
  unstaged and unchanged.
- Recorded one real, non-sensitive workflow cut in the ACFS private journal.
  The cut passed balanced policy as clean; doctor then reported one healthy
  line and no findings.
- Proved the ACFS Git status was byte-identical before and after the private
  append: 27 pre-existing entries and the same sanitized SHA-256 fingerprint.
- Verified user-only private state modes `700/600`, contract 2, the gated source
  SHA and binary SHA-256, and healthy private doctors for both aliases.
- Exercised append-only correction in the Papercuts journal: resolved one
  partly inaccurate tooling cut with a non-sensitive note, then logged the
  narrower verified envelope-shape friction as a separate open cut. Doctor is
  healthy with two cuts and one resolution event. The ACFS alias retains its
  one clean workflow signal.
- Exercised three no-write synthetic policy paths with the gated binary:
  balanced email warning, strict email refusal, and high-confidence secret-
  assignment refusal. Exit/category behavior matched contract 2 and the private
  journal hash remained unchanged.
- Started the combined 14-day clock at `2026-07-12T16:07:01+07:00`. Day 1, 3,
  7, and 14 reviews are due on July 13, 15, 19, and 26 respectively. The pilot
  cannot complete before `2026-07-26T16:07:01+07:00`.
- Sanitized activation evidence is in
  `docs/evidence/x30.15-combined-pilot-activation-2026-07-12.md`.

### Next step

Keep `x30.15` in progress. Perform the day-1 review after the elapsed-time gate;
do not replace it with repeated immediate commands or unlock wider adoption.
The synthetic override scenario remains separately approval-gated.
