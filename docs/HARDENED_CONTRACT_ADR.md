# Hardened Contract v2

Bead: `br-hardened-papercuts-fork-x30.5`

Decision date: 2026-07-12

Status: accepted architecture gate for phase-2 implementation; not yet
implemented

Applies to: the first single-project hardened prerelease of
`39elbarto/papercuts`

## 1. Authority and decision

This document is the normative contract for the first hardened implementation.
It consolidates and, where necessary, resolves interactions among:

- the upstream v0.1 compatibility audit;
- safe storage and read-only semantics;
- path minimization and project identity;
- deterministic sensitive-data guardrails;
- upstream synchronization, naming, and release policy.

The source ADRs remain design evidence. If wording differs, this consolidated
contract controls phase-2 implementation and tests.

The fork adopts machine contract **2**. Contract 2 deliberately changes the
default storage destination, repository validation, path projection, accepted
input, record fields, metadata, and error vocabulary. It preserves the
append-only journal, fold semantics, content-ID algorithm, locking discipline,
deterministic envelopes, and legacy record readability.

This ADR authorizes only the dependency-ordered implementation Beads. It does
not claim that the current binary already behaves this way.

## 2. Resolution of the eight planning decisions

| # | Planning question | Contract-2 resolution |
|---:|---|---|
| 1 | Safe/private default or explicit profile? | `private` is the global default; `committed` is explicit. |
| 2 | Warn or refuse likely-sensitive input? | Both: private defaults to `balanced`; committed defaults to `strict`. |
| 3 | Which deterministic checks are honest? | A versioned bounded local catalog; no entropy, decoding, network, or completeness claim. |
| 4 | Omit, relativize, hash, or flag paths? | Private automatic paths are omitted with compatible sentinels; committed retains labeled legacy absolute paths. |
| 5 | Where do private logs live? | Valid Git projects use `GIT_COMMON_DIR/papercuts/log.jsonl`; private non-Git mutation requires an explicit file. |
| 6 | Retain package and binary names? | Retain only through development and exact-SHA isolated pilot; rename before non-isolated distribution while fork-only behavior remains. |
| 7 | Upstream versus fork-only changes? | Generic contract-1 fixes are isolated upstream candidates; changed defaults, paths, validation, content policy, and ACFS adapters are fork-only unless accepted upstream. |
| 8 | Smallest cross-project interface? | Deferred behind pilot evidence to `x30.17`; its boundary is an explicit private allowlist, operator aliases, bounded read-only digest, and no directory scan or source mutation. |

Decision 8 does not block the single-project contract. No multi-project command,
registry, locator, or automatic project identity is added in contract 2.

## 3. Goals, non-goals, and security claim

### 3.1 Goals

Contract 2 must:

- avoid normal worktree publication for a first-time user;
- respect an explicit mechanical no-write guard;
- omit automatically captured path identity in the private profile;
- refuse common high-confidence credential shapes before append-side I/O;
- make every policy decision machine-readable and deterministic;
- retain upstream journals without rewriting them;
- keep normal operation local, offline, non-interactive, and serverless;
- preserve a reviewed route to inspect or use legacy repository-visible state.

### 3.2 Non-goals

Contract 2 does not provide:

- encryption, a secret vault, DLP, sandboxing, or complete secret detection;
- protection from a malicious process with the same local authority;
- a hosted service, daemon, telemetry, sync server, or central database;
- automatic Git-history redaction, journal merging, migration, or deletion;
- recursive repository discovery or multi-project aggregation;
- automatic issue creation, fixing, resolution, or promotion;
- reliable operation on network filesystems;
- an unchecked compatibility switch inside the hardened binary.

### 3.3 Exact security claim

The product may claim:

> In the private profile, Papercuts stores implicit Git-project state outside
> normal worktrees, omits automatic path metadata, and checks bounded
> caller-controlled fields for a versioned set of common sensitive-data shapes
> before append. Refused values are not written or echoed by Papercuts.

It must also state:

> Private does not mean encrypted. Accepted text can contain sensitive data.
> Detection is incomplete. Balanced warnings and deliberate overrides persist
> the original input. Legacy journal bytes can retain paths and unscanned text.

The product must not claim that it cannot leak paths or secrets, that clean
scanner output proves safety, or that the CLI understands conversational write
authority.

## 4. Preserved upstream invariants

The following v0.1 properties remain normative:

- source storage is append-only JSONL containing `cut` and `resolve` events;
- normal commands never edit, delete, compact, reorder, or rewrite events;
- first cut for an ID wins; first resolve for an ID wins;
- resolve-before-cut is foldable after the complete scan;
- malformed, unknown, torn-tail, duplicate, conflict, and orphan behavior stays
  compatible unless contract 2 explicitly adds a finding;
- ordering remains severity descending, timestamp descending, then ID;
- tags remain sorted for records and ID computation;
- the cut ID remains the first six SHA-256 bytes over the existing
  length-prefixed sequence `ts`, `agent`, `text`, `severity`, sorted tags joined
  by commas;
- profile, path, project, and `content_policy` fields are not ID inputs;
- writes use one exclusive read-fold-decide-append critical section;
- reads use a shared lock;
- lock exhaustion remains retryable exit 75;
- torn-tail healing and failed-append truncation remain intact;
- durability remains best effort without a per-append fsync promise;
- JSON success is stdout-only and JSON errors are stderr-only;
- Markdown list, help, and version remain the documented plaintext exceptions;
- fixed inputs and a fixed clock produce deterministic bytes;
- local operation has no mandatory Git executable, network, server, or
  telemetry dependency.

