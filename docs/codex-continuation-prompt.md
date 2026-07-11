# Codex Continuation Prompt

Continue the public `papercuts` fork project in `/data/projects/papercuts`.

Before acting, read:

- `AGENTS.md`
- `README.md`
- `docs/PROJECT_PLAN.md`
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

Current phase: planning and threat-model review. Do not begin broad
implementation until safe defaults, path handling, secret detection behavior,
backward compatibility, and upstream boundaries are explicitly decided in
`docs/PROJECT_PLAN.md`.

After a meaningful slice, update `docs/WORKLOG.md`. If code changes, run the
upstream Rust quality gate and a scoped UBS check before handoff.
