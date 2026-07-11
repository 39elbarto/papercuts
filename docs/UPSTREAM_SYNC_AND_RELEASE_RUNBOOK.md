# Upstream Sync, Contribution, and Release Runbook

Bead: `br-hardened-papercuts-fork-x30.6`

Decision date: 2026-07-11

Maintainer: fork maintainer or explicitly delegated release operator

Applies to: `39elbarto/papercuts`

Upstream: `treygoff24/papercuts`

## 1. Purpose and operating rule

This runbook keeps the public fork auditable while allowing two different kinds
of work:

1. small, generic corrections that should be offered to upstream; and
2. fork-only safety policy and ACFS workflow integration.

The governing rule is: **never rewrite published history to make the fork look
closer to upstream**. Fetch upstream, integrate it on a review branch, run the
gate, and merge it through a fork pull request. If a published integration is
wrong, revert it with another commit.

The GitHub repository remains a real fork named `papercuts`. The upstream MIT
license, copyright notice, and clear README attribution remain in every source
distribution. Fork release notes must say that the release is community
maintained and is not an upstream release.

## 2. Current evidence and compatibility matrix

Verified on 2026-07-11:

| Reference | Immutable revision | Product relationship | Distribution status |
|---|---|---|---|
| Upstream `v0.1.0` | `5d8b827abbd054f5f506d26be865f5b7f573a298` | contract 1 release baseline | GitHub release and crates.io `papercuts` 0.1.0 |
| Upstream `main` | `ffba2bd453ab0faeadf4f923fc727586958c8d6f` | `v0.1.0` plus upstream's first dogfood journal event | no newer upstream release |
| Fork baseline before this decision | `6e7dd774778866821e6969779772d02e18d572c1` | product code identical to `upstream/main`; fork-only docs, plan, and Beads added | no fork release |
| First hardened prerelease | not yet cut | may change defaults, accepted input, paths, and the machine contract | minimum version line `0.2.0-alpha.1`; release is gated below |

At the decision snapshot, `origin/main` was four commits ahead of and zero
commits behind `upstream/main`. Their merge base was exactly
`ffba2bd453ab0faeadf4f923fc727586958c8d6f`.

The official sparse crates.io index contains package `papercuts` version 0.1.0,
published on 2026-07-10. Its package metadata points at the upstream repository.
The crates.io API was unavailable with HTTP 503 during this check, so the live
owners list could not be read. This does not create ambiguity in our policy:
the namespace is occupied and this fork has no verified publication authority.

Update this matrix whenever an upstream tag, upstream contract, fork release,
or package identity changes. Record exact full SHAs, not only branch names.

## 3. Remote and branch contract

The only valid remote roles are:

| Remote | URL | Allowed use |
|---|---|---|
| `origin` | `git@github.com:39elbarto/papercuts.git` | fetch and push fork branches |
| `upstream` | `git@github.com:treygoff24/papercuts.git` | fetch only |

Configure a no-push sentinel once per checkout:

```bash
git remote set-url --push upstream DISABLED
git remote -v
```

An attempted `git push upstream ...` must then fail before any GitHub write.
Upstream contributions are pushed to `origin` and opened as cross-repository
pull requests.

Branch roles:

- `main`: public fork history; never rebase, reset, or force-push it;
- `sync/upstream-YYYY-MM-DD`: one upstream integration attempt;
- `contrib/upstream-SLUG`: a generic change based directly on
  `upstream/main` for an upstream PR;
- `fork/BEAD-SLUG`: fork-only product or workflow work.

Do not mix an upstream integration, a generic upstream fix, and a fork-only
policy change in one commit or pull request. Upstream sync pull requests use a
merge commit so the integration boundary and rollback parent are explicit.

## 4. Change classification

### 4.1 Offer upstream

A change is an upstream candidate only when all of these are true:

- it applies to upstream users without ACFS, ClickUp, Beads, CM, or local host
  assumptions;
- it is independently testable and does not depend on a fork-only commit;
- it preserves contract 1, or the upstream maintainer has explicitly agreed to
  review a versioned contract change;
- it contains no private path, runtime evidence, customer data, or fork project
  administration;
- one focused commit or a small coherent series can explain it.

Current generic candidates are:

- prominent documentation of committed free-text and absolute-path risk;
- an accurate `rust-version` plus MSRV documentation and CI coverage;
- removal or repository-relative replacement of the dangling absolute skill
  symlink.

### 4.2 Keep fork-only

The following remain fork decisions unless upstream explicitly accepts their
behavioral contract:

- safe/private storage as the default;
- omission, normalization, or replacement of required path fields;
- sensitive-input warning or refusal policy;
- strict repository validation that changes `.git.exists()` discovery;
- ACFS multi-project review and promotion adapters.

Generic core code and ACFS adapters must live in separate commits and, where
possible, separate directories. An adapter must not be required to build or
test the generic Rust CLI.

## 5. Routine upstream preflight

Trigger this procedure before a fork release, before a large product slice, and
at least once during any month with active development.

Start at the canonical checkout. Do not automatically stash a dirty worktree:

```bash
cd /data/projects/papercuts
test "$(git rev-parse --show-toplevel)" = "$PWD"
git status --short --branch
git remote get-url origin
git remote get-url upstream
git remote get-url --push upstream
gh repo view 39elbarto/papercuts --json nameWithOwner,isFork,parent,defaultBranchRef
gh repo view treygoff24/papercuts --json nameWithOwner,isFork,defaultBranchRef,latestRelease
```

Required results:

- the worktree is clean;
- the current public branch is tracking `origin/main`;
- GitHub still reports `39elbarto/papercuts` as a fork of
  `treygoff24/papercuts`;
- upstream push URL is `DISABLED`;
- no credentials or tokens appear in remote URLs.

If any check fails, stop and repair the checkout before fetching or merging.

Fetch and record the relationship:

```bash
git fetch origin --prune --tags
git fetch upstream --prune --tags
git rev-parse origin/main upstream/main
git merge-base origin/main upstream/main
git rev-list --left-right --count origin/main...upstream/main
git log --left-right --graph --oneline --decorate origin/main...upstream/main
```

For `git rev-list --left-right --count A...B`, the first number is commits only
on `A` and the second is commits only on `B`. Save the full SHAs and counts in
the pull request or worklog evidence.

## 6. Integrate upstream without rewriting history

### 6.1 Create the integration branch

```bash
git switch main
git pull --ff-only origin main
git switch -c sync/upstream-YYYY-MM-DD
git merge --no-ff upstream/main
```

If Git reports `Already up to date`, do not create an empty commit. Record the
SHAs and end the sync attempt.

If upstream is ahead, the merge commit must contain upstream integration only.
Do not fix unrelated fork behavior while resolving the merge.

### 6.2 Resolve ordinary conflicts

Inspect before editing:

```bash
git status --short
git diff --name-only --diff-filter=U
git log --merge --oneline
```

For each conflict, preserve both sides' documented intent. In particular:

- keep the upstream MIT license and copyright notice;
- keep the fork attribution and implemented-versus-planned warning;
- do not resolve `Cargo.toml`, schema, journal, or storage conflicts by choosing
  an entire side without reviewing the contract diff;
- do not edit `.papercuts.jsonl` into a synthesized history; retain complete
  JSONL events and validate the journal after the merge.

After resolving only the listed files:

```bash
git add path/to/resolved-file ...
git diff --cached --check
git merge --continue
```

### 6.3 Stop conditions for abnormal divergence

Stop without merging if any of these occurs:

- `git merge-base` returns no commit;
- upstream history was force-updated past the last recorded upstream SHA;
- a tag name points to different objects locally and upstream;
- conflict resolution would silently change contract version, default storage,
  accepted input, record shape, or exit semantics;
- the upstream release contains an unexplained binary or generated artifact.

Do not use `--allow-unrelated-histories`, `git reset --hard`, tag overwrite, or
force-push as recovery. Preserve the failed branch, record the old and new SHAs,
inspect the upstream GitHub event/release, and open a dedicated compatibility
Bead before continuing.

### 6.4 Verify and publish the sync pull request

Run the complete inherited product gate even for an apparently documentation-
only upstream merge:

```bash
cargo build --release
cargo test --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --check
cargo run --quiet -- doctor
git diff --check origin/main...HEAD
git log --oneline --decorate origin/main..HEAD
```

