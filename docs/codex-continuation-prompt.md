# Codex Continuation Prompt

Continue the public `papercuts` fork project in `/data/projects/papercuts`.

Before acting, read:

- `AGENTS.md`
- `README.md`
- `docs/PROJECT_PLAN.md`
- `docs/HARDENED_CONTRACT_ADR.md`
- `docs/WORKLOG.md`
- `docs/plans/2026-07-09-papercuts-design.md`

Then inspect:

```bash
git status --short
git log --oneline --decorate -8
git remote -v
br ready --json
cm context "<current papercuts task>" --json
```

Treat repository files and current Git state as the source of truth over chat
memory. This is a public fork, so do not introduce private infrastructure data,
credentials, customer data, raw private logs, or unnecessary absolute paths.

Current phase: dependency-ordered contract-2 implementation. The phase-1
architecture gate is complete; `docs/HARDENED_CONTRACT_ADR.md` is normative and
the current binary still has upstream v0.1 behavior until implementation Beads
prove otherwise. Use `br ready --json` or `bv --robot-next`; begin with the
first unblocked Bead and do not skip its dependencies or widen into deferred
multi-project work.

Every implementation Bead is self-contained. If code evidence conflicts with
the contract, stop that slice and record a narrow architecture issue instead of
silently changing the behavior.

After a meaningful slice, update `docs/WORKLOG.md`. If code changes, run the
upstream Rust quality gate and a scoped UBS check before handoff.