Implementation must not refactor fold or locking merely to add policy. Any
necessary change to those surfaces requires its own evidence and repeated
concurrency tests.

## 5. User workflows

### 5.1 Normal private add in a Git project

```bash
papercuts add "workspace tests only pass from apps/web" --tag tooling
```

The command:

1. selects the private profile by default;
2. validates the nearest Git repository and common Git directory;
3. selects `GIT_COMMON_DIR/papercuts/log.jsonl`;
4. validates and scans caller-controlled fields under balanced policy;
5. creates private state with user-only permissions if needed;
6. appends a path-omitted contract-2 cut;
7. leaves `git status --porcelain` unchanged.

### 5.2 Explicit strict private add

```bash
papercuts --sensitive-policy strict add \
  "workspace tests only pass from apps/web" --tag tooling
```

This retains private storage and path omission but raises the content policy to
strict. Policy cannot be lowered below the active profile's floor.

### 5.3 Explicit committed compatibility lane

```bash
papercuts --profile committed add \
  "workspace tests only pass from apps/web" --tag tooling
```

This selects repository-visible v0.1 storage and legacy absolute path capture,
but still uses strict content policy, contract-2 records, strict Git validation,
and contract-2 input limits. `committed` is not an unchecked v0.1 mode.

### 5.4 Explicit file without profile change

```bash
papercuts --file ./private-review/cuts.jsonl add \
  "workspace tests only pass from apps/web" --tag tooling
```

`--file` changes only the target. The default profile remains private, so path
projection and safe diagnostics remain private. The target can still be inside
a worktree or otherwise publishable; an explicit target receives no implicit
privacy guarantee.

### 5.5 Read-only task

```bash
PAPERCUTS_READ_ONLY=1 papercuts list --status all
```

Read commands remain available. Actual `add` and `resolve` return
`writes_disabled` before stdin, clock, repository, migration, or journal probes.
Dry-run forms remain allowed because they create no state, subject to their
normal target and input requirements.

Repository instructions remain stricter than the CLI mechanism: during an
explicitly read-only, audit-only, inspection-only, or no-write task, an agent
must not invoke `add` or `resolve`, including dry-run, unless the task explicitly
asks for that preview.

### 5.6 Deliberate category override

An operator-authorized example using a non-routable sample domain is:

```bash
PAPERCUTS_ALLOW_SENSITIVE=1 papercuts --sensitive-policy strict add \
  "contact operator@example.invalid" \
  --allow-sensitive email_address
```

Both controls are required. The override is accepted only when every refusing
category is named exactly and no unused category is pre-authorized. The record
stores category and field names, not matched fragments. Canonical agent policy
forbids autonomous use of either control.

### 5.7 Review and resolve

```bash
papercuts doctor
papercuts list --status open --format md
papercuts resolve pc_94f5df71022d --note "fixed in br-example"
```

`resolve` appends an event. It never edits the cut. The note and persisted agent
name receive the same bounded content preflight as add fields.

## 6. Public CLI and environment surface

### 6.1 Global flags

Contract 2 exposes:

```text
--file PATH
--pretty
--profile private|committed
--read-only
--sensitive-policy balanced|strict
```

`--file` and `--pretty` retain their v0.1 meanings subject to profile-safe
projection. The new policy flags are global.

### 6.2 Mutation flags

`add` retains text/stdin, agent, repeated tag, severity, and dry-run flags and
adds:

```text
--allow-sensitive CATEGORY
```

`resolve` retains ID, note, agent, and dry-run flags and adds the same repeated
category flag. The flag is not accepted on list, doctor, or schema.

### 6.3 Environment variables

Existing:

```text
PAPERCUTS_FILE
PAPERCUTS_AGENT
PAPERCUTS_NOW
```

New:

```text
PAPERCUTS_PROFILE=private|committed
PAPERCUTS_READ_ONLY=0|1|false|true
PAPERCUTS_SENSITIVE_POLICY=balanced|strict
PAPERCUTS_ALLOW_SENSITIVE=0|1|false|true
```

Enumerated and Boolean values are ASCII case-insensitive. Empty values are
unset. Other non-empty values are `config_error`, exit 78. Relevant textual
values (`PAPERCUTS_PROFILE`, `PAPERCUTS_READ_ONLY`,
`PAPERCUTS_SENSITIVE_POLICY`, `PAPERCUTS_ALLOW_SENSITIVE`,
`PAPERCUTS_AGENT`, and `PAPERCUTS_NOW`) must be valid UTF-8 and never silently
fall through.

Path-valued `PAPERCUTS_FILE` and `HOME` retain native OS encoding. On Unix they
must not be forced through UTF-8 merely to resolve a target. Private output
never serializes them; committed output follows the explicit lossy-encoding
contract where a path must be shown.

There is no repository config file, `--write`, `--unsafe`, `--force`, scanner
off switch, or wildcard override.

### 6.4 Command-relevant environment reads

Environment is read only when it can affect the command:

- `schema` is static self-orientation and does not inspect storage, profile,
  clock, agent, read-only, or override environment;
- `list` and `doctor` resolve profile, file, and read-only policy for target,
  metadata, and path projection, but ignore agent, sensitive-policy, and
  override controls;
- `add` and `resolve` resolve all applicable storage, write, content, agent, and
  clock inputs;
- `list` reads `PAPERCUTS_NOW` only for a relative `--since` value;
- an absolute `--since`, list without `--since`, doctor, and schema do not read
  the clock.

