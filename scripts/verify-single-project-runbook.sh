#!/usr/bin/env bash
set -euo pipefail

ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
TMP=$(mktemp -d)
trap 'rm -rf -- "$TMP"' EXIT

if ! cargo build --release --manifest-path "$ROOT/Cargo.toml" >/dev/null 2>&1; then
  printf '%s\n' 'single-project runbook verification: build failed' >&2
  exit 1
fi
if ! cargo install --path "$ROOT" --locked --offline --root "$TMP/install" --force \
  >/dev/null 2>&1
then
  printf '%s\n' 'single-project runbook verification: local install failed' >&2
  exit 1
fi
PC="$TMP/install/bin/papercuts"

grep -F 'This instruction does not grant permission to write' \
  "$ROOT/docs/AGENTS_PAPERCUTS_SNIPPET.md" >/dev/null
grep -F 'Never set `PAPERCUTS_ALLOW_SENSITIVE`' \
  "$ROOT/docs/AGENTS_PAPERCUTS_SNIPPET.md" >/dev/null

"$PC" schema >"$TMP/schema.json"
jq -e '
  .ok == true and
  .data.contract == 2 and
  (.data.implementation_status.storage_policy | startswith("implemented")) and
  (.data.implementation_status.path_projection | startswith("implemented")) and
  (.data.implementation_status.sensitive_preflight | startswith("implemented")) and
  (.data.implementation_status.adversarial_acceptance | startswith("implemented"))
' "$TMP/schema.json" >/dev/null

REPO="$TMP/repo"
git init -q "$REPO"
git -C "$REPO" config user.email runbook@example.invalid
git -C "$REPO" config user.name Runbook
before=$(git -C "$REPO" status --porcelain=v1)
(
  cd "$REPO"
  "$PC" --profile private add "private lifecycle" --agent runbook --tag tooling
) >"$TMP/private-add.json"
after=$(git -C "$REPO" status --porcelain=v1)
test "$before" = "$after"
jq -e '
  .data.changed == true and
  .data.record.cwd == "." and
  .data.record.repo == null and
  .data.record.path_policy == "omitted" and
  .meta.file == null
' "$TMP/private-add.json" >/dev/null
PRIVATE="$REPO/.git/papercuts/log.jsonl"
test -f "$PRIVATE"
test "$(stat -c %a "$(dirname "$PRIVATE")")" = 700
test "$(stat -c %a "$PRIVATE")" = 600

(
  cd "$REPO"
  "$PC" --profile private list --status all
) >"$TMP/private-list.json"
id=$(jq -r '.data.items[0].id' "$TMP/private-list.json")
(
  cd "$REPO"
  "$PC" --profile private resolve "$id" --agent runbook --note "verified"
  "$PC" --profile private doctor
) >"$TMP/private-lifecycle.jsonl"

before_hash=$(sha256sum "$PRIVATE" | cut -d' ' -f1)
set +e
(
  cd "$REPO"
  PAPERCUTS_READ_ONLY=1 "$PC" --profile private add "must not write"
) >"$TMP/read-only.stdout" 2>"$TMP/read-only.stderr"
read_only_exit=$?
set -e
test "$read_only_exit" = 78
jq -e '.error.code == "writes_disabled"' "$TMP/read-only.stderr" >/dev/null
test "$before_hash" = "$(sha256sum "$PRIVATE" | cut -d' ' -f1)"

git -C "$REPO" commit -q --allow-empty -m base
WORKTREE="$TMP/worktree"
git -C "$REPO" worktree add -q -b runbook-worktree "$WORKTREE"
(
  cd "$WORKTREE"
  "$PC" --profile private add "shared worktree journal" --agent runbook
) >"$TMP/worktree.json"
test "$(wc -l <"$PRIVATE")" = 3

NON_GIT="$TMP/non-git"
mkdir -p "$NON_GIT"
EXPLICIT="$TMP/explicit/log.jsonl"
(
  cd "$NON_GIT"
  "$PC" --profile private --file "$EXPLICIT" add "explicit non git" --agent runbook
) >"$TMP/non-git.json"
test -f "$EXPLICIT"
jq -e '.meta.storage_source == "flag-file" and .meta.file == null' "$TMP/non-git.json" >/dev/null

COMMITTED_REPO="$TMP/committed"
git init -q "$COMMITTED_REPO"
(
  cd "$COMMITTED_REPO"
  "$PC" --profile committed add "committed exposure lane" --agent runbook
) >"$TMP/committed.json"
test -f "$COMMITTED_REPO/.papercuts.jsonl"
jq -e '.meta.warnings | index("legacy_absolute_path_exposure") != null' "$TMP/committed.json" >/dev/null

MIGRATION="$TMP/migration"
git init -q "$MIGRATION"
(
  cd "$MIGRATION"
  "$PC" --profile committed add "legacy source" --agent runbook
) >"$TMP/migration-legacy.json"
set +e
(
  cd "$MIGRATION"
  "$PC" --profile private add "blocked before copy" --agent runbook
) >"$TMP/migration.stdout" 2>"$TMP/migration.stderr"
migration_exit=$?
set -e
test "$migration_exit" = 78
jq -e '.error.code == "migration_required"' "$TMP/migration.stderr" >/dev/null
MIGRATION_PRIVATE="$MIGRATION/.git/papercuts/log.jsonl"
install -d -m 700 "$(dirname "$MIGRATION_PRIVATE")"
install -m 600 "$MIGRATION/.papercuts.jsonl" "$MIGRATION_PRIVATE"
cmp -s "$MIGRATION/.papercuts.jsonl" "$MIGRATION_PRIVATE"
(
  cd "$MIGRATION"
  "$PC" --profile private doctor
  "$PC" --profile private list --status all
  "$PC" --profile committed list --status all
) >"$TMP/migration-readback.jsonl"
cmp -s "$MIGRATION/.papercuts.jsonl" "$MIGRATION_PRIVATE"

printf '%s\n' 'single-project runbook verification: pass'
