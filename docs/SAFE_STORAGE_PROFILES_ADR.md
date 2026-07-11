# Safe Storage Profiles and Read-only Task Semantics

Bead: `br-hardened-papercuts-fork-x30.2`

Decision date: 2026-07-11

Status: accepted input to the consolidated hardened-contract ADR; not yet
implemented

## 1. Decision

The hardened fork will have two storage profiles and one independent write
guard:

- `private` is the default profile;
- `committed` is the explicit upstream-compatible legacy profile;
- `--read-only` / `PAPERCUTS_READ_ONLY` is a monotonic guard that can disable
  appends without changing which journal read-only commands inspect.

Inside a validated Git working tree, `private` stores one journal for the whole
Git project under the repository's common Git directory:

```text
GIT_COMMON_DIR/papercuts/log.jsonl
```

This path is outside every worktree and is not eligible for a normal Git commit.
Linked worktrees share the journal. A submodule gets its own common Git
directory and therefore its own journal.

Outside a validated Git working tree, `private` does not invent a global or
path-derived project identity. A mutating command requires an explicit
`--file` or `PAPERCUTS_FILE`; otherwise it returns `storage_required` with exit
78. This prevents unrelated non-Git directories from being folded into one
implicit journal and avoids choosing the project-key algorithm assigned to the
separate path/identity decision.

`committed` preserves the upstream v0.1 storage locations and missing-file
semantics for valid repositories:

```text
validated repository root/.papercuts.jsonl
otherwise HOME/.papercuts/log.jsonl
```

No journal is automatically moved, copied, merged, deleted, gitignored, or
rewritten during upgrade. Existing committed journals trigger an explicit
migration decision before the first private append.

## 2. Why this decision is required

The upstream behavior is coherent but unsafe as a broad default:

- `src/store.rs:37-77` selects `.papercuts.jsonl` in the nearest directory with
  a `.git` marker, then falls back to `$HOME/.papercuts/log.jsonl`;
- `src/commands/add.rs:73-90` creates missing parent directories and the file on
  an actual add;
- `README.md:64-74` describes committed repository-local storage as the
  default and private storage as opt-out;
- `tests/cli.rs:632-728` pins flag, environment, repository, and home precedence
  plus no-create dry-run behavior;
- `tests/cli.rs:860-888` pins explicit missing-file and relative-path behavior;
- `tests/cli.rs:907-953` treats a gitignored repository journal as unhealthy.

That default makes every recorded complaint eligible for accidental commit and
push. A warning does not change the destination and does not protect an agent
that follows a proactive logging instruction during unrelated work.

The fork also cannot infer that a conversation was declared read-only. Command
line arguments do not contain the human authorization boundary. The CLI can
provide a mechanical no-write guard, but the repository or harness instruction
must decide when to activate it and when not to call a mutation at all.

## 3. Preserved invariants

This decision changes discovery defaults, not the journal mechanism. The
following upstream invariants remain mandatory:

- append-only cut and resolve events;
- no edit, delete, compaction, or implicit migration;
- one exclusive-lock read-fold-decide-append critical section per mutation;
- shared locks for reads;
- deterministic output for fixed inputs and clock;
- dry-run creates no file or directory;
- missing discovered state is virtual empty for `list` and `doctor`;
- missing explicit state is exit 66;
- stdout carries success data only and stderr carries structured errors only;
- local filesystems only; no server, daemon, telemetry, or network call.

The private directory and file are implementation details of discovery. They do
not become a second database, an index, or a synchronization layer. “Private”
means absent from normal worktree commits and protected by local permissions;
it does not mean encrypted or unreadable to the local user, administrator,
backup system, or another process with equivalent authority.

## 4. Public CLI and environment surface

Add two global flags:

```text
--profile private|committed
--read-only
```

Add two environment variables:

```text
PAPERCUTS_PROFILE=private|committed
PAPERCUTS_READ_ONLY=0|1|false|true
```