This intentionally replaces v0.1's global eager clock resolution. Contract-2
tests pin the new relevance and error ordering.

## 7. Unified policy resolution

One typed policy context is resolved centrally and passed through command,
storage, record, projection, metadata, and error code. Command modules must not
re-read environment variables or scatter profile checks.

### 7.1 Profile precedence

1. `--profile`;
2. non-empty `PAPERCUTS_PROFILE`;
3. `private` default.

Metadata source is `flag-profile`, `env-profile`, or `default`.

### 7.2 Target precedence

1. non-empty `--file` -> `flag-file`;
2. non-empty `PAPERCUTS_FILE` -> `env-file`;
3. profile default -> `profile-default`.

An explicit file never changes profile, path policy, content-policy floor, or
diagnostic projection. Relative explicit paths normalize lexically against cwd.

### 7.3 Monotonic write guard

```text
read_only = CLI --read-only OR PAPERCUTS_READ_ONLY is true
```

The CLI flag can add but never remove the environment restriction. There is no
negative flag. `write_policy` metadata is `normal` or `read-only`.

### 7.4 Sensitive-policy precedence and floor

Profile floors:

| Profile | Floor |
|---|---|
| private | balanced |
| committed | strict |

Requested policy precedence:

1. `--sensitive-policy`;
2. non-empty `PAPERCUTS_SENSITIVE_POLICY`;
3. profile floor.

A weaker-than-floor request is `config_error`, exit 78. There is no `off`.
Metadata source is `flag`, `env`, or `profile-default`.

### 7.5 Override resolution

Override acceptance requires both a truthy `PAPERCUTS_ALLOW_SENSITIVE` and one
repeated `--allow-sensitive` for every refusing category. The environment gate
alone is inert. Category flags without the gate are `config_error`, exit 78.
Unknown syntax/category is invalid argument, exit 2. Partial coverage and unused
pre-authorization are invalid input, exit 65. Duplicate exact categories are
deduplicated. `all`, `*`, comma lists, negation, and `off` are invalid.

## 8. Deterministic evaluation and side-effect order

### 8.1 Static schema

`schema` parses CLI syntax, writes the static contract, and performs no policy,
clock, repository, storage, or journal discovery.

### 8.2 Actual mutation

For actual `add` and `resolve`:

1. parse CLI;
2. validate command-relevant non-clock environment/configuration and override
   syntax;
3. if the monotonic guard is active, return `writes_disabled`;
4. validate non-consuming semantic arguments such as a resolve ID prefix;
5. resolve logical profile/target and validate Git metadata without creating
   state;
6. resolve default-profile migration state; return `storage_required` or
   `migration_required` where applicable without consuming stdin;
7. read stdin with a bound where applicable;
8. validate UTF-8, non-empty rules, counts, per-field sizes, and total size;
9. scan caller-controlled persisted fields and decide clean/warn/refuse/
   override;
10. on refusal, return `sensitive_input` without creating/opening a journal;
11. lazily resolve the clock and construct the candidate event/ID;
12. create/open/lock/read-fold-decide-append through the existing store seam;
13. project output according to profile and serialize exactly once.

No refused content reaches clock-dependent IDs, duplicate lookup, journal I/O,
debug logs, or error formatting. A strict retry cannot retrieve an identical
legacy suspect record through duplicate lookup because scanning occurs first.

### 8.3 Dry run

Dry run follows the same path through input validation, scanner, clock, record
construction, and output projection, but performs no directory creation,
journal open, lock, read, or append for add.

Exceptions required by target semantics:

- private non-Git without explicit storage returns `storage_required` even for
  dry-run add;
- legacy-only default private add dry-run returns a preview plus a sanitized
  warning that real append requires migration or committed profile;
- resolve dry-run must identify an existing item and therefore uses shared
  read-only journal access;
- missing explicit resolve target remains `not_found`;
- legacy-only default private resolve dry-run is `migration_required`.

Dry-run refusal is the same `sensitive_input`. Accepted warning/override preview
returns the original proposed record; it is not redaction.

### 8.4 Read commands

List and doctor resolve the selected target without creating it, use shared
access when it exists, apply profile projection, and serialize sanitized
diagnostics. Missing discovered state is virtual empty. Missing explicit state
is `not_found`, exit 66.

## 9. Storage contract

### 9.1 Private profile default

Inside a validated non-bare Git working tree:

```text
GIT_COMMON_DIR/papercuts/log.jsonl
```

Ordinary checkouts and linked worktrees share one journal. A submodule has its
own common Git directory and journal. New implicit private state uses directory
mode 0700 and file mode 0600 on Unix. Other platforms use the strongest tested
user-only access available.

Existing group/other-readable implicit private state is not silently chmodded.
Doctor reports `insecure_private_permissions`; mutation refuses with that code
and exit 77 until the operator reviews permissions.

Private means outside normal worktree commits and user-restricted when the
implicit target is created. It does not mean encrypted or hidden from the local
administrator, backups, or equivalent-authority processes.

### 9.2 Private profile outside Git

Without an explicit file:

| Command | Result |
|---|---|
| schema | static success |
| list | virtual empty plus `storage_required_for_writes` warning |
| doctor | healthy virtual empty plus the same warning |
| add / add dry-run | `storage_required`, exit 78 |
| resolve / resolve dry-run | `storage_required`, exit 78 |

No global home journal is invented for private mode.

### 9.3 Committed profile

Committed target selection preserves v0.1 locations:

1. validated repository root plus `.papercuts.jsonl`;
2. otherwise `$HOME/.papercuts/log.jsonl`;
3. absent or empty HOME outside Git is `config_error`, exit 78.

Missing discovered list/doctor is virtual empty. Actual add may create parents
and the file. A gitignored repository journal remains a doctor finding because
this profile promises repository visibility.

Strict repository validation, strict content-policy floor, input bounds,
contract-2 records, and new errors still apply. Help calls this
“repository-visible, upstream-compatible storage,” never “safe” or “v0.1 mode.”

### 9.4 Explicit target

An explicit target preserves v0.1 target precedence and missing-file behavior:

- add may create missing parents and file;
- add dry-run creates nothing;
- missing list, doctor, or resolve target is `not_found`, exit 66;
- relative paths normalize lexically against cwd.

Explicit targets use ordinary OS/umask creation semantics, even under the
private profile. They can be inside a worktree or shared. Private profile still
controls path omission, safe diagnostics, content floor, and final-file symlink
rejection, but does not claim the selected target is private.

### 9.5 Legacy/private state machine

This state machine applies only to private `profile-default` target resolution
inside a validated Git project:

| Private target | Legacy target | Behavior |
|---|---|---|
| absent | absent | virtual empty; actual add creates private target |
| present | absent | read/append private |
| absent | present | read commands return private virtual empty with legacy-detected warning; real mutation refuses `migration_required` |
| present | present | use private only; warn that legacy is retained and not merged |

Explicit targets do not trigger this implicit migration state machine.

Legacy-only add dry-run is the preview exception described in section 8.3.
Resolve dry-run and real resolve require journal selection and refuse.

### 9.6 Filesystem boundary

Only local filesystems are supported. The product does not promise reliable
filesystem-type detection. Operator docs exclude network mounts. If an
unsupported filesystem is explicitly detected, return `unsupported_filesystem`
without falling back to another target.

Permission, validation, symlink, or lock failure never falls back to committed,
HOME, or another journal.

## 10. Repository and path contract

### 10.1 One strict repository resolver

One platform-native typed resolver supplies physical working-tree root, Git
directory, and common Git directory internally. It does not serialize private
paths and does not require a Git subprocess.

Search the physical existing cwd and ancestors for the nearest `.git` marker.
No marker means non-Git. A malformed nearest marker is `invalid_repository`,
exit 78; never skip it to inherit an outer repository.

Ordinary `.git` directory requirements:

- marker is a real directory, not symlink;
- regular `HEAD` and `config` files exist;
- `objects` is a directory;
- marker is the common directory.

Gitdir-file requirements:

- marker is a regular file, not symlink;
- exactly one `gitdir: PATH` logical line, LF or CRLF;
- target can be absolute or marker-parent-relative;
- reject empty target, NUL, extra non-empty lines, unknown prefix,
  non-directory/unreadable target;
- target Git directory has regular `HEAD`.

Commondir requirements:

- if present, exactly one absolute or Git-directory-relative path using the
  same line/NUL rules;
- otherwise Git directory is common directory;
- common directory exists with `objects` directory and regular `config`;
- do not require `refs`, allowing alternate ref backends.

Bare repositories are non-Git for implicit private storage. They require an
explicit target for private mutation.

### 10.2 Symlink and traversal rules

- canonicalize existing cwd for discovery only;
- direct symlink `.git` marker is invalid;
- canonicalize validated gitdir/commondir targets internally;
- implicit private `papercuts` directory and final `log.jsonl` cannot be
  symlinks;
- private explicit final-file symlink is rejected; explicit parent symlinks are
  allowed without output exposure;
- committed explicit paths retain ordinary OS symlink behavior;
- relative explicit `.` and `..` normalize lexically.

Symlink rejection is an accidental-redirection guard, not protection from a
malicious same-user time-of-check/time-of-use race.

### 10.3 Private path record

Every new private cut uses:

```text
cwd = "."
repo = null
path_policy = "omitted"
path_encoding = "omitted"
```

The cwd sentinel means automatic path context was withheld, not that execution
occurred at repository root.

No cwd, repository, common Git directory, target, remote, basename, username,
home, drive, UNC share, symlink target, path hash, machine ID, or automatic
project key is serialized.

### 10.4 Committed path record

Every new committed cut uses `path_policy: legacy-absolute`. Valid UTF-8
captured paths use `path_encoding: utf8`; any lossy capture makes it
`lossy-utf8` and emits a sanitized warning. Cwd and repo retain v0.1 meanings.

### 10.5 Output projection

Private projection applies to add (including duplicate/dry-run), list, resolve
(including already-resolved/dry-run), doctor, Markdown context, and errors.

Any contract-1 or `legacy-absolute` cut is projected as:

```json
{
  "cwd": ".",
  "repo": null,
  "path_policy": "omitted",
  "path_encoding": "omitted"
}
```

Projection never rewrites the journal. A source contract-1 record has no
`content_policy`; path projection does not invent one or sanitize its text.

Committed projection can expose stored legacy paths and `meta.file`, with an
explicit exposure warning. It never reconstructs paths for omitted records.

### 10.6 Non-UTF-8 behavior

Private filesystem paths remain native `OsStr`/wide-string values and never
round-trip through JSON. Private output contains no replacement character
derived from a path.

Committed mode retains v0.1 lossy conversion and labels it. No path hash is
computed before or after conversion.

## 11. Sensitive-data contract

### 11.1 Scanned persisted fields and bounds

