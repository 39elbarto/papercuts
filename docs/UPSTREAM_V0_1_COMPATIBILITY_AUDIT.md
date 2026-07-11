# Upstream v0.1.0 Contract and Compatibility Audit

Bead: `br-hardened-papercuts-fork-x30.1`  
Audit date: 2026-07-11  
Mode: product-code read-only; documentation and Beads writeback only

## 1. Snapshot and conclusion

This audit uses these immutable reference points:

- upstream repository: `treygoff24/papercuts`;
- release tag `v0.1.0`: `5d8b827abbd054f5f506d26be865f5b7f573a298`;
- upstream `main`: `ffba2bd453ab0faeadf4f923fc727586958c8d6f`;
- fork baseline before hardening: the same upstream product code;
- upstream issue: `https://github.com/treygoff24/papercuts/issues/1`, open and
  without comments at audit time.

`upstream/main` differs from the release tag only by the first dogfood event in
`.papercuts.jsonl`. The fork contains planning, Beads, and documentation commits,
but no changes under `src/`, `tests/`, `Cargo.toml`, or `Cargo.lock` relative to
`upstream/main`.

The v0.1 implementation is internally coherent and well tested for its stated
contract: append-only storage, locking, deterministic output, fold behavior,
structured errors, dry runs, and concurrency all pass. The central adoption
risk is also confirmed: the intended default persists unrestricted agent text
and absolute local paths into a repository-tracked file. Safe-by-default
hardening therefore changes observable behavior and cannot be treated as a
documentation-only patch.

## 2. Inspected sources

The audit inspected these exact public paths:

- `Cargo.toml` and `Cargo.lock`;
- `README.md`;
- `AGENTS.md`;
- `docs/plans/2026-07-09-papercuts-design.md`;
- `src/main.rs`;
- `src/cli.rs`;
- `src/lib.rs`;
- `src/error.rs`;
- `src/output.rs`;
- `src/store.rs`;
- `src/commands/add.rs`;
- `src/commands/list.rs`;
- `src/commands/resolve.rs`;
- `src/commands/doctor.rs`;
- `src/commands/schema.rs`;
- `tests/cli.rs`;
- `.claude/skills/rust-agent-cli`;
- `.papercuts.jsonl`;
- `LICENSE`;
- upstream release metadata and issue #1 through GitHub.

No private repositories, customer data, runtime transcripts, or unrelated
infrastructure paths were inspected or copied into this report.

## 3. Public CLI contract

### Commands and inputs

The parser exposes five commands in `src/cli.rs:27-38`:

| Command | Contract | Mutation annotation |
|---|---|---|
| `add` (`log` alias) | positional text or stdin, agent, repeated tags, severity, dry-run | appends unless dry-run |
| `list` | status, agent, tag, severity, since, limit, JSON/Markdown | read-only |
| `resolve` | ID prefix, note, agent, dry-run | appends unless dry-run |
| `schema` | all/record/error/exit-codes | read-only |
| `doctor` | diagnose-only | read-only |

Global flags are `--file PATH` and `--pretty` (`src/cli.rs:16-24`). `list`
defaults to open status, limit 50, and JSON (`src/cli.rs:54-70`). `add` defaults
to minor severity (`src/cli.rs:40-52`). `resolve` accepts an optional `pc_`
prefix followed by at least four hexadecimal digits (`src/commands/resolve.rs:95-128`).

`schema` publishes the same surface and labels every command as read-only,
appending, and non-destructive as appropriate (`src/commands/schema.rs:20-41`).

### Output and error envelopes

Normal JSON success is one envelope on stdout. Errors are one envelope on
stderr. Both carry `meta.contract: 1` (`src/output.rs:6-87`). Markdown listing
is the sole raw stdout exception (`src/commands/list.rs:87-95,99-142`). Help and
version are the only parser plaintext exceptions (`src/main.rs:5-20`).

The public error dictionary is centralized in `src/error.rs:18-92`:

| Exit | Error meaning |
|---:|---|
| 0 | success or empty result |
| 1 | doctor findings |
| 2 | invalid argument |
| 65 | invalid input or ambiguous ID |
| 66 | explicit file/ID not found |
| 70 | internal error |
| 74 | I/O error |
| 75 | retryable lock timeout |
| 77 | permission denied |
| 78 | configuration error |

Path-bearing I/O, permission, missing-file, and lock errors interpolate the
resolved path into their message (`src/error.rs:123-173`). Path hardening must
therefore cover errors as well as successful cut records and `meta.file`.

### Clock and agent resolution

