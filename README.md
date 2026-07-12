# papercuts

> This repository is a public fork of
> [`treygoff24/papercuts`](https://github.com/treygoff24/papercuts). The fork is
> implementing its accepted hardened contract in dependency-ordered slices.
> The profile/storage, privacy-preserving path, and bounded sensitive-content
> preflight, contract-2 compatibility, and adversarial single-project
> acceptance surfaces and operator instructions are implemented. The
> single-project gate passed for exact SHA `804d2b1`; that SHA is eligible only
> for an isolated, allowlisted pilot—not general release or publication. See
> the live status in [`docs/PROJECT_PLAN.md`](docs/PROJECT_PLAN.md).

A tiny CLI that gives AI agents a complaint box.

Agents hit friction constantly — dead-end tool calls, broken links, missing helpers, footgun configs — and silently push through without telling anyone. The signal evaporates. `papercuts` gives an agent a one-line way to file the complaint at the moment it happens, and gives you (or another agent) a way to review the backlog and fix the actual problems in your repo, your tooling, your docs.

```
$ papercuts add "the workspace test command required the package working directory" --tag tooling
{"ok":true,"data":{"changed":true,"record":{"kind":"cut","id":"pc_...","cwd":".","repo":null,"path_policy":"omitted","path_encoding":"omitted",...}},"meta":{"contract":2,"storage_profile":"private","path_policy":"omitted",...}}
```

The idea comes from [a tool Steve Ruiz built](https://x.com/steveruizok) for his own repos: once agents had a place to complain, they immediately surfaced real workflow defects — quoting bugs, wrong test working directories, YAML footguns — that they'd been eating silently for months.

## Install

Install the reviewed fork SHA into an isolated root; do not confuse it with the
upstream registry package:

```bash
SHA=<full-reviewed-commit>
PC_ROOT="$HOME/.local/opt/papercuts-fork/$SHA"
cargo install --git https://github.com/39elbarto/papercuts \
  --rev "$SHA" --locked --root "$PC_ROOT"
PC="$PC_ROOT/bin/papercuts"
"$PC" schema | jq -e '.data.contract == 2'
```

See the [single-project runbook](docs/SINGLE_PROJECT_RUNBOOK.md) for preflight,
migration, verification, and rollback.

## How it works

Papercuts live in an **append-only JSONL file**. The hardened default is the
private profile: in Git it stores the journal under the Git common directory,
outside the worktree. Repository-visible `.papercuts.jsonl` storage is an
explicit committed compatibility lane. No server, sync, network call, or
telemetry is involved in logging. The selected journal is the product.

```bash
papercuts add "text"            # file a papercut (also: papercuts log, or pipe stdin to add -)
papercuts list                  # open papercuts, severity-first then newest, JSON envelope
papercuts list --format md      # human review digest
papercuts resolve pc_9f2c        # mark one fixed (unique ID prefix ok)
papercuts schema                # full machine contract — agents self-orient with this
papercuts doctor                # validate the log file
```

- **Agent-first contract**: stdout is data only; one JSON envelope per command; structured errors on stderr with stable codes, documented exit codes, and a paste-ready `suggested_fix`. `papercuts schema` returns the whole contract.
- **Concurrency-safe**: multiple agents on one file are fine (advisory locking, atomic appends, self-healing torn lines).
- **Deterministic**: content-addressed IDs, stable sort, reproducible-clock override for tests.
- **Never rewrites history**: `resolve` appends an event; the log is a journal, not a database.

## Give your agents the pen

Copy the reviewed [canonical `AGENTS.md` block](docs/AGENTS_PAPERCUTS_SNIPPET.md).
It preserves two boundaries that a one-line logging prompt can accidentally
erase: read-only tasks do not gain write authority, and autonomous agents may
not enable sensitive-input overrides.

Then periodically: `papercuts list --format md` and fix what your agents keep tripping over.

For a hardened single-project setup, use the copy-ready
[`AGENTS.md` snippet](docs/AGENTS_PAPERCUTS_SNIPPET.md) and the executable
[`single-project runbook`](docs/SINGLE_PROJECT_RUNBOOK.md). The default private
profile omits automatically discovered paths, but it does not redact accepted
text or grant writes during read-only work.

## Security acceptance

Contributors can run the focused contract-2 real-binary acceptance suite with:

```bash
scripts/security-acceptance.sh
```

It writes sanitized, ignored evidence under `target/security-acceptance/`.
The complete mapping from contract surfaces to test names lives in
[`docs/SECURITY_ACCEPTANCE_MATRIX.md`](docs/SECURITY_ACCEPTANCE_MATRIX.md).
This focused runner complements rather than replaces `cargo test
--all-features` and the remaining release gates.

## Team modes

**Private (default).** In a valid Git repository, the journal lives under
`<git-common-dir>/papercuts/log.jsonl` with user-only permissions. New records
omit automatically discovered path fields. Non-Git mutation requires an
explicit target. Accepted text is still stored verbatim; private is not
encryption or redaction.

**Committed (explicit exposure lane).** `.papercuts.jsonl` is a normal tracked
file, absolute legacy context is retained, and papercuts appear in diffs and
PRs. Add this to `.gitattributes` so parallel branches merge cleanly:

```
.papercuts.jsonl merge=union
```

Duplicate lines after a merge are harmless—the fold is first-wins and add is
duplicate-safe—but committed storage deliberately exposes the journal.

## Contract

Everything an agent needs is in static contract-2 `papercuts schema`: commands,
mutation annotations, profiles, precedence, bounds, known scanner limitations,
record shapes, warnings, errors, and exits. Command-relevant environment is
limited to `PAPERCUTS_PROFILE`, `PAPERCUTS_FILE`, `PAPERCUTS_READ_ONLY`,
`PAPERCUTS_SENSITIVE_POLICY`, `PAPERCUTS_ALLOW_SENSITIVE`, `PAPERCUTS_AGENT`,
and the test/reproducibility clock `PAPERCUTS_NOW`. Empty list results are exit
0; missing selected files and IDs are exit 66.

## License

MIT
