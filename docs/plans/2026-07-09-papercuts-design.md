# papercuts — design doc

2026-07-09. Coordinator-authored. Status: r2 — amended after adversarial review (see Amendments).

## Thesis and provenance

Agents hit friction constantly — dead-end tool calls, broken links, missing helpers, footgun configs — and silently push through without telling anyone. The signal evaporates. `papercuts` is a tiny agent-first CLI that gives agents a one-line way to file the complaint at the moment they hit it, and gives humans (and other agents) a way to review and burn down the backlog.

Provenance: Steve Ruiz shipped a private version of this inside his repo (X post, 2026-07-09, 39K views / 770 bookmarks in hours) and reported it immediately surfaced real workflow defects his agents had been eating silently: unquoted zsh globs breaking `rg`, wrong test cwd in a yarn workspace, tab-indented YAML breaking deploys, stale Supabase CLI ambiguity. Every one is an actionable fix a human would never have heard about otherwise. This is a validated behavior pattern, not a speculative product.

Why a CLI and not an MCP server or harness feature: every agent harness (Claude Code, Codex, Cursor, Droid, anything) can shell out. A single static binary with a JSON contract is the lowest common denominator and needs zero per-harness integration. One line in an AGENTS.md/CLAUDE.md activates it.

## External contract

Binary and crate: `papercuts` (crates.io name verified free 2026-07-09; bare `papercut` is taken by an image tool). Repo: `treygoff24/papercuts`. License: MIT.

### Commands

```text
papercuts add <TEXT | ->        # file a papercut ('-' reads text from stdin)
papercuts list                  # read papercuts (default: open only, newest first)
papercuts resolve <ID>          # mark a papercut resolved (append-only event)
papercuts schema [all|record|error|exit-codes]   # machine contract, self-orientation
papercuts doctor [--fix]        # validate the log file, quarantine malformed lines
```

