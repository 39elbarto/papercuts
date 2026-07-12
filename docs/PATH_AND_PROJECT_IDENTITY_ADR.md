# Path Minimization and Project Identity Contract

Bead: `br-hardened-papercuts-fork-x30.3`

Decision date: 2026-07-12

Status: accepted input to the consolidated hardened-contract ADR; not yet
implemented

## 1. Decision

The hardened fork will not automatically persist a filesystem path or derived
path fingerprint in its default `private` profile.

New safe cut records retain the contract-1 `cwd` and `repo` fields for parser
compatibility, but use non-identifying sentinel values and an explicit policy:

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
  "path_encoding": "omitted"
}
```

`cwd: "."` is a compatibility sentinel meaning “automatic path context was
withheld.” It does not assert that the process ran at repository root. New
contract-aware readers use `path_policy`, not the sentinel alone, to interpret
the fields.

The example text deliberately contains a user-authored relative hint. Automatic
path omission does not sanitize text; the sensitive-content policy decides
whether such input is accepted.

The explicit `committed` compatibility profile retains upstream absolute path
capture and labels it:

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
  "path_encoding": "utf8"
}
```

The profile remains active when `--file` or `PAPERCUTS_FILE` overrides the
journal target. An explicit target does not silently opt into absolute path
capture:

- default or `--profile private` plus `--file` uses `path_policy: omitted`;
- `--profile committed` plus `--file` uses `legacy-absolute`.

Project identity is not stored in source journal records. A later
multi-project adapter assigns a human-chosen alias to each explicitly
allowlisted journal and adds that alias only to the aggregate view. No absolute
path, path hash, remote URL, repository name, random UUID, or machine identity
is derived and written automatically.

## 2. Why this decision is required

The upstream implementation has path-bearing behavior in every major surface:

- `src/lib.rs:39-50` requires `cwd: String` and `repo: Option<String>`;
- `src/commands/add.rs:63-70` captures current directory and repository through
  `to_string_lossy`;
- `src/output.rs:6-31` supports `meta.file`, populated by add, list, resolve,
  and doctor;
- `src/error.rs:123-173` interpolates paths into lock, permission, I/O, and
  missing-file errors;
- `src/commands/list.rs:27-35` and `src/commands/doctor.rs:34-42` build their own
  path-bearing missing-file errors;
- `src/commands/schema.rs:5-41` advertises absolute record paths, absolute
  `meta.file`, and absolute discovery locations;
- `tests/cli.rs:632-728,860-888` pins absolute discovery and metadata behavior.

A disposable non-UTF-8 probe on 2026-07-12 confirmed the current lossy output:

```text
physical directory suffix: nonutf8 followed by byte FF
current JSON cwd suffix: nonutf8 followed by Unicode replacement character
```

Lossy conversion both discloses layout and destroys byte identity. Hashing the
original path would still create a stable correlator and would not restore the
lost semantics.

The default private journal already has a project scope through the validated
Git common directory selected by `SAFE_STORAGE_PROFILES_ADR.md`. Repeating that
identity inside every event adds exposure without adding storage correctness.

## 3. Threat boundary and security claim

This decision protects against accidental path disclosure in journals, command
output, error output, review digests, and public commits. It does not protect
against a malicious local user or another process with equivalent filesystem
authority.

Omission applies only to automatically captured path metadata. Agent-authored
`text`, tags, resolution notes, and explicit project aliases can still contain
sensitive names or paths. The sensitive-content ADR governs those inputs.

The fork must say:

- automatic filesystem paths are omitted in the private profile;
- old or explicitly legacy records can still contain paths on disk;
- safe output redacts stored legacy path fields;
- path omission is not encryption and does not make arbitrary text safe.

It must not say “papercuts cannot leak paths.”

## 4. Record contract

### 4.1 New fields

Contract-2 cut records add:

```text
path_policy: omitted|legacy-absolute
path_encoding: omitted|utf8|lossy-utf8
```