`PAPERCUTS_NOW` overrides the clock and is rounded to milliseconds
(`src/lib.rs:84-107`). `main` resolves the clock before dispatching every
command (`src/main.rs:22-24`), so an invalid `PAPERCUTS_NOW` currently prevents
even `schema` or `doctor`; this is pinned by `tests/cli.rs:304-317`.

Agent resolution is flag, then `PAPERCUTS_AGENT`, then harness detection, then
`unknown` (`src/lib.rs:174-192`). The source is returned in metadata and is
covered by `tests/cli.rs:347-394`.

## 4. Storage and record contract

### Discovery precedence

`src/store.rs:37-77` implements:

1. `--file PATH`;
2. non-empty `PAPERCUTS_FILE`;
3. nearest ancestor for which `.git` merely exists, then
   `<root>/.papercuts.jsonl`;
4. `$HOME/.papercuts/log.jsonl` outside a detected repository.

Relative explicit paths are lexically normalized against cwd
(`src/store.rs:86-103`). An absent/empty HOME outside a detected repository is
configuration error 78.

The public design deliberately defines repository-local committed storage as
the default (`docs/plans/2026-07-09-papercuts-design.md:110-121`), and README
offers private storage only as an opt-out (`README.md:64-74`).

### Cut and resolve records

`CutRecord` requires `kind`, `id`, timestamp, agent, unrestricted text, tags,
severity, `cwd`, and optional `repo` (`src/lib.rs:39-50`). `ResolveRecord`
contains ID, timestamp, agent, and optional note (`src/lib.rs:52-59`).

`add` validates only non-empty UTF-8 text, a 10,000-byte maximum, and non-empty
agent identity. It then copies text unchanged and captures cwd/repository using
lossy path conversion (`src/commands/add.rs:17-71`). There is no secret,
credential, PII, or sensitive-path preflight.

The ID is `pc_` plus the first six SHA-256 bytes over a length-prefixed sequence
of timestamp, agent, text, severity, and sorted tags (`src/lib.rs:150-171`).
Paths are not part of the ID. With fixed time and identical user fields, racing
adds converge on one ID. A retry at a later instant creates another ID and is
not exactly-once idempotent.

### Append and locking invariants

Reads acquire a shared file lock; writes acquire an exclusive lock. Both retry
50 times with a 100 ms delay and return retryable exit 75 on exhaustion
(`src/store.rs:105-164`). The runtime does not detect network filesystems; the
local-filesystem restriction is documentation, not enforcement.

Actual `add` creates missing parent directories and the log, then performs
read-fold-check-append inside one exclusive critical section
(`src/commands/add.rs:73-90`, `src/store.rs:118-141`). `resolve` requires an
existing log and performs read-fold-match-append inside one exclusive section
(`src/commands/resolve.rs:37-80`).

An append is serialized into one buffer, heals a torn tail with a newline, uses
`write_all`, and truncates back to the original length if the write fails
(`src/store.rs:175-208`). Durability is best-effort: no per-append fsync.

### Fold behavior

`src/store.rs:211-316` implements the materialized view:

- skip a torn final fragment;
- skip malformed or unknown events with warnings;
- first cut for an ID wins;
- first resolve for an ID wins;
- resolve-before-cut is valid after the full scan;
- unresolved orphan resolves produce warnings;
- sort blocker before major before minor, then timestamp descending, then ID;
- sort tags within records.

`doctor` separately recomputes IDs and reports malformed data, torn lines,
complete conflict markers, unknown kinds, orphan resolves, duplicate cuts, ID
conflicts, and a gitignored journal (`src/commands/doctor.rs:26-216`). Every
finding makes doctor unhealthy and returns exit 1; v0.1 does not distinguish a
non-failing warning severity.

## 5. Side-effect matrix

| Operation | Missing discovered default | Explicit missing file | Filesystem mutation |
|---|---|---|---|
| `schema` | not applicable | not applicable | none |
| `list` | exit 0, virtual empty | exit 66 | none |
| `doctor` | exit 0, healthy empty | exit 66 | none; may run `git check-ignore` for an existing repository-local file |
| `add --dry-run` | returns proposed record | returns proposed record | none, including no parent-directory creation |
| `resolve --dry-run` | exit 66 | exit 66 | shared read only; requires an existing journal |
| `add` | creates directory/file and appends | creates directory/file and appends | yes |
| `resolve` | exit 66 | exit 66 | appends to existing journal only |