Values are ASCII case-insensitive. Empty values are treated as unset, matching
the existing empty-environment convention. Any other non-empty value is a
`config_error` with exit 78. There is no `--write`, `--unsafe`, or command-line
flag that negates an environment-enforced read-only guard.

No repository config file is added in the first hardened release. A tracked or
untrusted repository must not be able to silently select `committed` storage or
turn off a caller's write guard.

### 4.1 Effective storage precedence

Storage target resolution uses two axes. Explicit target selection is more
specific than profile selection:

1. non-empty `--file PATH` selects `custom` storage;
2. otherwise non-empty `PAPERCUTS_FILE` selects `custom` storage;
3. otherwise `--profile` selects `private` or `committed`;
4. otherwise non-empty `PAPERCUTS_PROFILE` selects the profile;
5. otherwise the profile is `private`.

If `--file` or `PAPERCUTS_FILE` selects `custom`, a simultaneous profile value
does not change the path. The success or error metadata includes a sanitized
warning that the profile was superseded by an explicit target. This preserves
upstream's `--file` over `PAPERCUTS_FILE` rule and never silently ignores a
more specific path.

Relative explicit paths continue to resolve lexically against the current
working directory. The path/identity ADR may strengthen validation and output
redaction, but it must not invert this precedence.

### 4.2 Effective write guard

The write guard is independent of storage:

```text
read_only = CLI --read-only OR PAPERCUTS_READ_ONLY=true
```

Truthy environment values are `1` and `true`; false values are `0` and `false`.
The CLI flag only adds a restriction. It cannot clear an environment
restriction.

This is defense in depth, not a sandbox. A local process with the user's full
authority can invoke another binary or construct a different environment. The
agent instruction remains the primary authorization rule.

The implementation order is deliberately explicit:

```text
parse CLI and environment
validate profile and read-only values
if actual append and read-only is true: return writes_disabled
if --file is set: resolve custom flag target
else if PAPERCUTS_FILE is set: resolve custom environment target
else resolve selected profile
if private and legacy-only state: apply migration state machine
run command-specific input validation and I/O
```

This policy result should be one typed value shared by every command, not a set
of independent environment reads in command modules.

## 5. Profile target resolution

### 5.1 Private profile in a Git project

The repository resolver must return a validated working-tree root and its
absolute common Git directory. The storage target is:

```text
common Git directory + papercuts/log.jsonl
```

The implementation must not assume `.git` is a directory. It must support:

- ordinary repositories where `.git` is a directory;
- linked worktrees where `.git` points at a per-worktree admin directory and a
  `commondir` file identifies the shared common directory;
- submodules where `.git` points into the superproject's modules directory.

A disposable live probe on 2026-07-11 confirmed these expected relationships:

```text
ordinary main common dir: MAIN/.git
linked worktree git dir: MAIN/.git/worktrees/linked
linked worktree common dir: MAIN/.git
submodule common dir: SUPER/.git/modules/module
```

The main checkout and linked worktree therefore resolved the same private
journal, while the submodule resolved a distinct journal.

The implementation should parse and validate repository metadata through the
repository/path seam approved by the path ADR. It must not introduce a
mandatory ambient Git subprocess merely to append a cut. A Git command may be
used in operator migration instructions and disposable acceptance tests.

If a repository marker is present but invalid, do not silently fall back to a
home or committed path. Return a configuration error defined by the path ADR.
Storage remains supported only on local filesystems. The implementation need
not claim reliable cross-platform filesystem-type detection; operator docs must
exclude network mounts, and any explicitly detected unsupported filesystem
must fail without falling back to a different journal.

A bare Git repository is not a working tree. Private implicit mutation there is
treated like non-Git mutation and requires an explicit file. A shared
multi-user common Git directory is not a supported private default because
0600/0700 state created by one user may block another. Use an explicitly chosen
per-user file for that topology.

Private-profile doctor validates the selected common directory, journal format,
and private permissions. It does not run the committed-profile “gitignored
journal” check against a path outside the worktree.

### 5.2 Private profile outside Git

There is no implicit journal. The behavior is:

