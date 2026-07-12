# Deterministic Sensitive-data Guardrail Contract

Bead: `br-hardened-papercuts-fork-x30.4`

Decision date: 2026-07-12

Status: accepted input to the consolidated hardened-contract ADR; not yet
implemented

## 1. Decision

The hardened fork will run one bounded, local, deterministic content preflight
before a caller-controlled value can enter an append path. The preflight is a
guardrail against common accidental disclosures. It is not a secret scanner,
data-loss-prevention system, sandbox, or proof that accepted text is safe.

The first release has two modes:

- `balanced`, the `private` profile default, refuses high-confidence credential
  categories and accepts medium-risk categories with an auditable warning;
- `strict`, the `committed` profile default, refuses high-confidence and
  medium-risk categories.

There is no `off` mode. A caller may request a stricter mode, but cannot select
a mode below the active profile's floor. A deliberate category-specific
override can accept a refusing match only when both an operator-controlled
environment gate and explicit command flags agree. There is no wildcard
override.

All decisions use a repository-owned, versioned pattern table compiled into the
binary. Logging and validation make no network request, do not invoke another
scanner, and do not fetch a runtime pattern catalog.

## 2. Threat boundary and honest claim

This decision protects primarily against an agent or human accidentally placing
obvious credential or identifier material in:

- cut text;
- tags;
- an explicitly supplied or environment-derived agent name;
- a resolution note.

It does not protect against a malicious local process with the same authority,
an agent deliberately transforming a value to evade the rules, a compromised
binary, or disclosure that does not match the bounded catalog.

The fork may say:

> Papercuts checks bounded user-authored fields for a versioned set of common
> sensitive-data shapes before append. Refused values are not echoed or written
> by Papercuts.

It must also say:

> Detection is incomplete. Review text before logging and before publishing a
> journal. Private storage and path omission reduce exposure but do not make
> arbitrary content safe.

It must not say that Papercuts detects all secrets, sanitizes accepted input, or
prevents a sufficiently authorized caller from writing elsewhere.

Automatic `cwd`, repository, journal target, and project-alias values are not
scanner inputs. The private path contract omits them before record assembly.
The committed profile's legacy automatic absolute paths are an explicit
compatibility exposure and are not made safe by this content guardrail.

## 3. Public CLI and environment contract

Add one global policy flag and one repeatable flag on `add` and `resolve`:

```text
--sensitive-policy balanced|strict
--allow-sensitive CATEGORY
```

Add two environment variables:

```text
PAPERCUTS_SENSITIVE_POLICY=balanced|strict
PAPERCUTS_ALLOW_SENSITIVE=0|1|false|true
```

Values are ASCII case-insensitive. Empty values are unset. Other non-empty
values are `config_error`, exit 78. There is no repository config source: a
tracked repository cannot weaken the caller's policy or authorize an override.
`PAPERCUTS_AGENT` that is not valid UTF-8 is also a `config_error`; it must not
silently fall through to a detected or default agent name.

### 3.1 Policy resolution

Resolve the profile floor first:

| Active storage profile | Minimum content policy |
|---|---|
| `private` | `balanced` |
| `committed` | `strict` |

Then resolve a requested policy:

1. `--sensitive-policy`;
2. otherwise non-empty `PAPERCUTS_SENSITIVE_POLICY`;
3. otherwise the profile floor.

`strict` is stronger than `balanced`. A requested mode weaker than the profile
floor is a `config_error`, exit 78, rather than a silently ignored flag. Thus a
private caller may opt into strict behavior, while a committed caller cannot
quietly downgrade to balanced.

Success metadata reports:

```json
{
  "sensitive_policy": "balanced",
  "sensitive_policy_source": "profile-default|flag|env",
  "sensitive_policy_version": 1
}
```

### 3.2 Deliberate override

An override requires both:

1. `PAPERCUTS_ALLOW_SENSITIVE=1` or `true`; and
2. one repeated `--allow-sensitive CATEGORY` for every refusing category found.

Rules:

- the environment gate alone is inert;
- category flags without a truthy environment gate are `config_error`, exit 78;
- categories are exact, ASCII-lowercase names from `schema`;
- duplicate categories are deduplicated;
- an unknown category is `invalid_argument`, exit 2;
- `all`, `*`, comma-separated lists, negation, and an `off` category are invalid;
- if any refusing category remains uncovered, the command is refused;
- every supplied category must correspond to an observed refusing category;
  unused pre-authorizations are `invalid_input`, exit 65;
- the command never requires the caller to repeat the suspect value;
- accepted override metadata stores categories only, never values, offsets,
  fragments, hashes, or lengths.

This is defense in depth, not a human-approval mechanism. A local agent can
construct an environment. Canonical `AGENTS.md` policy must forbid an agent from
setting the gate or adding override flags without explicit human authorization
for that exact command. Operator documentation may explain the mechanism;
refusal output must not print a ready-to-run bypass command.

## 4. Inputs and size bounds

All lengths are UTF-8 bytes after CLI parsing and after trailing CR/LF removal
from stdin text. Validation precedes scanning.

| Field | Per-value maximum | Count maximum | Notes |
|---|---:|---:|---|
| add text | 10,000 bytes | 1 | preserves the v0.1 bound |
| resolution note | 2,000 bytes | 1 | new explicit bound |
| tag | 64 bytes | 16 | every tag is scanned separately |
| agent name | 128 bytes | 1 | flag and environment values share the bound |
| total scan payload | 16,384 bytes | one command | sum of field bytes |

Empty and whitespace validation remains command-specific. Oversize input is
`invalid_input`, exit 65, and reports only field name, actual byte count, and
maximum. These sizes are not sensitive match diagnostics.

Stdin must be read with an implementation-enforced `10,001` byte ceiling, not
unbounded `read_to_end` followed by a size check. The extra byte distinguishes
an exact-limit input from an oversize input. Argument and environment values
are already resident, but are rejected before storage discovery or record
construction.

The scanner consumes valid UTF-8. It does not normalize Unicode, perform
case-folding beyond ASCII where a rule says so, decode escapes, or join fields
into a display string. Multiline text and notes are scanned as supplied. Pair
rules may correlate category presence across fields without concatenating or
logging their contents.

## 5. Policy catalog version 1

The catalog has stable category names. Pattern details are implementation data,
not public diagnostics. Any semantic pattern change increments
`sensitive_policy_version`, updates the corpus, and appears in release notes.

### 5.1 High-confidence categories

These refuse in both modes unless deliberately overridden.

| Category | Detects | False-positive control |
|---|---|---|
| `private_key` | PEM private-key begin or end markers, including common PKCS, RSA, EC, OpenSSH, and PGP forms | requires a private-key marker, not any base64 text |
| `authorization_header` | non-placeholder HTTP `Authorization: Bearer ...` or `Authorization: Basic ...` material | header name and scheme required |
| `credential_url` | URI authority containing non-placeholder user and password material before `@` | both user and password positions required |
| `secret_assignment` | non-placeholder literal assigned to a secret-labelled key such as password, token, secret, api key, access key, client secret, or private key | label plus assignment delimiter required; variable references are exempt |
| `github_token` | GitHub token prefixes documented by GitHub, including `ghp_`, `github_pat_`, `gho_`, `ghu_`, `ghs_`, and `ghr_` | prefix plus bounded token body; no fixed 40-character assumption |
| `slack_token` | Slack prefixes documented for bot, user, workflow, and app-level tokens: `xoxb-`, `xoxp-`, `xwfp-`, and `xapp-` | prefix plus bounded token body |
| `stripe_secret_key` | Stripe secret or restricted key prefixes `sk_test_`, `sk_live_`, `rk_test_`, and `rk_live_` | publishable `pk_` keys are not classified as secrets |
| `aws_credential_pair` | an `AKIA` or `ASIA` access-key identifier paired with secret-access-key-labelled material in one command | access-key ID alone is not enough |

The initial vendor prefix set is intentionally small and sourced from vendor
documentation:

- [GitHub authentication documentation](https://docs.github.com/en/authentication/keeping-your-account-and-data-secure/about-authentication-to-github?apiVersion=2022-11-28);
- [AWS programmatic access documentation](https://docs.aws.amazon.com/IAM/latest/UserGuide/security-creds-programmatic-access.html);
- [Slack token type documentation](https://api.slack.com/docs/token-types);
- [Stripe API key documentation](https://docs.stripe.com/keys).

The table must not infer undocumented fixed token lengths. For example, GitHub
documents a staged `ghs_APPID_JWT` form, so a prefix rule must remain bounded
without assuming all `ghs_` tokens have the historical length.

### 5.2 Medium-risk categories

Balanced mode accepts these with `warn`; strict mode refuses them.

| Category | Detects | False-positive control |
|---|---|---|
| `email_address` | conventional local-part, `@`, and DNS-like domain shape | conservative ASCII form; no claim of RFC completeness |
| `personal_identifier` | value assigned to labels such as email, phone, customer ID, patient ID, user ID, or account ID | label and assignment delimiter required |
| `filesystem_path` | common absolute Unix paths, `/home/`, `/Users/`, Windows drive paths, and UNC paths | relative paths and isolated slash words remain allowed |
| `config_block` | two or more assignment-like lines in one text or note | threshold avoids classifying a single ordinary `key=value` example |

A config block warning does not imply that the block contains a secret. It is a
review signal for raw configuration context. A secret-labelled assignment is
still high-confidence and refuses under balanced mode.

### 5.3 Exact placeholder and reference exemptions

After surrounding quotes and ASCII whitespace are removed, a candidate value
is exempt only when the entire value ASCII-case-insensitively equals one of:

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

Shell variable references consisting only of `$NAME` or `${NAME}` are also
exempt from assignment-value rules. The allowlist does not use substring
matching: `production-test-token-123` is not exempt. Vendor-prefix fixtures use
synthetic values that cannot authenticate and are isolated in controlled test
fixtures. Public documentation shows prefixes with an ellipsis, not a
token-shaped body.

There is no generic allowlist for `localhost`, hashes, UUIDs, issue IDs, or text
containing the word `test`. Benign SHA-256 strings, UUIDs, and commit hashes are
accepted unless a separate contextual rule, such as `token=<hash>`, applies.

## 6. Decision matrix

| Finding | `balanced` | `strict` | Fully authorized category override |
|---|---|---|---|
| none | append with `clean` audit | append with `clean` audit | supplied category flags are rejected as unused |
| medium only | append with `warn` audit | refuse | append with `override` audit |
| any high | refuse | refuse | append only when every refusing category is covered |
| high plus medium | refuse on high; record all categories safely | refuse on all | append only when every refusing category is covered; balanced-mode uncovered medium remains `warn` |
| unknown/invalid override category | exit 2 before append | exit 2 before append | not applicable |
| oversize/invalid UTF-8 | exit 65 before scan | exit 65 before scan | override cannot bypass validation |

Policy warnings are not printed as prose containing the input. Their diagnostic
portion reports only policy version, decision, sorted categories, and sorted
field names. The normal success record still contains accepted record content.

## 7. Record and output audit contract

Every new cut and resolve event stores a `content_policy` object:

```json
{
  "version": 1,
  "mode": "balanced",
  "decision": "clean|warn|override",
  "categories": ["email_address"],
  "fields": ["text"]
}
```

Rules:

- categories are sorted and deduplicated;
- fields use the stable names `text`, `tag`, `agent`, and `resolution_note`, and
  are sorted and deduplicated;
- `clean` has empty category and field arrays;
- `warn` records every medium category observed;
- `override` records every observed category, not which pattern matched;
- `content_policy` is excluded from the content-addressed cut ID;
- the resolution projection carries the resolve event's policy audit alongside
  its note;
- contract-1 events with no object remain valid and mean `legacy-unscanned`;
- readers never rewrite old events merely to add an audit object.

Because policy metadata does not affect the ID, fixed content retains the same
ID across profiles and policy modes. Scanner decision occurs before duplicate
lookup. A strict command therefore refuses suspect input even if an identical
legacy or balanced record already exists; it must not return and echo the old
record as a duplicate.

This record and envelope expansion requires machine contract version 2 or
higher. Upstream v0.1 serde readers ignore the unknown cut field, but semantic
compatibility requires the hardened schema and mixed-journal tests. The
consolidated ADR owns the final version number.

## 8. Refusal and redacted diagnostics

Add the public error code:

```text
sensitive_input -> exit 65, retryable false
```

Representative error:

```json
{
  "ok": false,
  "error": {
    "code": "sensitive_input",
    "message": "input matched the sensitive-data guardrail",
    "details": {
      "policy_version": 1,
      "policy": "strict",
      "categories": ["credential_url"],
      "fields": ["text"]
    },
    "retryable": false,
    "suggested_fix": "Replace sensitive values with a non-sensitive description, then retry. Review the original value outside Papercuts."
  },
  "meta": {"contract": 2}
}
```

No refusal, policy warning metadata, debug output, panic message, benchmark, or
retained failure artifact may contain:

- the matching value or substring;
- surrounding context;
- line, column, byte offset, match length, or encoded form;
- a hash or fingerprint of the value;
- the regex or catalog pattern that matched;
- raw filesystem targets or OS error strings.

The scanner returns category and field enums, never borrowed match text. Its
debug representation must be safe by construction. Tests assert absence using
sentinel fragments from synthetic fixtures across stdout, stderr, target
journal, private state directories, and retained artifacts.

An accepted `warn` or deliberately authorized `override` is different: the
original caller input is record content and is therefore persisted and may
appear in the normal success record. Policy metadata never repeats it. This is
why high-confidence matches refuse by default and why documentation must not
describe balanced warnings as redaction.

## 9. Evaluation and side-effect order

For `add` and `resolve`, the required order is:

```text
parse CLI
resolve and validate profile, read-only guard, policy, and override controls
deny an actual mutation when read-only is active
resolve the logical storage target with read-only metadata inspection
if private storage requires an explicit target: return storage_required
read bounded command input (stdin where applicable)
validate UTF-8, field counts, per-field sizes, and total size
scan caller-controlled fields and compute the policy decision
if refusal: return sensitive_input
only then create/open/lock/read the journal
construct clock-dependent event and ID where applicable
append or return dry-run projection
```

Logical target resolution may inspect the current directory and validated Git
metadata, but must not create a directory or journal. Outside-Git private
storage requirements remain an earlier policy prerequisite: if no writable
target can exist under the storage ADR, return `storage_required` without
consuming stdin. Once a target is logically available, scanning occurs before
any filesystem creation, journal open, lock, journal read, timestamp-dependent
record construction, or duplicate lookup.

`resolve` validates and scans the note and agent before opening the journal or
looking up an ID. The ID prefix itself is validated but is not a scanner input.
List filters do not append their values and are outside this ingestion guard.

### 9.1 Dry run

Dry run executes the same validation and scanner decision:

- a refusing input returns `sensitive_input`, exit 65;
- a balanced medium-risk input returns success with `changed:false`, `warn`
  audit, and no file creation;
- a fully authorized override returns success with `changed:false`, `override`
  audit, and no file creation;
- dry run never weakens policy; a refusal does not expose the value, while an
  accepted warning or override returns the normal caller-supplied record just as
  a non-dry-run success does.

This makes dry run useful for policy validation without turning it into a bypass.

## 10. Determinism, offline behavior, and performance

The implementation should use a pure policy module with typed inputs and typed,
category-only output. Patterns are static literals, compiled once per process,
and capped at 128 entries in policy version 1. Runtime configuration cannot add
patterns.

If the Rust `regex` crate is used, pin it through `Cargo.lock`, apply an explicit
compiled-size limit, and use `RegexSet` or single-match APIs rather than
unbounded all-match iteration. The crate documents `O(m * n)` worst-case search
for fixed patterns and untrusted haystacks; the input and pattern caps bound both
factors. See the [regex crate untrusted-input guidance](https://docs.rs/regex/latest/regex/#untrusted-input).

Correctness tests enforce:

- at most 128 compiled patterns;
- at most 16,384 scanned bytes per command;
- no network, subprocess, filesystem, clock, locale, or random dependency in
  the pure scanner;
- identical category output for identical UTF-8 bytes and policy version.

A release-mode benchmark scans the maximum payload against the full catalog for
at least 10,000 iterations and records host, binary SHA, p50, p95, and maximum.
The initial acceptance budget is p95 at or below 5 ms and maximum at or below
20 ms on the recorded release-gate host. The benchmark is release evidence, not
a flaky wall-clock unit-test assertion. Exceeding the budget blocks release
until the catalog or implementation is reviewed.

## 11. Synthetic adversarial corpus

All fixtures use unmistakably synthetic, non-authenticating values. Each case
asserts decision, sorted category set, persisted audit where accepted, absence
of side effects where refused, and absence of sentinel fragments in outputs.

### 11.1 Required positive cases

| Family | Cases |
|---|---|
| authorization | mixed-case Bearer and Basic headers; quoted shell/curl forms; placeholder exemption |
| vendor tokens | every catalog prefix; short/incomplete prefix non-match; documented sample handling |
| AWS | `AKIA` and `ASIA` ID alone allowed; paired secret refused; pair split across two caller-controlled fields refused |
| private keys | PKCS, RSA, EC, OpenSSH, and PGP begin/end markers; multiline blocks |
| URLs | HTTP, HTTPS, SSH, and database URI with credentials; username-only and placeholder-password controls |
| assignments | YAML, dotenv, shell `export`, JSON-like, quoted/unquoted values, `$VAR` exemption |
| identifiers | email; labelled phone, customer, patient, user, and account IDs |
| paths | Unix home/common absolute path, macOS user path, drive path, UNC path; relative-path control |
| config | two-line assignment block; single benign assignment control; secret line escalates high |
| fields | text, each tag, explicit agent, environment agent, resolution note, and cross-field pair rule |
| modes | private balanced warning, private strict refusal, committed strict floor, attempted downgrade error |
| override | env only, flags only, unknown category, partial coverage, duplicate category, full coverage, no wildcard |
| dry run | clean, warning, refusal, and override with zero file/directory writes |

### 11.2 Required false-positive and boundary cases

- SHA-1 and SHA-256 hashes, Git commit IDs, UUIDs, Bead IDs, and issue IDs;
- the exact placeholder list in every supported assignment delimiter;
- values merely containing `test`, `example`, or `redacted`;
- `$TOKEN` and `${TOKEN}` references versus literal assigned tokens;
- `pk_test_` and `pk_live_` publishable Stripe keys;
- `AKIA`/`ASIA` access-key IDs without a paired secret;
- shell snippets with relative paths and one ordinary assignment;
- Unicode prose, emoji, combining marks, homoglyph lookalikes, and invalid UTF-8
  stdin;
- CRLF and LF multiline input;
- exact byte limits and one byte over each limit;
- sixteen tags, seventeenth tag, duplicate tags, and total payload boundary;
- findings in multiple fields with stable sorted/deduplicated categories;
- deterministic repeat runs under changed locale, working directory, and clock.

### 11.3 No-echo and no-write assertions

For every refusing fixture, generate a unique synthetic sentinel inside the
suspect value and assert the sentinel is absent from:

- stdout and stderr;
- the selected journal and its parent if they did not exist before;
- existing journal bytes and line count;
- diagnostic structures and debug formatting;
- test failure artifacts and benchmark reports.

Tests may retain controlled fixture source literals in the test corpus. Failure
reporting names only the fixture ID and expected category.

## 12. Known misses and residual risk

Policy version 1 intentionally does not:

- calculate generic Shannon entropy or classify arbitrary random-looking text;
- decode Base64, hexadecimal, URL encoding, JSON escapes, compression, archives,
  QR codes, or nested encodings;
- normalize Unicode or detect homoglyph-obfuscated labels and prefixes;
- detect an unmarked private-key body or a non-paired signature deliberately
  split across separate fields;
- recognize every vendor, private token format, national identifier, phone
  number, email form, connection string, or private hostname;
- inspect referenced files, clipboard contents, terminal history, existing
  journals, Git history, environment variables other than persisted agent and
  policy controls, or data written by another program;
- redact or rewrite accepted text;
- sanitize legacy events during list, resolve, doctor, migration, or digest.

An encoded credential can therefore pass. An unrecognized literal can pass. A
legacy journal may already contain secrets. Medium-risk data accepted under
balanced mode remains present in private storage and backups. A deliberate
override persists the original input. These are reasons for human review, not
reasons to make the scanner broader and less explainable without a new policy
version.

Read paths report `legacy-unscanned` honestly; they do not retroactively label
old content clean. The later multi-project digest must exclude `warn`,
`override`, and `legacy-unscanned` records by default or require a separate
explicit operator choice. That aggregation decision remains owned by its bead.

## 13. Compatibility, migration, and rollback

This is a behavioral break from upstream v0.1, which accepts any non-empty UTF-8
text up to 10,000 bytes and has no equivalent bounds for agent names, tags, or
resolution notes.

Compatibility rules:

- committed storage preserves upstream locations and paths, not unchecked
  ingestion;
- there is no silent legacy mode that disables the scanner;
- contract-1 journal lines remain readable and append-only;
- new audit fields are additive JSON fields but require contract-2 semantics;
- policy is applied only to a new attempted append, never by rewriting history;
- rejected inputs create no migration, backup, directory, or journal;
- release notes call out new refusals and size limits.

Rollback is selection-only:

1. stop using the hardened binary;
2. retain journal bytes unchanged;
3. use the exact upstream v0.1 binary only with explicit acceptance that it has
   no content guardrail and does not understand the audit semantics;
4. do not strip audit fields, rewrite records, or force-push public history.

No product flag will recreate fully unchecked upstream ingestion inside the
hardened binary. An emergency need to persist intentionally sensitive evidence
belongs in an approved secret-management or incident system, not Papercuts.

## 14. Rejected alternatives

### Warn only

Rejected because a tracked journal can be pushed before a warning is reviewed.
High-confidence categories require a refusal.

### Refuse every finding in every profile

Rejected as the only mode because emails, paths, and configuration-like prose
are common in useful private workflow complaints. Balanced mode preserves that
signal with an audit while committed storage remains strict.

### Generic entropy threshold

Rejected for version 1 because hashes, UUIDs, generated IDs, compressed data,
and ordinary opaque identifiers create poorly explainable false positives, while
low-entropy passwords still evade it.

### Decode likely encodings recursively

Rejected because it expands CPU and memory cost, creates ambiguous provenance,
and still cannot prove safety. Encoded material is an explicit limitation.

### Run Gitleaks or fetch a cloud pattern service on every add

Rejected because logging must work offline, remain bounded, and avoid
subprocess/network availability, version, output, and privacy drift. Gitleaks
remains a repository release gate, not an ingestion dependency.

### One `--force` or `--allow-sensitive all`

Rejected because it is easy for an agent to cargo-cult and impossible to audit
by category. Overrides must be exact and two-key.

### Echo, mask, hash, or show the matching fragment

Rejected because partial masking, hashes, positions, and context can still leak
or correlate the suspect value. Category and field enums are sufficient.

### Scan after storage discovery or duplicate lookup

Rejected because a refused input must not create state or cause an old suspect
record to be returned. Preflight precedes the append store path.

## 15. Implementation and review gates

Implementation may begin only after the consolidated hardened-contract ADR
reconciles this policy with storage, path projection, schema version, and release
naming.

The implementation bead must provide:

- pure versioned scanner module and bounded input reader;
- exact CLI/env precedence and profile floor;
- category-specific two-key override;
- contract-2 audit fields and `sensitive_input` error;
- synthetic unit corpus and black-box no-echo/no-write tests;
- release-mode performance evidence;
- schema/help/docs generated from shared category constants where practical;
- full Cargo gate and scoped UBS results.

Fresh-context review must answer:

1. Can any refusing path echo, hash, persist, or debug-print suspect bytes?
2. Can any flag, environment value, profile, dry run, duplicate, or old record
   weaken the profile floor implicitly?
3. Are all scanned inputs bounded before I/O and before pattern work?
4. Do placeholder rules remain exact rather than substring-based?
5. Are benign hashes and vendor publishable keys still accepted?
6. Are known misses visible enough to prevent a false security promise?
7. Can a contract-aware reader distinguish clean, warning, override, and
   legacy-unscanned history?
8. Do downstream beads contain the exact decisions they need without relying
   on chat history?
