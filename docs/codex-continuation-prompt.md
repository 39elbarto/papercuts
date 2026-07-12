# Codex Continuation Prompt

Continue the public `papercuts` fork project in `/data/projects/papercuts`.

Before acting, read:

- `AGENTS.md`
- `README.md`
- `docs/PROJECT_PLAN.md`
- `docs/PILOT_PLAN.md`
- `docs/PILOT_STATUS.md`
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

Current phase: the 14-day allowlisted pilot in
`br-hardened-papercuts-fork-x30.15`. Contract 2 and the single-project gate are
complete. The pilot is active only for `papercuts` and `acfs-workbench` and
cannot finish before `2026-07-26T16:07:01+07:00`.

If this chat was opened from a ClickUp reminder, confirm the checkpoint date in
`docs/PILOT_STATUS.md`. Do not run an elapsed review before its gate. Use the
exact pilot binary, keep raw journals private, retain only sanitized counts and
categories, and update evidence, `docs/WORKLOG.md`, and `x30.15` after the
review. Do not widen the allowlist or begin multi-project work.

ClickUp reminders point back to the Codex chat named **PaperCuts Project**.
Repository files remain authoritative if a reminder description is stale.

After a meaningful slice, update `docs/WORKLOG.md`. If code changes, run the
upstream Rust quality gate and a scoped UBS check before handoff.