If storage or concurrency code changed, run the test suite five times and run a
scoped UBS check. Then push only the sync branch and open a fork PR:

```bash
UPSTREAM_SHORT_SHA="$(git rev-parse --short upstream/main)"
PR_BODY_FILE=path/to/prepared-public-pr-body.md
git push -u origin sync/upstream-YYYY-MM-DD
gh pr create \
  --repo 39elbarto/papercuts \
  --base main \
  --head sync/upstream-YYYY-MM-DD \
  --title "sync: merge upstream through ${UPSTREAM_SHORT_SHA}" \
  --body-file "$PR_BODY_FILE"
```

The PR evidence must include old/new upstream SHAs, ahead/behind counts,
conflict decisions, commands and results, journal doctor result, and any
remaining contract risk. Merge this PR with a merge commit, not squash or
rebase.

## 7. Prepare a generic upstream contribution

Start from current upstream, not fork `main`:

```bash
SLUG=msrv-declaration
BRANCH="contrib/upstream-${SLUG}"
git fetch upstream --prune --tags
git switch --detach upstream/main
git switch -c "$BRANCH"
```

Implement one generic concern, add its focused tests, and run the full product
gate. Confirm that no fork-only commits leaked in:

```bash
git log --oneline upstream/main..HEAD
git diff --stat upstream/main...HEAD
git diff --check upstream/main...HEAD
```

Push to the fork and open the cross-repository PR:

```bash
PR_TITLE="docs: declare the supported Rust version"
PR_BODY_FILE=path/to/prepared-public-pr-body.md
git push -u origin "$BRANCH"
gh pr create \
  --repo treygoff24/papercuts \
  --base main \
  --head "39elbarto:${BRANCH}" \
  --title "$PR_TITLE" \
  --body-file "$PR_BODY_FILE"
```

The PR must explain the upstream user problem, compatibility impact, tests, and
any migration requirement. It must not mention private infrastructure as
supporting evidence.

### Accepted, rejected, or inactive upstream work

- **Accepted:** record the PR URL and merge SHA, fetch upstream, and integrate
  it with the normal sync procedure. Remove any now-duplicate fork patch in a
  normal review commit.
- **Changes requested:** add follow-up commits; do not force-push a reviewed
  public branch unless the upstream maintainer explicitly asks for it.
- **Rejected or closed:** record URL, date, and rationale in the relevant Bead.
  Do not disguise the same change in a new upstream PR. If the fork still needs
  it, cherry-pick the focused commit onto a `fork/...` branch and document it as
  a carried patch.
- **No response:** keep the contribution branch intact. The fork may carry the
  isolated patch after a documented review deadline, but release notes must
  identify it as fork-only and link the upstream attempt.

## 8. Package, binary, tag, and registry policy

### 8.1 Name retained during development and isolated pilot

The GitHub fork keeps the repository name `papercuts`. During planning,
implementation, and an exact-revision pilot, the Cargo package and binary may
also remain `papercuts` to minimize mechanical diffs and preserve upstream test
coverage.

Because `papercuts --version` is then ambiguous, pilot installations must be
pinned to a full fork SHA, installed into an isolated root, and invoked by
exact path:

```bash
FORK_SHA="$(git rev-parse HEAD)"
PILOT_ROOT="$HOME/.local/opt/papercuts-fork/${FORK_SHA}"
cargo install \
  --git https://github.com/39elbarto/papercuts \
  --rev "$FORK_SHA" \
  --root "$PILOT_ROOT" \
  --locked \
  --force
"$PILOT_ROOT/bin/papercuts" --version
```

Do not put that `bin` directory ahead of an upstream `papercuts` installation
on global `PATH`. Record the full SHA in pilot evidence.

### 8.2 Mandatory rename boundary

Before the first non-isolated public binary/package distribution, rename both
the Cargo package and installed binary if the fork still has any fork-only
default, record-shape, input-acceptance, discovery, or contract behavior. The
release Bead must choose a distinct name, recheck GitHub and package registries
at that time, update help/schema/install docs, and prove coexistence with the
upstream `papercuts` binary.

The package and binary may keep the name `papercuts` beyond the isolated pilot
only if the relevant behavior is accepted upstream and released by upstream,
or if both namespace authority and contract compatibility are explicitly
documented. Repository ownership and apparent package-name availability are not
publication authority.