The no-write behavior of mutation dry runs is pinned in
`tests/cli.rs:398-418`. Discovery precedence and missing-default behavior are
pinned in `tests/cli.rs:632-728`. A representative live probe against a
disposable directory with an empty `.git` directory confirmed:

- the directory is accepted as a repository without Git validation;
- `add --dry-run` returns absolute `cwd`, `repo`, and `meta.file`;
- `list` and `doctor` report virtual empty state;
- no `.papercuts.jsonl` is created.

## 6. Test coverage baseline

The current suite has 30 tests: six library/parser unit tests and 24 black-box
CLI tests.

Covered surfaces include:

- all command success envelopes and the error/exit matrix
  (`tests/cli.rs:85-127,304-344,954-998`);
- stdin, validation, duplicate add, filters, sorting, limit, since, Markdown,
  and resolve prefix/idempotence (`tests/cli.rs:130-303`);
- agent precedence, dry-run side effects, permissions, and lock timeout
  (`tests/cli.rs:347-448`);
- doctor findings, recomputed IDs, torn-tail healing, and doctor/fold parity
  (`tests/cli.rs:451-629`);
- discovery precedence, explicit versus discovered missing state, `.git` file
  markers, HOME fallback, and relative explicit paths
  (`tests/cli.rs:632-728,860-888`);
- deterministic fixed-clock output, ID hashing, tag sorting, and deterministic
  Markdown (`tests/cli.rs:731-742,832-857,891-904`);
- eight-way distinct add, identical add, and resolve races
  (`tests/cli.rs:745-829`);
- detection of a gitignored journal (`tests/cli.rs:907-953`).

Confirmed gaps relevant to hardening:

- no sensitive-data or PII-like guard exists or is tested;
- no safe/private-by-default storage profile exists or is tested;
- no test proves absence of absolute paths in records, metadata, or errors;
- invalid/stray `.git` markers are accepted by design rather than rejected;
- no CI/test pins the actual minimum Rust toolchain;
- no test/audit rejects absolute or dangling repository symlinks;
- Windows remains untested and network filesystems remain unsupported by
  declared non-goal;
- the mid-write rollback branch is not forced by a portable black-box test.

## 7. Upstream issue #1 classification

### 7.1 Committed default, free text, and absolute paths

Status: **confirmed**.

Evidence:

- README declares committed repository-local mode as the default
  (`README.md:27-29,64-74`);
- `.papercuts.jsonl` is tracked;
- `CutRecord` requires absolute-path fields (`src/lib.rs:39-50`);
- `add` persists arbitrary validated text plus lossy absolute cwd/repo
  (`src/commands/add.rs:17-71`);
- schema advertises absolute cut paths and absolute `meta.file`
  (`src/commands/schema.rs:5-41`);
- path-bearing errors echo resolved paths (`src/error.rs:123-173`).

Classification:

- prominent risk documentation is a generic upstream documentation fix and can
  remain contract 1;
- optional path omission/redaction flags can be upstream-compatible only if
  defaults and required record fields remain unchanged;
- private/safe storage as the fork default, omission/replacement of required
  path fields, and default refusal of sensitive-looking text are fork hardening
  decisions and observable behavior changes;
- AGENTS.md rules for conversational read-only/audit-only tasks are policy,
  because the CLI cannot infer that authorization boundary from argv alone.

### 7.2 Undocumented effective minimum Rust version

Status: **confirmed**.

Evidence:

- `Cargo.toml:4` uses edition 2024;
- `src/store.rs:143-149` uses `File::try_lock` and `try_lock_shared`;
- Cargo metadata reports `rust_version: null`;
- README installation is only `cargo install papercuts` (`README.md:21-25`);
- the audit host builds with Rust/Cargo 1.99 nightly, which does not validate
  older supported compilers.