| Field | Maximum |
|---|---:|
| add text | 10,000 UTF-8 bytes; stdin reader stops at 10,001 |
| resolution note | 2,000 UTF-8 bytes |
| each tag | 64 UTF-8 bytes |
| tag count | 16 |
| persisted agent | 128 UTF-8 bytes |
| total scanned command payload | 16,384 bytes |
| compiled policy patterns | 128 |

Text, every tag, explicit/environment agent, and resolution note are scanned.
Automatic path fields, file target, Git metadata, and later project aliases are
not scanner inputs.

The scanner is pure, local, deterministic, versioned `1`, and compiled from
repository-owned static patterns. It has no runtime catalog, caller patterns,
filesystem, network, subprocess, clock, locale, or randomness dependency.

### 11.2 Category matrix

High-confidence categories refuse in both modes:

```text
private_key
authorization_header
credential_url
secret_assignment
github_token
slack_token
stripe_secret_key
aws_credential_pair
```

Medium-risk categories warn in balanced and refuse in strict:

```text
email_address
personal_identifier
filesystem_path
config_block
```

Policy-version-1 matching requirements are:

| Category | Required shape |
|---|---|
| `private_key` | private-key PEM/PGP begin or end marker, including PKCS, RSA, EC, OpenSSH, and PGP forms |
| `authorization_header` | `Authorization` header with non-placeholder Bearer or Basic material |
| `credential_url` | URI authority with non-placeholder user and password before `@` |
| `secret_assignment` | literal assigned to password/passwd/pwd/secret/token/api-key/access-key/client-secret/private-key label |
| `github_token` | bounded body after `ghp_`, `github_pat_`, `gho_`, `ghu_`, `ghs_`, or `ghr_`; no fixed 40-character assumption |
| `slack_token` | bounded body after `xoxb-`, `xoxp-`, `xwfp-`, or `xapp-` |
| `stripe_secret_key` | bounded body after `sk_test_`, `sk_live_`, `rk_test_`, or `rk_live_` |
| `aws_credential_pair` | `AKIA`/`ASIA` access-key ID plus secret-access-key-labelled material in one command |
| `email_address` | conservative ASCII local-part, `@`, and DNS-like domain |
| `personal_identifier` | labelled email, phone, customer, patient, user, or account ID assignment |
| `filesystem_path` | common absolute Unix, `/home/`, `/Users/`, Windows drive, or UNC shape |
| `config_block` | at least two assignment-like lines in one text or resolution note |

After surrounding quotes and ASCII whitespace are removed, assignment and URL
candidate values are exempt only when the complete value ASCII-case-
insensitively equals:

```text
example
placeholder
redacted
[redacted]
xxxxx
changeme
not-a-real-secret
test-token
dummy
your_token_here
```

Pure `$NAME` and `${NAME}` shell references are also exempt from assignment-
value rules. Exemptions never use substring matching.

AWS access-key IDs `AKIA`/`ASIA` alone are not enough; a secret pair is
required. Stripe publishable `pk_` keys are not secret-key findings. Benign
hashes, UUIDs, commit IDs, Bead IDs, and issue IDs remain accepted unless a
contextual rule such as a secret assignment applies.

### 11.3 Decision matrix

| Finding | balanced | strict |
|---|---|---|
| none | clean append | clean append |
| medium only | warn and append | refuse |
| any high | refuse | refuse |
| fully authorized exact override | append with override audit | append with override audit |

Balanced warnings and overrides persist original content and can appear in
normal success output. Diagnostic metadata never repeats a value.

For a new audit object, `clean` has empty category and field arrays; `warn`
contains medium categories only; `override` contains every observed category
and matching field for the command. All arrays are sorted and deduplicated.

### 11.4 Refusal contract

`sensitive_input` is non-retryable exit 65. Its details contain only:

```json
{
  "policy_version": 1,
  "policy": "strict",
  "categories": ["credential_url"],
  "fields": ["text"]
}
```

No refusal, policy diagnostic, debug output, panic, benchmark, or retained
failure artifact contains a match, substring, context, position, line, length,
pattern, encoded form, or hash. Suggested action asks the caller to replace the
value; it does not print an override command.

### 11.5 Honest limitations

Policy v1 does not use generic entropy, recursive decoding, Unicode
normalization, homoglyph detection, unmarked private-key-body recognition, or
complete vendor/PII catalogs. It does not inspect referenced files, clipboard,
terminal history, existing journals, Git history, or other-program output.
Encoded, split, transformed, low-entropy, or unknown material can pass.

## 12. Contract-2 record shapes

### 12.1 Exact private clean cut

With fixed time and the existing ID algorithm:

```json
{
  "kind": "cut",
  "id": "pc_94f5df71022d",
  "ts": "2026-07-12T00:00:00.000Z",
  "agent": "codex",
  "text": "workspace tests only pass from apps/web",
  "tags": ["tooling"],
  "severity": "minor",
  "cwd": ".",
  "repo": null,
  "path_policy": "omitted",
  "path_encoding": "omitted",
  "content_policy": {
    "version": 1,
    "mode": "balanced",
    "decision": "clean",
    "categories": [],
    "fields": []
  }
}
```

Field order shown above is the canonical serialized order for new cut events.

### 12.2 Exact committed clean cut