### 8.3 Version and tag line

- No fork release is cut while `Cargo.toml` still reports upstream `0.1.0`.
- A prerelease containing changed defaults or machine contract starts no lower
  than `0.2.0-alpha.1`; the ADR decides the exact contract number.
- Fork GitHub tags use the `hardened-v` prefix plus the version so they cannot
  be confused with upstream tags, for example `hardened-v0.2.0-alpha.1`.
- Never move or overwrite an upstream or published fork tag.
- Upstream-compatible fixes should normally be offered upstream instead of
  creating a competing fork patch release.

### 8.4 Registry prohibition

Do not run `cargo publish`, create a crates.io token for this project, or add a
registry release workflow for the package name `papercuts`. The namespace is
already occupied by upstream 0.1.0 and this fork has no verified ownership.

A future renamed package may be published only after the release Bead records:

- the chosen namespace and current owners/availability from the official
  registry;
- explicit authorization for the publishing identity;
- a dry-run package review and secret scan;
- package contents including `LICENSE` and attribution;
- a tested rollback/yank procedure;
- user approval for the external publication.

Until then, fork releases are GitHub source/prerelease artifacts only.

## 9. Release-note contract

Every fork release begins with language equivalent to:

> This is a community-maintained fork of `treygoff24/papercuts`, based on
> upstream `UPSTREAM_TAG` (`UPSTREAM_FULL_SHA`). It is not an upstream release.

Release notes must then state:

- the exact fork commit and upstream base;
- whether package/binary names differ from upstream;
- implemented changes only, clearly separating planned work;
- changed defaults, record/schema/exit compatibility, and migration path;
- carried patches and their upstream issue/PR outcomes;
- supported installation route and exact verification commands;
- test, secret-scan, journal doctor, and release-gate results;
- known limitations and rollback instructions;
- the MIT license and upstream attribution.

Do not call a build “hardened”, “secure”, or production-ready before the
corresponding project release gates pass. “Hardened” describes verified
guardrails, not a guarantee that sensitive data can never be disclosed.

## 10. Rollback and recovery

### Before a merge commit exists

```bash
git status --short
git merge --abort
git switch main
```

Keep the failed branch until the conflict evidence is recorded. Do not delete
or reset it as an automatic cleanup step.

### After a local merge commit but before push

Do not rewrite `main`; the integration is on a disposable sync branch. Switch
back to `main`, record the failed branch name and SHA, and either correct it
with additional commits or create a fresh dated sync branch after review.

### After a sync merge reached public `main`

Create a rollback PR; never force-push or reset the public branch:

```bash
: "${SYNC_MERGE_SHA:?set SYNC_MERGE_SHA to the reviewed upstream sync merge}"
git switch main
git pull --ff-only origin main
git switch -c rollback/upstream-YYYY-MM-DD
git show --no-patch "$SYNC_MERGE_SHA"
git revert -m 1 "$SYNC_MERGE_SHA"
cargo test --all-features
git push -u origin rollback/upstream-YYYY-MM-DD
```

Open a fork PR containing the failed verification, merge SHA, revert result,
and follow-up Bead. If a release was already published, keep its tag immutable,
mark it affected in GitHub release notes, and publish a new corrective version.
Do not silently replace artifacts.

## 11. Evidence and completion checklist

Store durable evidence in the relevant Bead, fork PR, release notes, and
`docs/WORKLOG.md`. Do not commit raw credentials, private logs, or volatile
local state.

A sync or release slice is complete only when all applicable items are true:

- [ ] origin, upstream, parent-fork relationship, and no-push upstream remote verified;
- [ ] old/new full SHAs and ahead/behind counts recorded;
- [ ] change classified as generic upstream, fork-only, or sync-only;
- [ ] conflicts and contract decisions documented;
- [ ] build, tests, clippy, formatting, and journal doctor passed;
- [ ] concurrency repetition and UBS run when their code triggers apply;
- [ ] MIT license and upstream attribution preserved;
- [ ] package/binary name gate evaluated;
- [ ] no unverified registry namespace publication attempted;
- [ ] rollback command and evidence location recorded;
- [ ] public history changed only through normal commits and pull requests.