Both fields are serialized on every new cut. They are not included in the
content ID. `cwd` and `repo` also remain excluded from the ID, preserving the
current deterministic hash inputs:

```text
timestamp, agent, text, severity, sorted tags
```

The same user content and fixed time therefore produces the same ID in private
and committed profiles. This preserves duplicate behavior across migration and
profile selection.

### 4.2 Private record invariants

For `path_policy: omitted`:

- `cwd` is exactly `.`;
- `repo` is exactly null;
- `path_encoding` is exactly `omitted`;
- no current-directory, repository-root, common-Git-directory, file-target,
  remote, username, home directory, drive letter, UNC share, or symlink target
  is serialized automatically.

A mismatch is a doctor finding named `path_policy_mismatch`. Examples include
an omitted-policy record with a non-sentinel cwd or a non-null repo.

### 4.3 Committed record invariants

For `path_policy: legacy-absolute`:

- `cwd` and `repo` retain upstream meanings;
- `repo` may be null outside a detected repository;
- valid UTF-8 paths use `path_encoding: utf8`;
- if either captured path is not valid UTF-8, upstream-compatible lossy
  conversion is retained and `path_encoding` is `lossy-utf8`;
- command metadata warns that legacy absolute path exposure is active;
- the sensitive-data guard does not claim to make these automatic fields safe.

This is an explicit compatibility lane. It is not selected by `--file` alone.

### 4.4 Contract-1 records

A cut with no `path_policy` is interpreted as a contract-1 legacy record. The
stored bytes remain untouched.

- hardened readers continue to fold it;
- private-profile output projects it through the omitted safe view;
- committed-profile output may expose its stored absolute fields;
- doctor may report a sanitized count of retained legacy path-bearing records,
  but their presence after explicit migration is not corruption;
- no automatic migration rewrites it into a contract-2 line.

Private doctor reports the number of retained legacy path-bearing records as a
sanitized metadata warning such as `legacy_path_records_retained:3`. The count
does not make doctor unhealthy and does not include line contents or paths.

### 4.5 Resolve events

Resolve records gain no automatic path or project field. Their free-form note
is governed by the sensitive-content policy. A resolve response that embeds the
original cut applies the active output projection before serialization.

## 5. Output projection

Storage bytes and command output are separate concerns. Every command returning
a cut must project it according to the active profile.

### 5.1 Private profile

Private projection applies to:

- `add`, including duplicate-existing and dry-run results;
- `list` JSON;
- `list --format md` warnings and any future context fields;
- `resolve`, including already-resolved and dry-run results;
- doctor findings and warnings;
- every structured error and suggested action.

For any stored contract-1 or legacy-absolute cut, private projection returns:

```json
{
  "cwd": ".",
  "repo": null,
  "path_policy": "omitted",
  "path_encoding": "omitted"
}
```

The projection does not rewrite the journal. It creates a sanitized output
value after fold and before filtering/serialization. Filters must never inspect
or expose hidden path values.

### 5.2 Committed profile

Committed projection preserves stored legacy paths and emits a warning such as:

```text
legacy absolute path exposure is active
```

New omitted-policy records remain omitted; the committed profile does not
invent a path that was never stored.

### 5.3 Markdown

The current Markdown list does not render cwd, repo, or file target. It retains
that property. Warnings must use policy names and opaque location labels only.

## 6. Metadata and diagnostics

### 6.1 Success metadata

Private-profile success metadata omits `meta.file`. It includes:

```json
{
  "storage_profile": "private",
  "profile_source": "default|flag-profile|env-profile",
  "storage_source": "profile-default|flag-file|env-file",
  "write_policy": "normal|read-only",
  "path_policy": "omitted"
}
```

Committed-profile metadata may include the upstream-compatible `meta.file` and
uses `path_policy: legacy-absolute`. If the file or captured record paths are
lossy, metadata adds a sanitized warning without echoing the original bytes.