```json
{
  "kind": "cut",
  "id": "pc_94f5df71022d",
  "ts": "2026-07-12T00:00:00.000Z",
  "agent": "codex",
  "text": "workspace tests only pass from apps/web",
  "tags": ["tooling"],
  "severity": "minor",
  "cwd": "/Users/alice/work/papercuts/apps/web",
  "repo": "/Users/alice/work/papercuts",
  "path_policy": "legacy-absolute",
  "path_encoding": "utf8",
  "content_policy": {
    "version": 1,
    "mode": "strict",
    "decision": "clean",
    "categories": [],
    "fields": []
  }
}
```

The ID is identical because path and policy fields are not hash inputs.

### 12.3 Resolve event

```json
{
  "kind": "resolve",
  "id": "pc_94f5df71022d",
  "ts": "2026-07-12T01:00:00.000Z",
  "agent": "codex",
  "note": "fixed in br-example",
  "content_policy": {
    "version": 1,
    "mode": "balanced",
    "decision": "clean",
    "categories": [],
    "fields": []
  }
}
```

The folded resolution projection carries the event's `content_policy` beside
`ts`, `agent`, and `note`.

### 12.4 Legacy and mixed events

- missing `path_policy` means contract-1 legacy path semantics;
- missing `content_policy` means `legacy-unscanned`;
- hardened readers accept v1, omitted, legacy-absolute, and mixed records;
- private output projects legacy paths safely but leaves content policy absent;
- doctor reports sanitized retained counts, not raw values;
- no read or migration rewrites old events to add fields.

Doctor reports `path_policy_mismatch` when an omitted record has non-sentinel
path fields or a legacy record contradicts its encoding label. It reports
`content_policy_mismatch` when decision, category, field, mode, or version-1
invariants conflict. These are findings for malformed new records; absence of
policy on a contract-1 event is a compatible warning count, not corruption.

## 13. Output metadata and diagnostics

### 13.1 Private success metadata

Add and resolve success metadata include the full effective policy:

```json
{
  "contract": 2,
  "storage_profile": "private",
  "profile_source": "default",
  "storage_source": "profile-default",
  "write_policy": "normal",
  "path_policy": "omitted",
  "sensitive_policy": "balanced",
  "sensitive_policy_source": "profile-default",
  "sensitive_policy_version": 1
}
```

Private metadata omits `file`. Committed metadata may include `file` and uses
`path_policy: legacy-absolute`. An explicit target changes only
`storage_source`.

List and doctor include contract, storage profile/source, target source,
write policy, and path policy, but omit current sensitive-policy fields because
no ingestion decision is made. Schema metadata is only the static contract
version and has no effective environment-derived policy.

Existing v0.1 dry-run, duplicate, and already-resolved warnings remain. New
sanitized warnings use stable code-like values where applicable:

```text
storage_required_for_writes
legacy_journal_detected
legacy_journal_retained
legacy_path_records_retained:N
legacy_unscanned_records:N
legacy_absolute_path_exposure
lossy_legacy_path_encoding
```

Counts contain no values or paths. `content_policy.decision` and categories are
the machine source for warn/override state; warning prose is not.

All `meta.warnings` strings are deduplicated and lexicographically sorted before
serialization. Category, field, candidate-ID, and other set-like arrays are also
sorted and deduplicated. Doctor findings retain their deterministic line/
finding order defined by schema. Tests pin these rules under changed locale,
cwd, and clock.

### 13.2 Safe private errors

Contract-2 argument and configuration errors never echo a rejected argv or
environment value. This applies before a valid profile can be resolved and
therefore applies to both profiles. The parser reports the argument name and
accepted form, not Clap's raw value-bearing error string. Invalid ID and
`--since` messages likewise omit the rejected value. Ambiguous-ID details may
list validated candidate papercut IDs because those IDs are product-generated,
but they do not repeat the caller's prefix.

Invalid environment errors name the variable and expected format without its
contents. Oversize errors may report field name, byte count, and maximum; they
do not report bytes. Help and version remain intentional plaintext output.

Private diagnostics never interpolate `Path::display`, lossy path conversion,
raw OS error text, basenames, parent counts, drive letters, or home-relative
forms. They use an opaque location:

```text
current_working_directory
repository_marker
git_directory
git_common_directory
private_journal
explicit_journal
stdin
stdout
```

Representative error:

```json
{
  "ok": false,
  "error": {
    "code": "permission_denied",
    "message": "permission denied for the selected private journal",
    "details": {
      "location": "private_journal",
      "os_kind": "permission-denied"
    },
    "retryable": false,
    "suggested_fix": "Review the selected storage permissions without pasting the path into logs."
  },
  "meta": {
    "contract": 2,
    "storage_profile": "private",
    "profile_source": "default",
    "storage_source": "profile-default",
    "path_policy": "omitted"
  }
}
```

Committed profile explicitly permits v0.1-style path-bearing records,
`meta.file`, and diagnostics and must warn about that exposure. Explicit file
under private remains safely redacted.

### 13.3 Error and exit dictionary

Preserved codes keep their exits. Contract 2 adds:

| Code | Exit | Meaning |
|---|---:|---|
| `sensitive_input` | 65 | refusing content-policy finding |
| `insecure_private_permissions` | 77 | implicit private state is not user-only |
| `writes_disabled` | 78 | monotonic no-write guard denied mutation |
| `storage_required` | 78 | private non-Git mutation needs explicit target |
| `migration_required` | 78 | legacy-only state needs operator selection/copy |
| `invalid_repository` | 78 | nearest Git marker or metadata is malformed |
| `unsupported_filesystem` | 78 | explicitly detected unsupported filesystem |
| `unsafe_journal_symlink` | 78 | selected final private journal is a symlink |