| Command | Result without explicit file |
|---|---|
| `schema` | success; no discovery |
| `list` | exit 0, virtual empty, warning `storage_required_for_writes` |
| `doctor` | exit 0, healthy virtual empty, same warning |
| `add --dry-run` | exit 78 `storage_required`; no input persistence or filesystem creation |
| `add` | exit 78 `storage_required`; no stdin read and no filesystem creation |
| `resolve --dry-run` | exit 78 `storage_required` |
| `resolve` | exit 78 `storage_required` |

The user can opt into an exact file with `--file` or `PAPERCUTS_FILE`, or opt
into upstream behavior with `--profile committed`.

This is intentionally stricter than the upstream home fallback. A single
implicit global file would mix unrelated directories, and the current content
ID does not include project identity. The path/project-key decision may later
add an allowlisted non-Git identity, but absence of that feature is not a reason
to guess now.

### 5.3 Committed profile

`committed` is the legacy storage compatibility profile, not the safe default:

1. use the validated repository root plus `.papercuts.jsonl`;
2. outside Git, use `$HOME/.papercuts/log.jsonl`;
3. if HOME is absent or empty outside Git, return `config_error`, exit 78;
4. actual add creates missing parents and the journal;
5. list and doctor on a missing discovered target return virtual empty;
6. a gitignored repository journal remains a doctor finding because committed
   mode promises diff visibility.

Repository validation is shared with the private profile. Therefore malformed
or dangling `.git` markers that upstream v0.1 accepted are not promised legacy
compatibility. That deliberate divergence is versioned by the hardened
contract and specified by the path ADR.

The help and schema descriptions must call this profile “repository-visible,
upstream-compatible storage,” not “safe.”

### 5.4 Custom target

An explicit file preserves upstream behavior:

- missing `list`, `doctor`, or `resolve` target is `not_found`, exit 66;
- `add --dry-run` does not create it;
- actual `add` may create its parents and the file;
- relative paths resolve against cwd;
- an explicit path inside the working tree may be tracked or published, so
  private safety is not claimed;
- doctor continues to report visibility findings where applicable.

The effective profile is reported as `custom`, with source `flag-file` or
`env-file`.

## 6. Read-only task behavior

### 6.1 Mechanical behavior

When the write guard is active:

- `schema`, `list`, and `doctor` behave normally and never create state;
- `add --dry-run` and `resolve --dry-run` are allowed because their existing
  contract is no-write; metadata states `write_policy: read-only` and includes
  a preview-only warning;
- actual `add` and actual `resolve` fail with `writes_disabled`, exit 78;
- the refusal happens before reading add text from stdin, resolving the clock,
  creating directories, opening a journal, acquiring a lock, or probing a
  legacy journal;
- the error's suggested action does not tell the agent how to bypass the guard.

The `writes_disabled` suggested action is:

```text
Do not append during this task. Run a read-only command or ask the operator to
authorize a separate writable step.
```

The dispatcher should apply this gate before command-specific mutation logic.
The mutation annotations in `schema` remain truthful: `add` and `resolve`
append unless `--dry-run`, and the write guard can deny their append form.

### 6.2 Agent instruction behavior

The canonical agent rule must say:

- if the current task is explicitly read-only, audit-only, no-write, or limited
  to inspection, do not invoke `add` or `resolve`;
- do not reinterpret a proactive papercut instruction as permission to modify a
  repository, Git metadata, home state, or an external system;
- if a papercut is worth retaining, mention it in the task handoff or ask for a
  separate writable step;
- when the harness can enforce environment policy, set
  `PAPERCUTS_READ_ONLY=1` before running agent commands;
- continue the main task after a normal writable papercut append; do not turn a
  minor cut into an unrequested debugging project.

The CLI cannot inspect the conversation, AGENTS hierarchy, Codex mode, or user
intent. Documentation must not imply that it can.

## 7. First-run and existing-journal state machine

The private resolver considers the private target and the matching legacy
target without reading or combining their contents. The table applies inside a
validated Git project; non-Git behavior is defined in section 5.2.