### 6.2 Safe error vocabulary

Private-profile errors do not interpolate `Path::display()`, `to_string_lossy`,
or raw `std::io::Error` text. OS error strings can themselves contain paths.

They identify only an opaque location enum:

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

Representative safe error:

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

Raw OS error messages, path bytes, basenames, parent counts, drive letters, and
home-relative forms are forbidden in private stdout/stderr. A basename is still
potentially identifying and is not a safe substitute.

### 6.3 Legacy diagnostics

`--profile committed` explicitly opts into upstream path-bearing records,
`meta.file`, and path-bearing diagnostics. Help, schema, and each command's
metadata must make that exposure visible.

An explicit file does not enable legacy diagnostics unless the committed
profile is also selected.

## 7. Project identity contract

### 7.1 No identity inside source records

Source cuts and resolves contain no automatic `project`, repository name,
remote URL, host name, machine ID, UUID, or path-derived key.

Reasons:

- the journal is already scoped to one project by storage resolution;
- a path hash is a persistent correlator and can be brute-forced from likely
  local paths;
- remotes can include private hostnames, owners, credentials, or repository
  names;
- repository basenames collide and can themselves be sensitive;
- random IDs add lifecycle and clone semantics without helping single-project
  logging.

### 7.2 Later allowlisted inventory

The later multi-project adapter owns project naming. Its private local
configuration maps an operator-chosen alias to one resolved journal.

Alias contract:

- 1 to 64 lowercase ASCII characters;
- first character is `a-z` or `0-9`;
- remaining characters are `a-z`, `0-9`, `.`, `_`, or `-`;
- `.` and `..` are forbidden;
- no path separator, whitespace, URL, or automatic normalization;
- aliases must be unique;
- one canonical journal cannot appear under two aliases in one inventory.

The aggregate output can add:

```json
{
  "project": "papercuts",
  "items": []
}
```

The alias is deliberate operator-authored data. It is not written back to the
source journal. The adapter stores any absolute journal locator only in its
private allowlist and never includes it in a public digest.

Linked worktrees map to one journal and one alias through their common Git
directory. A separate clone is a separate source until the operator explicitly
chooses how to name or deduplicate it. No automatic remote comparison occurs.

### 7.3 Collision behavior

- duplicate alias: configuration error;
- same canonical journal under multiple aliases: configuration error;
- different journals with similar names: allowed only under distinct aliases;
- alias change: explicit configuration edit; source history is unchanged;
- no probabilistic hash means there is no hash-collision policy.

## 8. Strict repository resolver

The same typed resolver supplies repository root and common Git directory to
storage and path policy. Do not keep the current `.git.exists()` heuristic.

### 8.1 Search

1. Resolve the current directory to an existing physical path without
   serializing it.
2. Search it and its ancestors for the nearest `.git` marker.
3. If no marker exists, return non-Git context.
4. If a marker exists but is malformed, return a configuration error; do not
   skip it and inherit an outer repository.

This prevents a broken nested marker from silently routing a journal to an
unrelated parent project.

### 8.2 Ordinary repository marker

An ordinary `.git` marker must be a real directory, not a symlink. Validate:

- the directory exists;
- `HEAD` is a regular file;
- `objects` is a directory and `config` is a regular file;
- the resolved common directory is the marker directory.

### 8.3 Gitdir file

A worktree or submodule `.git` marker must be a regular file, not a symlink.
Its content must be one logical line:

```text
gitdir: PATH
```

Accept LF or CRLF termination and an absolute or marker-parent-relative target.
Reject empty targets, NUL, additional non-empty lines, unknown prefixes,
non-directory targets, and unreadable targets.

The target Git directory must contain a regular `HEAD` file.

### 8.4 Common directory

If the Git directory contains a `commondir` file, parse it as one absolute or
Git-directory-relative path using the same line and NUL rules. Otherwise the
Git directory is the common directory.