`log` is an alias of `add` (the verb people will guess from Steve's post); `add` is canonical.

Global flags: `--file <PATH>` (explicit log file, overrides discovery), `--pretty`, `--quiet`. No color anywhere, ever (agent-only tool; there is nothing to colorize — output is JSON).

### `add`

- **Idempotent**: if the computed ID already exists in the log, nothing is appended; the existing record is returned with `data.changed: false` and `meta.warnings: ["duplicate of existing cut; nothing appended"]`. A fresh append returns `data.changed: true`. This makes agent retries safe and makes post-merge duplicate lines self-healing (first-wins fold).
- Positional `TEXT` (or `-` for stdin; stdin also used when text is omitted and stdin is non-TTY).
- `--agent <NAME>`: reporter identity. Resolution order: flag → `PAPERCUTS_AGENT` env → harness detection (`CLAUDECODE`→`claude-code`, `CODEX_*`→`codex`, `CURSOR_*`→`cursor`) → `"unknown"`. The resolved value AND its source (`flag|env|detected|default`) are echoed in output meta — no silent ambient inference.
- `--tag <TAG>` (repeatable), `--severity minor|major|blocker` (default `minor`).
- Captures `cwd` and repo root automatically (filesystem walk for `.git`; no libgit2).
- Output: success envelope containing the full record + `meta.file` (resolved log path) + `meta.agent_source`.

### `list`

- Filters: `--status open|resolved|all` (default `open`), `--agent`, `--tag`, `--severity`, `--since <RFC3339 | Nd | Nh>`.
- `--limit N` (default 50) — bounded output by default; envelope carries `count`, `total`, `truncated`.
- `--format json|jsonl|md` (default `json`). `md` is the one human-facing surface: a compact review digest grouped by severity.
- Empty result is exit 0 with an empty array and a hint in `meta.warnings` — never exit 1.

### `resolve`

- `papercuts resolve <ID> [--note <TEXT>] [--agent <NAME>] [--dry-run]`. Appends a `resolve` event; never rewrites history. `--dry-run` reports what would be appended without writing. Output includes `data.changed: bool`.
- Unknown ID → structured `not_found` error, exit 66, with a hint naming `papercuts list --status all`.
- Already-resolved ID → **idempotent success** with `meta.warnings: ["already resolved"]` (agents retry; retries must be safe).
- ID prefix matching: a unique prefix (≥4 chars) resolves; an ambiguous prefix errors listing the candidates (deterministic forgiveness — never guess between two).

### `schema`

Prints the full machine contract as JSON: contract version, every command/flag, record schemas, error codes, exit-code dictionary. This is the self-orientation surface; an agent that has never seen the tool runs `papercuts schema` and knows everything.

### `doctor` (v1: diagnose-only)

- Validates the log file: every line parses as a known event, IDs well-formed; reports torn last line, git conflict markers (`<<<<<<<`), unknown kinds, orphan resolves, duplicate cut lines — each with line numbers.
- Duplicate cut lines are a **warning, not an error** (expected after git concat-merges; `list` folds them first-wins).
- If the `git` binary is available and the log lives in a repo, warns when the log path is gitignored (the diff-visibility feature silently off).
- **No `--fix` in v1** (review finding: an unguarded quarantine that eats a mis-judged line is worse than no fix; a safe fix path needs backup/undo/dry-run — v2). Exit dictionary: 0 healthy / 1 findings, published in `schema`.

### Envelope and exit codes

Success: `{"ok":true,"data":{…},"meta":{…}}` on stdout, single line (or pretty with `--pretty`).
Error: `{"ok":false,"error":{"code":"…","message":"…","details":{…},"retryable":bool,"suggested_fix":"paste-ready command"}}` on **stderr**.

Exit codes follow the rust-agent-cli skill dictionary: 0 success/empty, 2 usage, 65 bad input data, 66 missing file / not-found ID, 70 internal, 78 config — plus **74 (I/O error) as a documented extension** to the skill table (deliberate deviation, published in `schema`; implementer must not "fix" this back). No network, no auth → 75/77 unused. Doctor uses its own published dictionary.

Every envelope (success and error) carries `meta.contract: 1` so consumers can detect contract skew. `schema` output includes an env-var inventory (`PAPERCUTS_FILE`, `PAPERCUTS_AGENT`, `PAPERCUTS_NOW`) and per-command `read_only`/`appends`/`destructive` annotations.

### Record shapes (contract v1)

Cut event:

```json
{"kind":"cut","id":"pc_a1b2c3d4e5f6","ts":"2026-07-09T18:30:00.123Z","agent":"claude-code","text":"rg failed: unquoted zsh glob expanded before rg ran; quote globs or use --files","tags":["shell","rg"],"severity":"minor","cwd":"/Users/x/proj/apps/web","repo":"/Users/x/proj"}
```

Resolve event:

```json
{"kind":"resolve","id":"pc_a1b2c3d4e5f6","ts":"2026-07-10T09:00:00.000Z","agent":"trey","note":"added rg wrapper to CLAUDE.md"}
```

- `id` = `pc_` + first 12 hex of SHA-256 over `ts|agent|text` — content-addressed, deterministic, collision-negligible at this scale.
- `ts` = UTC RFC3339 milliseconds. `PAPERCUTS_NOW` env (RFC3339) overrides the clock for reproducible tests — documented, not hidden.
- Unknown `kind` values are skipped by `list` with a `meta.warnings` count (forward compatibility) but flagged by `doctor`.

## Storage

**Append-only JSONL, event-sourced.** Per the state-and-persistence reference: append-only + no transactional check-then-act = JSONL, not SQLite. `resolve` is an appended event, not a mutation, so the file is never rewritten in normal operation (only `doctor --fix` rewrites, atomically). `list` folds cut+resolve events into current state at read time — trivial at the scale of a papercuts log (thousands of lines, single-digit ms).

File discovery order:

1. `--file PATH` flag
2. `PAPERCUTS_FILE` env
3. Walk up from cwd to the git repo root; use `<repo-root>/.papercuts.jsonl` (created on first `add`)
4. No repo → `~/.papercuts/log.jsonl`

The per-repo default is the point: the log travels with the repo, and every `add` shows up in `git diff` — exactly how Steve's screenshot surfaced (the green block IS the diff). Teams see papercuts in review for free. This is deliberately committed-by-default (owner decision, review risk acknowledged); the README documents the opt-out (`echo .papercuts.jsonl >> .gitignore` + `PAPERCUTS_FILE`) and recommends `.papercuts.jsonl merge=union` in `.gitattributes` so branch merges concat instead of conflicting. The fold rules below make concat-merges (including duplicated lines) safe.

Repo-root detection treats `.git` as a root marker whether it is a **directory or a file** (worktrees and submodules use a `.git` file).

Concurrency: writes open with `O_APPEND`, take an exclusive `std::fs::File::lock` (stabilized std — no locking dep), serialize the full line to one buffer, and land it with a **single `write` call**, flush, unlock. Reads take a shared lock. Multiple concurrent agents on one file are safe. Durability is best-effort (no fsync per append — documented; a papercut lost to a power cut is acceptable). Advisory locks are only claimed for **local filesystems**; network mounts (NFS/SMB) are documented as unsupported.

### `list` fold algorithm (normative)

1. Read lines in file order. A final line without a trailing `\n` is **torn**: skip it, count it in `meta.warnings`, never fail the whole read.
2. Lines that fail to parse, or parse to an unknown `kind`, are skipped and counted in `meta.warnings` (forward compatibility; `doctor` reports them with line numbers).
3. `cut` events: **first occurrence of an ID wins**; later duplicates are ignored (this is what makes git concat-merges and idempotent-add races self-healing).
4. `resolve` events: mark the ID resolved, recording the **first** resolve's `ts`/`agent`/`note`. A resolve whose ID has not been seen *by end of file* is an **orphan**: counted in `meta.warnings`, otherwise ignored (a resolve line may legitimately precede its cut line after a merge, so resolution status is computed after the full scan).
5. Sort for output: severity rank (blocker > major > minor), then `ts` descending, then `id` ascending; tags sorted within each record. Same ordering for every format — `md` output is deterministic.

`--since` semantics: relative durations (`Nd`/`Nh`) are computed against the effective now (`PAPERCUTS_NOW` if set, else wall clock UTC). Absolute values must be full RFC3339 with offset (`Z` accepted); date-only input is rejected with a `suggested_fix` showing both forms (ambiguous timezone — reject, don't guess).

## Dependencies (each justified)

- `clap` 4 (derive) — parser, per skill.
- `serde` + `serde_json` — every output shape is a struct.
- `thiserror` — typed public error contract.
- `jiff` — RFC3339 UTC timestamps, parsing `--since`. (Frozen choice — implementer must not substitute.)
- `sha2` — content-addressed IDs.
- Dev: `assert_cmd`, `predicates`, `tempfile`.

Nothing else. No tokio, no color crates, no config-file crate, no git library.

## Testing strategy

- Parser unit tests via `Cli::try_parse_from` (conflicts, defaults, bad values).
- Black-box CLI tests via `assert_cmd`: every command's success shape deserialized into its envelope struct; every error path asserts code + exit code + that the `suggested_fix` hint survives (pinned per the error-rewriting craft).
- Concurrency test: N threads `add` simultaneously against one file; assert N valid lines, no interleaving/corruption.
- Determinism test: two identical invocations with `PAPERCUTS_NOW` fixed produce byte-identical output.
- Quality gate: `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test`, `cargo build --release`. 5x test sweep before any commit.
- Live acceptance (coordinator-driven): drive the real binary through the full agent lifecycle including empty states, malformed file, ambiguous prefixes, concurrent adds, stdin path.

## Distribution / ship plan

- Public GitHub repo `treygoff24/papercuts`, README written for two audiences: the human installing it, and the agent using it (an AGENTS.md-ready snippet to paste into any repo's agent instructions).
- `cargo install papercuts` as the v0.1.0 install path; `cargo publish` at ship.
- cargo-dist/homebrew/curl-installer deferred to a follow-up release (lens playbook exists; not v0.1 scope).

## Non-goals (v1)

- No server, sync, or telemetry — the file is the product.
- No TUI, no interactive anything.
- No dedup/clustering/AI summarization of cuts (the reviewing agent can do that; this tool is the substrate).
- No Windows CI (nothing platform-specific in the design; just untested).
- No `edit`/`delete` of history — append-only is a feature; nothing rewrites the file in v1 (`doctor --fix` deferred to v2 with backup/undo/dry-run).
- No config file.
- No `--correlation-id` (single-shot local CLI with no logs to correlate — echo-only ceremony; revisit if a long-running mode ever exists).

## Amendments (r2, from adversarial review 2026-07-09)

Reviewers: Cursor (Grok 4.5) `safe`, delivered; Codex GPT-5.6 Sol xhigh attempted twice (work-account quota exhaustion, then expired personal token) — re-run post re-auth or substituted per lane availability. Triage of all Cursor findings:

**Accepted (folded into the doc above):** torn-last-line handling (single-write append + skip-with-warning on read); idempotent `add` resolving the duplicate-ID/determinism contradiction; normative fold algorithm (first-cut-wins, orphan resolves, post-scan status); `.git`-file root detection (worktrees); exit-74-as-documented-extension; `meta.contract` version on every envelope; `--dry-run` + `changed:bool` on mutations; doctor demoted to diagnose-only (cut `--fix`); doctor gitignore check; `--since` semantics pinned; deterministic md sort; jiff frozen; local-fs-only locking note; best-effort durability note; `merge=union` README guidance.

**Accepted-reduced:** NFS handling = documentation only, no runtime network-fs detection (unreliable heuristics); prefix-resolve stays ≥4 chars but all emitted examples use full IDs.

**Rejected with reasons:** `--correlation-id` (see non-goals); `meta.ignored_by_git` on every `add`/`list` (spawning `git check-ignore` per invocation buys little — doctor covers it); runtime `tempfile` dep (moot — no rewrite path in v1); Windows lock-behavior work (stays a documented non-goal).

## Wave plan

Slimmed foundry (reduced config: Codex authors and fixes, cross-family review via Cursor/Grok, coordinator independently gates and reads riskiest files).

- **Plan review** (this doc): `delegate codex safe --model sol --reasoning-effort xhigh` + `delegate cursor safe` in parallel; coordinator triages all findings in writing; doc amended.
- **Wave 1 — the whole CLI, one lane** (task-clustering: ~1000 LOC sharing one design; splitting would fragment coherence): `delegate codex work --model sol --reasoning-effort high`. Layout per skill: `main.rs`/`cli.rs`/`commands/`/`output.rs`/`error.rs`/`lib.rs`/`tests/`.
- **Review wave**: `delegate cursor safe` adversarial review of the diff + coordinator riskiest-file read (locking/append path, ID fold logic in `list`, torn-line handling). Triage → Codex fix round → coordinator verifies every fix landed → re-review until dry (3-round cap).
- **Acceptance**: coordinator drives the real binary. Zero unexplained failures.
- **Ship**: README/AGENTS.md, GitHub repo + push, tag v0.1.0, `cargo publish`.

Budget: subscription lanes only (Codex + Cursor); zero metered spend expected.