| Private target | Legacy target | Default private behavior |
|---|---|---|
| absent | absent | virtual empty for list/doctor; actual add creates private target |
| present | absent | read/append private target |
| absent | present | reads report legacy-detected warning; actual mutation refuses with `migration_required` |
| present | present | use private target only; warn that legacy journal is retained and not merged |

`migration_required` is exit 78. It prevents a silent split where an upgraded
installation starts writing a new private history while the user believes the
old committed history is still active.

Error ordering for a mutating command is:

1. CLI and environment value validation;
2. monotonic read-only write refusal;
3. storage target and migration-state resolution;
4. command input validation;
5. permission, lock, and I/O operations.

This ordering means a read-only refusal does not probe legacy state and an
invalid profile does not consume stdin.

In the legacy-only state:

- `list` and `doctor` return private virtual-empty data and explicitly state
  that legacy items were not included;
- `add --dry-run` may return a no-write private preview but warns that the real
  append will require migration or committed mode;
- `resolve --dry-run`, actual `add`, and actual `resolve` return
  `migration_required`, because resolving an item requires selecting the
  journal that owns its history.

Warnings must not contain raw private paths once the path ADR takes effect.
They may state that a legacy or second journal exists and identify the profile
needed to inspect it.

## 8. Migration from committed to private

Migration is explicit copy-and-verify. It is never performed by normal
discovery or `add`.

### 8.1 Preconditions

- stop all papercuts writers for the project;
- confirm the source journal with committed-profile doctor;
- resolve the validated common Git directory;
- confirm the private target does not exist;
- verify both locations are on supported local filesystems;
- keep the source unchanged as rollback evidence.

Representative operator procedure for a normal Unix Git checkout:

```bash
LEGACY_FILE="$(git rev-parse --show-toplevel)/.papercuts.jsonl"
COMMON_DIR="$(git rev-parse --path-format=absolute --git-common-dir)"
PRIVATE_DIR="$COMMON_DIR/papercuts"
PRIVATE_FILE="$PRIVATE_DIR/log.jsonl"

papercuts --profile committed doctor
test -f "$LEGACY_FILE"
test ! -e "$PRIVATE_FILE"
umask 077
mkdir -p "$PRIVATE_DIR"
cp -n "$LEGACY_FILE" "$PRIVATE_FILE"
chmod 600 "$PRIVATE_FILE"
cmp --silent "$LEGACY_FILE" "$PRIVATE_FILE"
papercuts --profile private doctor
```

This copies complete bytes to a new target. It does not rewrite, truncate, or
delete the legacy journal. The final implementation/runbook must test the exact
commands on every supported platform and replace platform-specific steps where
necessary.

Copying to private state does not remove the legacy file from Git, sanitize
existing commits, or retract anything already pushed. Untracking, redacting, or
history repair is a separate explicitly approved repository operation with its
own evidence and rollback.

### 8.2 Rollback

Immediate rollback is selection-only:

```bash
papercuts --profile committed doctor
papercuts --profile committed list --status all
```

Set `PAPERCUTS_PROFILE=committed` in the controlling environment only after the
operator chooses to restore legacy writes. Do not delete the private target.

If both targets received new events, stop writers and preserve both files. Do
not concatenate or merge them automatically. A later reviewed reconciliation
must validate complete JSONL lines, preserve both sources as backups, account
for first-wins duplicate semantics, and run doctor before selecting the
resulting journal.

## 9. Permissions and failure behavior

For newly created private state on Unix:

- create the `papercuts` directory with mode 0700;
- create `log.jsonl` with mode 0600;
- do not loosen existing parent/common-Git-directory permissions;
- if secure permissions cannot be established, fail and leave no appended
  record.

An existing private directory or file with group/other access is not silently
chmodded. Doctor reports an `insecure_private_permissions` finding, and an
actual append refuses until the operator reviews and corrects the permissions.
This avoids silently breaking a deliberately shared setup while preserving the
meaning of the private profile.

On other platforms, use the strongest supported user-only permissions and test
the resulting access contract. Do not claim Unix modes on platforms that do not
provide them.