The common directory must exist, contain an `objects` directory, and contain a
regular `config` file. Do not require a `refs` directory: a valid repository may
use another ref backend. The common path may legitimately sit outside the
worktree, as with linked worktrees and submodules.

The implementation uses platform-native path/OsString handling. It must not
round-trip repository metadata through UTF-8 merely to validate it. No ambient
Git subprocess is required for normal add/list/resolve/doctor behavior.

### 8.5 Bare repositories

A bare repository is not a validated working tree for this product. Without an
explicit journal it receives the private non-Git behavior selected by the
storage ADR.

## 9. Symlink and traversal rules

### 9.1 Discovery

- canonicalize the existing cwd for discovery only;
- use the nearest physical repository marker;
- direct symlink `.git` markers are configuration errors;
- canonicalize validated gitdir and commondir targets internally;
- never serialize the logical or physical cwd in private mode.

A live probe confirmed that entering a repository through a directory symlink
can leave shell `PWD` logical while Git resolves the physical top-level and
common directory. Private output exposes neither form.

### 9.2 Private journal

The common directory may be reached through a validated gitdir/commondir
reference, but the final `papercuts` directory and `log.jsonl` must not be
symlinks. Existing symlinks cause a configuration error before open or append.

This is an accidental-redirection guard, not a complete defense against a
malicious same-user time-of-check/time-of-use race. The threat model must state
that limit.

### 9.3 Explicit journal

Under the private profile, an explicit final file symlink is rejected. Parent
symlinks are permitted because the operator explicitly chose the path, but the
resolved target is never echoed. The implementation must still preserve
missing-explicit and create-parent semantics.

Under the committed profile, explicit paths retain upstream legacy behavior,
including normal operating-system symlink resolution. This is another reason
the profile is an explicit compatibility lane.

### 9.4 Lexical traversal

Relative explicit paths continue to normalize `.` and `..` against cwd as
defined by the storage ADR. Private diagnostics do not expose the normalized
result. Repository marker targets are canonicalized only after exact parsing
and existence checks.

## 10. Non-UTF-8 and platform behavior

Private policy avoids converting filesystem paths to JSON at all.

- on Unix, repository parsing and I/O retain raw `OsStr` bytes;
- on Windows, use native wide strings and do not claim Unix byte semantics;
- aliases remain explicit ASCII and are independent of filesystem encoding;
- no safe record or error contains Unicode replacement characters derived from
  a path.

Committed mode retains upstream lossy conversion for compatibility and marks
new records `path_encoding: lossy-utf8` when `Path::to_str()` fails for any
captured path. It must warn without including the original or converted path in
the warning itself.

No path hash is computed before or after lossy conversion.

## 11. Migration, mixed journals, and rollback

### 11.1 Existing bytes

Migration from committed to private storage can copy contract-1 bytes exactly,
as selected by the storage ADR. Those bytes may contain absolute paths. The
copy makes them private state; it does not sanitize them.

Private-profile reads redact them in output. Source bytes remain append-only.
If the operator needs historical erasure, that is a separately approved data
rewrite with backups and is outside the hardened first release.

### 11.2 New safe records in old readers

Contract-1 readers using serde's default unknown-field behavior can parse new
safe cuts because required `cwd` and `repo` remain present and new fields are
ignored. They will see cwd `.` and repo null. This is parse compatibility, not
semantic awareness of path omission.

A live compatibility probe on 2026-07-12 fed the exact safe example to the
unchanged v0.1 binary. `list --status all` returned one item with the expected
ID, cwd sentinel, and null repo; `doctor` was healthy and recomputed the ID.

The existing ID and doctor recomputation remain valid because paths and new
policy fields are not hash inputs.

### 11.3 Hardened readers

Hardened readers accept:

- contract-1 records with no policy field;
- contract-2 omitted records;
- contract-2 legacy-absolute records;
- mixed journals in any order.