Classification: generic upstream manifest/documentation fix. Add an accurate
`rust-version` (issue #1 identifies 1.89 due to the locking APIs), document it,
and ideally test it. This does not require a papercuts contract-version change.

### 7.3 Dangling absolute skill symlink

Status: **confirmed**.

`.claude/skills/rust-agent-cli` is a tracked symlink to
`/Users/treygoff/.agents/skill-library/rust-agent-cli`; it is broken in this
checkout and publishes an upstream-local username/path.

Classification: generic upstream repository-hygiene fix. Remove it, vendor the
intended public skill, or replace it with a valid repository-relative link.
No CLI contract-version change is needed. This fork should remove it before a
hardened public release even if upstream has not yet done so.

### 7.4 `.git` existence instead of valid repository detection

Status: **confirmed and pinned by current tests**.

`find_repo_root` accepts the first ancestor where `.git` merely exists
(`src/store.rs:79-84`). The discovery test intentionally writes a `.git` file
whose target need not be valid and expects it to define the root
(`tests/cli.rs:632-652`). The disposable live probe showed an empty `.git`
directory is also accepted.

Classification:

- documenting the existence heuristic is a generic upstream documentation fix;
- silently replacing it with `git rev-parse` would add a Git-binary runtime
  dependency and contradict the current no-git-library/simple-marker design;
- stricter validation is a fork architecture decision that must define behavior
  when Git is unavailable, a worktree `gitdir` target is missing, or a marker is
  malformed;
- changing default discovery for existing marker-shaped directories is an
  observable behavior change and should be versioned with the hardened contract
  unless upstream explicitly accepts it as a v0.1 bugfix.

## 8. Compatibility matrix for future work

| Surface | v0.1 requirement | Recommended fork posture | Contract impact |
|---|---|---|---|
| Append-only cut/resolve journal | preserve | preserve exactly; never rewrite existing journals | none |
| First-cut/first-resolve fold and deterministic sort | preserve | preserve and regression-test | none |
| Structured stdout/stderr envelopes | preserve | preserve; keep Markdown/help/version exceptions | none |
| Existing exit meanings | preserve | preserve; add new policy errors deliberately | additive only if old commands keep behavior; otherwise versioned |
| Locking and duplicate safety | preserve | preserve; avoid touching store path unless required | none |
| Committed repo-local default | intentional v0.1 behavior | select safe default plus explicit legacy mode in decision Bead | behavioral break; hardened contract/version required |
| Required absolute `cwd`/`repo` fields | contract 1 record shape | omit, relativize, or replace only after ADR | record-shape break; hardened contract/version required |
| Absolute `meta.file` and path-bearing errors | contract 1 behavior | minimize/redact consistently with record policy | output-contract change; hardened contract/version required |
| Any non-empty UTF-8 text up to 10 KB accepted | v0.1 behavior | local bounded guardrail with explicit override/legacy story | default refusal is behavioral break; version required |
| `.git.exists()` discovery | documented and tested | document now; decide strict mode/default later | opt-in strict mode can be additive; default change should be versioned |
| `PAPERCUTS_FILE`, agent, and clock env vars | preserve | preserve precedence unless ADR explicitly supersedes it | none or explicitly versioned |
| Diagnosing gitignored logs as unhealthy | v0.1 behavior | reconsider only together with safe/private default | likely behavior change; version with storage decision |
| MSRV declaration | absent | add manifest/docs/CI pin | no CLI contract change |
| Broken absolute skill symlink | repository artifact | remove or replace safely | no CLI contract change |
| Multi-project inventory and promotion adapters | non-goal v0.1 | separate later allowlisted layer; journals remain authoritative | new additive surface, not part of single-project compatibility |

The exact hardened contract number/name remains an ADR decision. This audit's
minimum recommendation is that changing defaults, required record fields, or
accepted input behavior must not be shipped silently under an unchanged
`meta.contract: 1` promise.

## 9. Baseline verification

Executed against unchanged product code:

```text
cargo build --release                                      PASS
cargo test --all-features                                  PASS: 30 tests
cargo clippy --all-targets --all-features -- -D warnings  PASS
cargo fmt --check                                          PASS
cargo run --quiet -- doctor                                PASS: healthy, 2 lines
```

Representative disposable-directory probes also passed:

- `schema` returned contract 1 and all declared command/error/storage fields;
- `add --dry-run` returned `changed:false` and created no journal;
- missing discovered `list` returned empty with exit 0;
- missing discovered `doctor` returned healthy empty with exit 0.

The first probe assumed a repository-local `target/release/papercuts`, but this
host's Cargo configuration resolves the target directory to a shared cache; the
probe was repeated successfully using `cargo metadata.target_directory`. This
is a local workflow papercut, not a papercuts product-contract failure.

## 10. Decision handoff

This audit intentionally does not choose the hardened defaults. It unblocks the
four next decision Beads:

- `br-hardened-papercuts-fork-x30.2`: safe storage profiles and read-only task
  semantics;
- `br-hardened-papercuts-fork-x30.3`: path minimization and project identity;
- `br-hardened-papercuts-fork-x30.4`: deterministic sensitive-data guardrail;
- `br-hardened-papercuts-fork-x30.6`: upstream contribution, naming, and sync
  strategy.

Those Beads must treat append-only history, folding, machine envelopes,
deterministic IDs, concurrency, and legacy readability as preservation
constraints rather than incidental implementation details.
