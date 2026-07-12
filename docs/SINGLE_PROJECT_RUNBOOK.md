# Single-project Papercuts Contract-2 Runbook

## Purpose

- Adopt and operate the reviewed fork in one explicitly selected project.
- Keep normal journal writes private by default while preserving an explicit
  repository-visible compatibility lane.
- Give agents and operators executable rules for logging, review, migration,
  read-only work, and rollback.

Skipping the preflight can select the wrong binary, create a migration block,
or expose accepted record content through the committed profile.

## Scope

In scope: one repository or one explicit non-Git journal using machine contract
exactly 2.

Out of scope: directory scanning, project aliases, multi-project inventory or
digests, automatic ClickUp/Beads promotion, telemetry, and autonomous sensitive
input overrides.

## Owner

The repository operator owns installation, profile selection, migration,
review cadence, and any exact sensitive-category override. Agents may append
ordinary non-sensitive friction only within the authority of their current
task.

## Trigger / Frequency

- Run installation and preflight once per selected binary SHA.
- Review open papercuts weekly during a pilot, then choose a cadence from the
  observed volume.
- Run `doctor` before review, after a migration copy, and before changing the
  selected binary or profile.

## Prerequisites

- Rust/Cargo and Git.
- `jq` for the verification commands below.
- A reviewed commit from `https://github.com/39elbarto/papercuts`.
- Write access to the selected private state directory or explicit journal.
- Explicit permission to mutate the project. A read-only task is not an
  installation or logging authorization.

## Inputs

- `SHA`: the full reviewed fork commit.
- `PC_ROOT`: an isolated install root that does not shadow upstream on PATH.
- The selected repository and, if needed, its legacy `.papercuts.jsonl`.
- The canonical agent block in `docs/AGENTS_PAPERCUTS_SNIPPET.md`.

## Procedure

### Step 1: Install an unambiguous binary

```bash
SHA=<full-reviewed-commit>
PC_ROOT="$HOME/.local/opt/papercuts-fork/$SHA"
cargo install --git https://github.com/39elbarto/papercuts \
  --rev "$SHA" --locked --root "$PC_ROOT"
PC="$PC_ROOT/bin/papercuts"
test "$("$PC" --version)" = "papercuts 0.1.0"
```

Invoke `$PC` by exact path during the pilot. Do not put it ahead of an upstream
`papercuts` binary on global PATH. The package version remains `0.1.0`; the full
SHA and isolated root are therefore the identity boundary.

### Step 2: Preflight the machine contract

```bash
"$PC" schema | jq -e '
  .ok == true and
  .data.contract == 2 and
  (.data.implementation_status.storage_policy | startswith("implemented")) and
  (.data.implementation_status.path_projection | startswith("implemented")) and
  (.data.implementation_status.sensitive_preflight | startswith("implemented")) and
  (.data.implementation_status.adversarial_acceptance | startswith("implemented"))
'
```

Stop if any assertion fails. Do not infer contract-2 behavior from the package
version alone.

Configuration precedence is deterministic:

- `--profile` beats `PAPERCUTS_PROFILE`; otherwise the default is `private`.
- `--file` beats `PAPERCUTS_FILE`, which beats the profile default target.
- An explicit file changes only the target; it does not change the selected
  profile or path/content policy.
- `--sensitive-policy` beats `PAPERCUTS_SENSITIVE_POLICY`, but cannot weaken
  the profile floor: private is at least balanced and committed is strict.
- `--read-only` and `PAPERCUTS_READ_ONLY=1` combine monotonically. There is no
  flag that can negate an environment read-only guard.
- `--allow-sensitive` is effective only together with the operator-controlled
  `PAPERCUTS_ALLOW_SENSITIVE=1` gate and exact refusing categories. There is no
  wildcard, off mode, or unchecked profile.

### Step 3: Inspect current project state without writing

From the repository root:

```bash
git status --porcelain=v1
test -e .papercuts.jsonl && echo legacy-journal-present || true
"$PC" --profile private list --status all
```

An absent private journal returns exit 66 and creates nothing. If only the
legacy repository journal exists, private list/doctor warns and private
mutation returns `migration_required`; continue at Step 8. If both exist,
private selects only the private journal and leaves both unchanged.

### Step 4: Install the agent policy

Copy the Markdown block from `docs/AGENTS_PAPERCUTS_SNIPPET.md` into the target
repository's `AGENTS.md`. Keep the read-only and override prohibitions intact.

For a harness-enforced no-write session, set:

```bash
export PAPERCUTS_READ_ONLY=1
```

Agents must still refrain from invoking `add` or `resolve`; the environment
guard is defense in depth. `schema`, `list`, and `doctor` remain available.

### Step 5: Add a normal private papercut

```bash
before=$(git status --porcelain=v1)
"$PC" --profile private add \
  "test command required the package working directory" \
  --agent operator --tag tooling
after=$(git status --porcelain=v1)
test "$before" = "$after"
```

In a valid Git repository, implicit private storage is
`<git-common-dir>/papercuts/log.jsonl` with user-only permissions. Linked
worktrees share that journal; a submodule has its own Git administration
directory and its own journal. Outside Git, actual private mutation requires an
explicit `--file` or `PAPERCUTS_FILE`.

A new private record uses `cwd:"."`, `repo:null`, `path_policy:"omitted"`, and
`path_encoding:"omitted"`. Private success metadata omits `file`.