Private output always applies omitted projection. Committed output returns
stored path context where present.

### 11.4 Rollback

Selecting `--profile committed` restores legacy projection for stored legacy
records and legacy capture for new records. It does not reconstruct paths for
omitted records.

An upstream v0.1 binary can parse omitted records but does not understand their
sentinel semantics. Operators must not describe that as full contract-2
support. No rollback step rewrites a journal.

## 12. Alternatives considered

### 12.1 Repository-relative cwd

Rejected as the default. Relative paths still reveal internal component names,
can identify ignored/private subtrees, and differ across linked worktrees and
symlink entry points. Agent-authored text can include a deliberately reviewed
relative hint when it is truly needed.

### 12.2 Omit cwd/repo keys entirely

Rejected for the first hardened contract because upstream contract-1 readers
would fail to deserialize the cut and silently skip it. Sentinel values plus an
explicit policy minimize data while preserving parse compatibility.

### 12.3 Hash absolute paths

Rejected. A deterministic hash is a cross-record and cross-output correlator,
likely paths can be brute-forced, and rename/clone behavior is poor. A salted
hash introduces secret lifecycle and breaks deterministic cross-machine use
without providing meaningful identity.

### 12.4 Derive project identity from remote URL

Rejected. Remotes may be absent, multiple, credential-bearing, private, or
unstable. Reading them is unnecessary for local logging and leaks ownership and
host information.

### 12.5 Persist a random project UUID

Deferred. It would require creation, clone/copy semantics, backup, and alias
mapping without helping the single-project journal. The allowlisted adapter can
introduce a private registry later if evidence proves a need.

### 12.6 Automatically capture basename only

Rejected. Repository and directory basenames can contain customer, product,
username, or incident names and are not reliably unique.

### 12.7 Expose paths in safe errors for debugging

Rejected. Errors are commonly copied into chat and logs. Opaque location codes
plus an explicit legacy profile keep the default honest.

### 12.8 Make `--file` imply legacy paths

Rejected. Choosing storage is not permission to publish unrelated path
metadata. The selected profile remains authoritative.

## 13. Schema and contract implications

`schema` must publish:

- exact omitted and legacy record examples;
- `path_policy` and `path_encoding` values and invariants;
- path fields excluded from IDs;
- contract-1 inference and mixed-journal rules;
- profile-driven output projection;
- `meta.file` omission in private and availability in committed;
- opaque safe diagnostic locations;
- strict Git marker, gitdir, commondir, symlink, and encoding rules;
- external allowlist alias semantics;
- the statement that text/notes can still contain paths.

This is an observable record/output change and requires machine contract 2 or
higher. The consolidated ADR decides the final number, but it cannot remain 1.

## 14. Implementation boundaries

This Bead changes planning/docs only. Product implementation remains blocked on
the consolidated ADR.

Expected ownership:

- typed Git/path resolver and safe record construction:
  `br-hardened-papercuts-fork-x30.8`;
- profile/target resolution shared seam: `br-hardened-papercuts-fork-x30.7`;
- output/error projection, schema, and compatibility:
  `br-hardened-papercuts-fork-x30.10`;
- adversarial filesystem/output tests: `br-hardened-papercuts-fork-x30.11`;
- safe agent/operator documentation: `br-hardened-papercuts-fork-x30.12`;
- multi-project alias/allowlist implementation, only after pilot evidence:
  `br-hardened-papercuts-fork-x30.18`.

Use one policy type through discovery, record construction, fold projection,
metadata, and errors. Do not sprinkle profile checks across command modules.
Do not change append/fold ordering or the ID algorithm.

## 15. Required unit and black-box matrix

### Record shapes and compatibility

- exact private record matches the example byte-for-byte with fixed time;
- exact committed record matches its example;
- same fixed content produces the same ID under both policies;
- omitted-policy mismatch is a doctor finding;
- contract-1 records infer legacy policy and remain readable;
- upstream v0.1 parses omitted records and recomputes IDs;
- mixed record policies fold and sort identically.