Committed and custom targets retain ordinary create semantics controlled by
the user's umask and explicit path, because they may intentionally be shared or
tracked.

Failure rules:

- private target permission failure: `permission_denied`, exit 77;
- insecure existing private permissions: `insecure_private_permissions`, exit
  77 for mutation and a doctor finding for inspection;
- missing explicit file for read/resolve: `not_found`, exit 66;
- missing HOME in committed non-Git mode: `config_error`, exit 78;
- private non-Git implicit mutation: `storage_required`, exit 78;
- legacy-only upgrade state: `migration_required`, exit 78;
- read-only actual mutation: `writes_disabled`, exit 78;
- lock exhaustion: existing retryable `lock_timeout`, exit 75;
- never fall back to committed, HOME, or another journal after a permission,
  validation, or lock failure.

The path ADR controls redaction of path-bearing I/O details. The storage error
code and exit meaning remain stable regardless of redaction.

## 10. Schema and metadata contract

`schema` must publish:

- both profiles and the `private` default;
- exact target precedence;
- `PAPERCUTS_PROFILE` and `PAPERCUTS_READ_ONLY` accepted values;
- read-only deny-wins semantics;
- per-command read-only, append, dry-run, and create behavior;
- private Git-common-directory behavior and non-Git explicit-file requirement;
- legacy detection/migration state machine;
- the new stable error codes and exits;
- local-filesystem and no-network constraints.

Success metadata for commands that resolve storage gains these fields:

```json
{
  "storage_profile": "private|committed|custom",
  "storage_source": "default|flag-profile|env-profile|flag-file|env-file",
  "write_policy": "normal|read-only"
}
```

Errors produced after policy parsing carry the same effective profile/source/
write-policy metadata where doing so does not reveal a sensitive path. The
exact fate of `meta.file` is decided by the path ADR.

Changing the default destination and adding policy errors is observable
behavior. The consolidated hardened ADR must bump the machine contract to at
least 2. No hardened release may claim unchanged contract 1 while shipping this
default.

Existing contract-1 journal lines remain readable. This decision changes where
new lines go, not how old lines are folded.

## 11. Alternatives considered

### 11.1 Keep committed default and add warnings

Rejected. A warning is emitted after choosing the dangerous destination and
does not stop later `git add` or push. It also trains agents to accept a known
leak-prone default.

### 11.2 Require a profile in every repository

Rejected as the universal default. It makes the safe path fail on every clean
Git checkout, encourages agents to select any value merely to proceed, and
turns the logging tool itself into recurring friction. Explicit selection is
still required outside validated Git.

### 11.3 Default to one global home journal

Rejected. It mixes unrelated projects, makes list/resolve context ambiguous,
and permits content-ID collision across directories because repository identity
is not currently part of the ID.

### 11.4 Put private state in the working tree and edit `.gitignore`

Rejected. The first add would mutate worktree-visible state or repository
ignore configuration, and an untracked journal can still be added accidentally.

### 11.5 Put private state in user state keyed by an absolute-path hash

Deferred, not selected. Hashing a private path is not automatically anonymous,
renames and clones change identity, and the stable project-key contract belongs
to the path/identity ADR. The Git common directory gives an existing project
scope without adding a reversible fingerprint to public data.

### 11.6 Auto-migrate or read both journals as one

Rejected. Automatic copy/move can lose data or change permissions. A union read
creates cross-file first-wins and resolution semantics that upstream never
defined. The safe transition is explicit copy, byte verification, and one
selected target.

### 11.7 Treat read-only mode as a storage profile

Rejected. Storage location and write authority are independent. Combining them
would make read commands inspect a different journal when a task becomes
read-only and would complicate precedence. A monotonic guard is simpler and
honest.

## 12. Implementation boundaries

This Bead authorizes documentation and downstream task context only. Product
implementation remains blocked on the consolidated ADR.

Expected ownership:

- storage/profile resolution: `br-hardened-papercuts-fork-x30.7`;
- path-safe repository/common-dir resolver and metadata:
  `br-hardened-papercuts-fork-x30.8` after the path ADR;