Existing `invalid_argument` 2, `invalid_input`/`ambiguous_id` 65,
`not_found` 66, `internal` 70, `io_error` 74, retryable `lock_timeout` 75,
`permission_denied` 77, and `config_error` 78 remain.

Suggested fixes never teach an agent to turn off read-only, select committed,
or authorize sensitive content merely to make the command pass.

## 14. Schema and compatibility policy

### 14.1 Schema is the source of truth

`papercuts schema` publishes contract 2 and includes:

- every command, flag, environment variable, mutation/read/dry-run annotation;
- profile and target precedence;
- policy floors, override rules, scanner categories and bounds;
- exact cut, resolve, list-item, metadata, and error examples;
- path policies, projection, strict repository grammar, and symlink rules;
- storage/migration state machine and permissions;
- all errors/exits and warning meanings;
- ID inputs and excluded fields;
- contract-1 inference and mixed-journal behavior;
- known content/path/security limitations;
- local-filesystem and no-network boundary.

Error codes and category names have one code-level source of truth consumed by
schema and tests. Documentation must not duplicate divergent constants.

### 14.2 v0.1 read compatibility

Default serde unknown-field behavior allows the unchanged v0.1 record reader to
parse new cuts/resolves because all required legacy fields remain. It ignores
new fields, sees private cwd `.` and repo null, and recomputes the same ID.

This is parse compatibility only:

- v0.1 does not understand sentinel, profile, path, or content-policy semantics;
- its output can drop unknown audit fields after deserialization;
- its own discovery does not find the private common-dir journal unless given
  the exact file;
- it has no scanner, no new limits, and path-bearing diagnostics;
- it must not be described as contract-2 compatible.

### 14.3 Committed compatibility boundary

Committed preserves v0.1 storage location, automatic absolute-path meaning,
explicit-target behavior, append/fold/lock semantics, and visible-journal
doctor behavior for valid repositories.

It intentionally diverges on default selection, Git validation, input limits,
content refusal, audit/path fields, schema/meta contract, and new errors. No
hardened flag disables those changes.

## 15. Migration and rollback

### 15.1 Explicit copy-and-verify migration

Migration is an operator procedure, never normal discovery:

1. stop writers;
2. doctor the committed source;
3. resolve validated common Git directory;
4. confirm private target absent and supported local filesystems;
5. create user-only private directory;
6. copy source bytes without overwrite;
7. set user-only file permissions;
8. compare bytes;
9. doctor the private copy;
10. retain the legacy source unchanged.

Representative Unix commands:

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

Copying does not sanitize paths or text, remove the worktree file, untrack Git
history, retract pushes, or merge histories. Those are separate approved
operations.

### 15.2 Selection-only rollback

To inspect the untouched legacy source:

```bash
papercuts --profile committed doctor
papercuts --profile committed list --status all
```

Restoring committed writes is an explicit operator choice. Do not delete the
private target. If both journals received new events, stop writers and preserve
both; do not concatenate or auto-merge.

To roll back the binary, use the exact upstream v0.1 binary only with explicit
acceptance that it is unchecked and not contract-2 aware. No rollback rewrites
events or reconstructs omitted paths.

Published Git history is rolled back by normal revert pull request, never reset,
rebase, tag movement, or force-push.

## 16. Mechanism, policy, and ownership boundaries

### 16.1 Rust mechanism owns

- CLI parsing and relevant environment validation;
- typed profile/target/write/content policy resolution;
- strict Git/common-dir resolver;
- path-safe record construction and projection;
- bounded content scanner and override enforcement;
- append/fold/lock journal behavior;
- deterministic schema, metadata, errors, and doctor findings;
- no-create dry runs and no-fallback failure behavior.

### 16.2 Repository or harness instructions own

- whether the current task authorizes any papercuts mutation;
- setting `PAPERCUTS_READ_ONLY=1` for restricted sessions;
- forbidding add/resolve during read-only work even if the binary could run;
- requiring the agent to continue the main task after ordinary logging;
- forbidding autonomous sensitive override controls;
- deciding when a friction item becomes a real bug or issue instead.

### 16.3 Human/operator owns

- explicit target choice outside Git;
- migration, permission repair, rollback, and journal reconciliation;
- exact sensitive-category override authorization;
- later project aliases and allowlist entries;
- pilot admission and release approval.

### 16.4 Later adapters own

- private multi-project inventory and aliases;
- bounded aggregate JSON/Markdown;
- promotion to Beads, ClickUp, CM, runbooks, or issues;
- no source-journal rewrite or implicit resolution.

ACFS, ClickUp, Beads, CM, or host-specific behavior does not enter the generic
Rust logging core.

## 17. Upstream, naming, and distribution

- GitHub repository remains the public fork `39elbarto/papercuts`.
- MIT license, upstream copyright, and README attribution remain.
- `upstream` is fetch-only with a disabled push URL.
- public `main` is never rebased, reset, or force-pushed.
- upstream is integrated on dated branches through reviewed merge-commit PRs;
  published mistakes are reverted through PRs.
- generic contract-1 documentation, MSRV, and repository-hygiene fixes remain
  separable upstream candidates.
- private default, path omission, strict validation, content policy, and ACFS
  adapters remain fork-only unless upstream explicitly accepts them.
- package and binary may remain `papercuts` only through development and an
  exact-SHA isolated pilot invoked by exact path.
- if fork-only behavior remains, rename package and binary before non-isolated
  distribution and prove coexistence with upstream.
- the first breaking prerelease is no lower than `0.2.0-alpha.1` and uses a
  `hardened-v` tag prefix.