### Projection and output

- private add, duplicate add, dry-run, list, resolve, already-resolved, doctor,
  and JSON errors contain no forbidden path fragments;
- private list redacts stored contract-1 and legacy records;
- Markdown contains no automatic path or file target;
- private `meta.file` is absent;
- committed output retains absolute context and exposure warning;
- explicit file under private stays redacted;
- explicit file under committed preserves legacy projection;
- stdout and stderr are scanned for username, home, repo, common-dir, basename,
  drive, UNC, and symlink-target fragments.

### Repository resolver

- ordinary `.git` directory;
- linked worktree absolute gitdir plus relative commondir;
- submodule relative gitdir;
- nested nearest valid repository;
- no repository;
- bare repository;
- empty marker, wrong prefix, NUL, extra line, missing target, file target, and
  unreadable target;
- invalid nested marker does not inherit outer repository;
- direct symlink marker is rejected;
- missing HEAD, common objects, or common config is rejected;
- no Git binary in PATH still supports normal resolution.

### Symlink and traversal

- logical symlink cwd and physical cwd resolve one project without output;
- private final journal symlink is rejected;
- private explicit final symlink is rejected;
- private explicit parent symlink works without path echo;
- committed explicit symlink retains documented legacy behavior;
- relative `.` and `..` normalization remains deterministic.

### Encoding

- Unix non-UTF-8 cwd, repo root, gitdir target, commondir target, and explicit
  file do not reach private JSON/stdout/stderr;
- private records use `path_encoding: omitted`;
- committed valid UTF-8 uses `utf8`;
- committed non-UTF-8 uses `lossy-utf8` and a sanitized warning;
- no replacement character derived from a private path appears in output.

### Project alias

- valid alias boundaries and maximum length;
- uppercase, whitespace, separators, `.` and `..` rejected;
- duplicate alias rejected;
- same canonical journal under two aliases rejected;
- digest emits alias but not locator;
- no directory scan or remote lookup occurs.

### Migration and rollback

- copied legacy bytes remain unchanged;
- private view redacts copied paths;
- committed view restores stored legacy values;
- new omitted records remain parseable by upstream v0.1;
- rollback does not rewrite or synthesize paths;
- release evidence distinguishes a clean safe journal from a migrated journal
  that still retains historical absolute bytes.

## 16. Review passes

Three local review passes were completed without changing Rust product code:

1. **Surface inventory:** traced every automatic path from discovery and record
   construction through fold/list/resolve/doctor, metadata, schema, errors, and
   current black-box assertions.
2. **Adversarial privacy pass:** challenged relative paths, basenames, path
   hashes, remotes, aliases, logical/physical cwd, malformed nested markers,
   symlinks, non-UTF-8 paths, mixed legacy bytes, and safe-error copies.
3. **Compatibility and implementation pass:** generated an exact shared ID,
   proved v0.1 parses and doctors the proposed record, separated profile source
   from target source, allowed alternate Git ref backends, and mapped one typed
   policy seam plus downstream tests and owners.

The final pass found no unresolved contradiction inside this path/identity
slice. The sensitive-content and consolidated-contract dependencies below are
deliberate external gates.

## 17. Remaining gates

This decision resolves default path capture, record representation, output
redaction, repository identity, Git marker validation, worktree/submodule and
symlink behavior, non-UTF-8 behavior, migration, rollback, and test coverage.

It deliberately leaves these items to their assigned decisions:

- sensitive-text and resolution-note detection/override;
- the consolidated machine contract number above the minimum of 2;
- final private allowlist file location and lifecycle for the later
  multi-project adapter.

No Rust implementation starts until the consolidated hardened-contract ADR
reconciles this document with storage and sensitive-data policy and copies the
result into downstream Beads.