### Step 6: Review, diagnose, and resolve

```bash
"$PC" --profile private doctor
"$PC" --profile private list --status open --format md
"$PC" --profile private list --status all | jq -r '.data.items[].id'
"$PC" --profile private resolve pc_0123abcd4567 \
  --agent operator --note "documented the correct working directory"
"$PC" --profile private doctor
```

Replace the example ID with a listed ID. A unique prefix of at least four hex
digits is accepted. `resolve` appends an event and never rewrites the cut.

Recommended weekly review:

1. Stop on unhealthy `doctor` output and inspect the finding kinds.
2. Review open items severity-first.
3. Promote real bugs or planned work into the project's normal tracker.
4. Resolve only after the underlying friction is fixed or deliberately
   accepted.
5. Record only counts, IDs, and sanitized outcomes in review evidence.

### Step 7: Use the committed exposure lane only deliberately

```bash
"$PC" --profile committed add \
  "repository-visible example without private context" \
  --agent operator --tag docs
git diff -- .papercuts.jsonl
```

Committed profile writes `.papercuts.jsonl` in a valid repository, records
legacy absolute context, emits `legacy_absolute_path_exposure`, and enforces a
strict sensitive-data floor. It is an exposure/compatibility lane, not an
unchecked or generally safer mode. An explicit `--file` changes the target,
not the selected profile or path policy.

### Step 8: Copy a legacy journal into private storage

Only run this when `.papercuts.jsonl` exists, the private target does not, and
`doctor` on the legacy source is acceptable.

```bash
"$PC" --profile committed doctor
COMMON=$(git rev-parse --path-format=absolute --git-common-dir)
PRIVATE="$COMMON/papercuts/log.jsonl"
test ! -e "$PRIVATE"
install -d -m 700 "$(dirname "$PRIVATE")"
install -m 600 .papercuts.jsonl "$PRIVATE"
cmp -s .papercuts.jsonl "$PRIVATE"
"$PC" --profile private --file "$PRIVATE" doctor
"$PC" --profile private list --status all
```

The copy preserves historical bytes, including old absolute paths and missing
content audits. Private output projects those paths safely, but the source and
copied journal still retain them. Migration does not sanitize Git history,
merge history, backups, or the journal bytes. Do not delete or rewrite the
legacy source as part of this procedure.

## Verification

Run the checked-in disposable verification:

```bash
scripts/verify-single-project-runbook.sh
```

Expected result: `single-project runbook verification: pass`. It builds and
locally installs the current tree into a temporary isolated root, then verifies
schema, private lifecycle, worktree sharing, explicit non-Git storage,
read-only refusal, committed warnings, migration copy/readback, and rollback
selection.

For the real selected repository also verify:

```bash
"$PC" schema | jq -e '.data.contract == 2'
"$PC" --profile private doctor
git status --porcelain=v1
```

## Failure Modes / Edge Cases

- `writes_disabled`: the current task or environment is read-only. Do not
  bypass it; continue without logging.
- `migration_required`: follow Step 8 or explicitly continue selecting
  committed. No automatic merge occurs.
- `storage_required`: non-Git private mutation needs an explicit journal.
- `sensitive_input`: replace the value with a non-sensitive description. Do
  not print or recommend an override command.
- `insecure_private_permissions`, `unsafe_journal_symlink`, or
  `invalid_repository`: stop. Correct permissions/metadata explicitly; private
  mode never falls back to committed or HOME.
- `lock_timeout`: retry after the competing writer finishes; exit 75 is marked
  retryable.
- Non-UTF-8 committed paths are lossy and warned. Private automatic path
  projection remains omitted.
- Contract-1 records remain readable and are reported as legacy/unscanned;
  this is compatibility, not retroactive scanning.

The local scanner is bounded and deterministic, not comprehensive. It does not
perform entropy analysis, recursive decoding, Unicode normalization,
homoglyph detection, network checks, or Gitleaks on every add. Private balanced
mode warns and persists medium-risk matches; accepted values and authorized
overrides are stored verbatim. Never rely on the scanner as redaction.

Policy version 1 bounds UTF-8 input to 10,000 bytes of text, 2,000 bytes of
resolution note, 64 bytes per tag, 16 tags, 128 bytes of agent name, and 16,384
bytes total. Persisted audit decisions are `clean`, `warn`, or `override`.
Missing content audit means `legacy-unscanned`; it is never synthesized by a
reader.

## Rollback / Abort

Rollback is selection-only:

```bash
"$PC" --profile committed list --status all
```

Or invoke the previously recorded upstream binary by its exact path. Do not
rewrite, merge, strip fields from, or delete either journal. Selecting upstream
v0.1 loses contract-2 profile, path, content, and diagnostic protections and
must be recorded as an explicit protection downgrade.

To remove the isolated binary without touching journals:

```bash
rm -rf -- "$PC_ROOT"
```

Confirm the variable is the recorded isolated install root before removal.

## Logs / Observability

- Authoritative journal: selected JSONL file.
- Health: `papercuts doctor` exit and finding kinds.
- Review: `papercuts list --status open --format md`.
- Focused security evidence: ignored `target/security-acceptance/<run-id>/`.
- Runbook verification emits only its final result; temporary fixtures are
  removed on exit.

## Review Cadence

Review this runbook whenever schema contract changes, a storage/path/content
policy changes, the binary identity strategy changes, or pilot evidence finds
a missed disclosure or repeated false positive.