- never publish to crates.io as `papercuts`; no registry workflow or token is
  added for that namespace.
- every release says it is a community-maintained fork, not upstream, and lists
  exact upstream/fork SHAs, compatibility, migration, limitations, gates, and
  rollback.

## 18. Deferred multi-project contract

Cross-project work is gated after the single-project pilot. Contract 2 adds no
multi-project CLI surface.

The future design Bead must preserve these already-fixed boundaries:

- one explicit private allowlist is the sole discovery authority;
- no recursive directory scan, Git-remote discovery, or broad filesystem read;
- operator alias is 1-64 lowercase ASCII, begins alphanumeric, then
  alphanumeric/dot/underscore/hyphen; `.` and `..` are forbidden;
- alias maps privately to one canonical journal and is never written back;
- one canonical journal cannot appear under two aliases;
- linked worktrees share one source; separate clones remain separate;
- digest emits alias, never locator/path/hash/remote/machine identity;
- source journals remain authoritative and read-only to the adapter;
- warn, override, and legacy-unscanned content is excluded by default unless a
  separately approved explicit inclusion gate exists;
- aggregation never relabels old content clean or resolves source items.

The pilot must demonstrate that single-project logging and review are useful
before this surface is implemented.

## 19. Rejected alternatives

The consolidated decision rejects:

- committed repository storage as the default;
- one global HOME journal for unrelated projects;
- tracked private state plus automatic `.gitignore` edits;
- automatic migration, union reads, concatenation, or journal rewrite;
- path-relative, basename, remote-derived, hashed, or random automatic project
  identity;
- omitted legacy keys that would break v0.1 record parsing;
- `--file` implicitly selecting legacy path exposure;
- safe errors containing paths or raw OS strings;
- warn-only high-confidence sensitive policy;
- refuse-everything as the only private mode;
- generic entropy, recursive decoding, runtime pattern downloads, or mandatory
  external scanners;
- one-key force/wildcard scanner bypass;
- using committed profile as a scanner bypass;
- eager clock/env reads on unrelated commands;
- Git subprocess dependency for normal discovery;
- automatic chmod of intentionally existing shared state;
- multi-project discovery before pilot evidence;
- package publication to an unverified upstream namespace;
- public-history recovery by rewrite.

## 20. Required implementation and test gates

### 20.1 Dependency order

1. `x30.7` implements the shared typed profile/target/write/content policy and
   storage state machine.
2. `x30.8` implements strict Git/path resolution, record path fields, and safe
   projection.
3. `x30.9` implements bounded content preflight and audit.
4. `x30.10` publishes contract-2 schema/errors/metadata after shapes stabilize.
5. `x30.11` completes adversarial real-binary acceptance across all surfaces.
6. `x30.12` publishes verified single-project instructions and runbook.
7. `x30.13` runs the release gate before pilot design proceeds.

`x30.8` and `x30.9` may proceed after the shared `x30.7` seam exists, with file
reservations for overlapping record/command surfaces. `x30.10` waits for both.

### 20.2 Mandatory functional matrix

Tests must cover:

- profile/file/policy/read-only precedence and invalid environment values;
- relevant-env and lazy-clock behavior for every command;
- ordinary Git, worktree, submodule, bare/non-Git, malformed nested marker,
  gitdir/commondir grammar, missing structures, and no Git in PATH;
- first run, legacy-only, dual-journal, explicit target, missing HOME,
  permissions, symlinks, unsupported filesystem, and no fallback;
- exact private/committed/resolve bytes and same cut ID;
- contract-1, contract-2, mixed, malformed, torn, duplicate, conflict, and
  orphan events;
- private path absence across journal/stdout/stderr for every command path;
- committed exposure warnings and lossy encoding;
- every high/medium content category, field, bound, false-positive control,
  override state, Unicode/encoded limitation, and dry-run behavior;
- unique-sentinel no-echo/no-write assertions for every refusal;
- schema/error dictionary and old-reader compatibility;
- distinct/identical add and resolve races; five repetitions when shared store
  or concurrency code changes;
- documented commands against a built binary in disposable repositories.

### 20.3 Performance and quality gates

- full release build, all-feature tests, clippy with warnings denied, and fmt;
- `git diff --check`;
- scoped UBS for code/scripts/executable configuration;
- journal doctor;
- Gitleaks over intended public diff/history before release;
- release-mode content benchmark over maximum payload and full catalog for at
  least 10,000 iterations, recording host/SHA/p50/p95/max;
- benchmark budget p95 at most 5 ms and max at most 20 ms on the release-gate
  host;
- manual stdout/stderr/journal inspection for hidden paths and synthetic
  sentinel fragments;
- exact commands, versions, commit SHA, evidence location, and rollback in each
  code Bead closeout.

## 21. Implementation authorization and change control

Closing this architecture Bead authorizes `x30.7` as the next implementation
slice. It does not authorize parallel edits to overlapping files without Agent
Mail reservations.

Implementation must follow this document and the copied context in its Bead.
If code proves a contract impossible or materially unsafe:

1. stop the affected implementation slice;
2. record the exact conflict and evidence;
3. reopen or create a narrow architecture Bead;
4. update this ADR, plan, schema expectations, and every affected downstream
   Bead together;
5. do not silently reinterpret the contract in code.

No release, pilot, or broader adoption may describe planned behavior as already
implemented. Contract 2 becomes a product claim only after the implementation,
adversarial suite, documentation, and release gates pass.