- schema, metadata, and errors: `br-hardened-papercuts-fork-x30.10`;
- adversarial/e2e matrix: `br-hardened-papercuts-fork-x30.11`;
- agent instructions and operator migration runbook:
  `br-hardened-papercuts-fork-x30.12`.

Do not modify fold, event serialization, content IDs, or locking merely to add
profiles. Repository validation must be one reusable seam, not duplicated in
storage and path modules.

## 13. Required unit and black-box matrix

### Resolution and precedence

- default profile is private;
- flag profile beats environment profile;
- flag file beats environment file and all profiles;
- environment file beats profile selection;
- empty environment values are unset;
- invalid profile/read-only values are exit 78;
- metadata reports effective profile, source, and write policy;
- profile superseded by explicit file produces a sanitized warning.

### First run and side effects

- private actual add in normal Git creates only common-dir state;
- `git status --porcelain` remains byte-identical before and after private add;
- linked worktrees resolve the same private journal;
- submodule resolves a distinct private journal;
- private non-Git mutation requires explicit storage;
- committed Git add uses the upstream `.papercuts.jsonl` location and append
  behavior in a valid repository;
- committed non-Git add uses HOME fallback;
- missing HOME in committed non-Git mode is exit 78;
- list, doctor, and all dry runs create no file or directory;
- explicit missing list/doctor/resolve remains exit 66.

### Read-only guard

- flag alone denies actual add and resolve;
- environment alone denies them;
- false flag does not exist and cannot negate true environment state;
- dry-run add/resolve remain no-write previews;
- actual add refusal occurs before stdin read and before filesystem probes;
- schema/list/doctor remain available and create nothing;
- suggested action does not teach an agent to bypass the guard.

### Existing journals and migration

- legacy only plus default list/doctor warns without reading it as private;
- legacy only plus actual default mutation is `migration_required`;
- explicit committed profile continues legacy writes;
- copied byte-identical private journal passes doctor;
- both journals present selects private only and warns;
- neither journal is rewritten, removed, or auto-merged;
- rollback selection reads the untouched legacy source.

### Failure and security honesty

- private permission failures never fall back to committed or HOME;
- invalid Git marker never becomes a home or worktree journal silently;
- an explicitly detected unsupported filesystem never falls back, and docs do
  not claim automatic detection where none exists;
- private permission findings and append refusals are deterministic;
- no private-mode operation emits an unsupported “cannot leak” guarantee;
- path-bearing diagnostics satisfy the path ADR;
- existing contract-1 journals remain readable;
- the full upstream suite remains green.

## 14. Review passes

Three local review passes were performed without changing product code:

1. **Source-contract pass:** checked the decision against `store.rs`, every
   command dispatcher, error/meta/schema behavior, the upstream design, and
   discovery/dry-run/permission tests.
2. **Adversarial operations pass:** challenged legacy-only upgrade, dual-log
   divergence, invalid Git metadata, bare/shared/worktree/submodule topology,
   insecure permissions, unsupported filesystems, missing HOME, and read-only
   refusal ordering.
3. **Implementation-readiness pass:** reduced the result to one typed policy
   resolution seam, stable error/metadata fields, explicit downstream ownership,
   rollback, and a black-box acceptance matrix.

The final pass found no remaining contradiction inside the storage slice. The
listed path, secret, and consolidated-contract gates remain deliberate external
dependencies rather than unspecified behavior.

## 15. Review result and remaining gates

This decision resolves project-plan questions about the default storage mode,
private location, compatibility mode, precedence, read-only semantics, first
run, and migration rollback.

It deliberately leaves these items to their assigned decisions:

- exact validation of Git markers and common-directory paths;
- record `cwd`/`repo` representation and `meta.file` redaction;
- stable project identity for later allowlisted inventory;
- sensitive-content detection and override policy;
- exact consolidated contract number above the minimum of 2.

Those are not loopholes in the selected storage default. Implementation cannot
start until the consolidated hardened-contract ADR reconciles them and copies
the final combined contract into downstream Beads.
