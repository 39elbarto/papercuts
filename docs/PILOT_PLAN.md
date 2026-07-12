# Reversible Allowlisted Pilot Plan

## Decision

Pilot exact gated SHA `804d2b17e65edd865f3dc6e0ec05939aa65cf1ee`
for 14 calendar days in exactly two repositories:

| Alias | Repository | Owner |
|---|---|---|
| `papercuts` | `/data/projects/papercuts` | repository operator |
| `acfs-workbench` | `/data/projects/acfs-workbench` | repository operator |

No other repository is implicitly included. Adding one requires a plan update
before execution. This planning document does not install a binary or modify an
allowlisted repository.

## Installation identity

```bash
SHA=804d2b17e65edd865f3dc6e0ec05939aa65cf1ee
PC_ROOT="$HOME/.local/opt/papercuts-fork/$SHA"
cargo install --git https://github.com/39elbarto/papercuts \
  --rev "$SHA" --locked --root "$PC_ROOT"
PC="$PC_ROOT/bin/papercuts"
"$PC" schema | jq -e '
  .data.contract == 2 and
  (.data.implementation_status.adversarial_acceptance | startswith("implemented"))
'
```

Pilot instructions must invoke `$PC` by this exact path. Do not add it to
global PATH or replace an upstream binary. Record the SHA and binary SHA-256 in
the activation evidence.

## Per-repository activation

For each allowlisted repository, execution bead `x30.15` must:

1. Capture `git status --porcelain=v1` and existing journal state.
2. Run schema preflight and a no-write private list/doctor probe.
3. Stop on migration, permissions, symlink, repository, or unhealthy-doctor
   findings until the exact state is documented.
4. Install the canonical block from `docs/AGENTS_PAPERCUTS_SNIPPET.md`, adapted
   only to use the exact `$PC` path.
5. Keep profile `private` and policy `balanced`; do not configure committed
   storage or any sensitive override.
6. Confirm the before/after Git status is identical except for the intentional
   `AGENTS.md` pilot instruction change.
7. Record activation time; the 14-day clock begins only after both repositories
   pass activation.

Private implicit storage remains under each Git common directory. The two
repositories never share a journal. No migration may rewrite or delete a
legacy source.

## Task authority

- Normal authorized work may append a non-sensitive papercut and continue.
- Read-only, audit, review, or no-write tasks do not invoke add or resolve.
- The harness should set `PAPERCUTS_READ_ONLY=1` for those tasks.
- Agents never set `PAPERCUTS_ALLOW_SENSITIVE` or use `--allow-sensitive`.
- A refusal is rewritten as a non-sensitive description or left unlogged.

## Observation and review

Operator review occurs on pilot days 1, 3, 7, and 14:

```bash
"$PC" --profile private doctor
"$PC" --profile private list --status open --format md
```

Review warnings for balanced matches, legacy paths, and legacy-unscanned
records. Do not copy matched values or raw journal lines into evidence.

## Metrics

Per alias and combined, retain only sanitized counts:

- sessions in which the instruction was available;
- cuts added and duplicate adds avoided;
- cuts judged useful, promoted to a real issue, resolved, or noise;
- scanner warnings and refusals by category only;
- false positives and suspected misses by category/fixture ID only;
- lock timeouts, doctor findings, migration blocks, and permission failures;
- unexpected Git-status changes;
- any path, personal-data, credential, or customer-context disclosure.

Success after 14 days requires: at least five total observed work sessions,
one useful cut or an explicit low-volume conclusion, zero disclosures, zero
unexpected worktree changes, zero unexplained doctor findings, no autonomous
override, and a documented keep/change/stop decision.

## Stop conditions

Stop both repositories immediately on any disclosure, contract not equal to 2,
wrong binary SHA, unhealthy doctor without an understood pre-existing cause,
unexpected tracked journal/worktree mutation, autonomous override attempt,
private-to-committed fallback, or evidence containing raw detected values.

Pause only the affected repository for a migration/permissions/lock issue that
is contained and sanitized. Record the category and decide repair versus
rollback before resuming.

## Evidence retention

Keep raw journals only in their selected private locations. Store pilot run
receipts under ignored
`target/pilot-evidence/<alias>/<YYYY-MM-DD>/` in this repository, containing
command identities, SHA, exit codes, counts, warning/finding kinds, and hashes
only. Promote a sanitized summary to `docs/evidence/` after day 14. Retain the
summary in Git; remove ignored receipts after 30 days only through an approved
cleanup action.

## Rollback

For each repository, remove the pilot instruction change through normal Git
history and stop invoking the exact pilot binary. Do not rewrite or delete the
journal. If review must continue, select the previous binary/profile by exact
path and record the protection downgrade.

After both repositories stop:

```bash
test "$PC_ROOT" = "$HOME/.local/opt/papercuts-fork/$SHA"
```

The operator may then remove only that confirmed isolated install root. Pilot
execution must not perform automatic deletion.
