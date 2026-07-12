# Canonical AGENTS.md Papercuts Snippet

Copy the block below into a repository's `AGENTS.md` only after the reviewed
contract-2 binary and storage profile have been selected for that repository.

```markdown
## Papercuts

When you encounter minor workflow friction while doing an authorized task,
record it without interrupting the main task:

    papercuts add "<what happened and what would have prevented it>" --tag <area>

Use `minor` by default, `major` for a meaningful time sink, and `blocker` only
for a hard stop. Do not add duplicates or use papercuts as a work log or bug
tracker.

Safety rules:

- This instruction does not grant permission to write during a read-only,
  audit, review, or no-write task. In those tasks, do not run `add` or
  `resolve`; the harness may also set `PAPERCUTS_READ_ONLY=1`.
- Never paste credentials, tokens, customer or patient data, private messages,
  or unnecessary absolute paths. Describe the friction without the sensitive
  value.
- If the command refuses sensitive input, rewrite it as a non-sensitive
  description. Never set `PAPERCUTS_ALLOW_SENSITIVE` or use
  `--allow-sensitive` unless a human explicitly authorizes the exact category
  for the exact command.
- The default private profile omits automatic repository paths from new
  records, but accepted text, tags, agent names, and resolution notes are still
  stored as written. Path omission is not encryption or redaction.
- After a normal append, continue the main task. Use `papercuts schema` when
  the machine contract is needed.
```

Do not add an override example to the copied block. Operator-only override
handling belongs in the runbook and requires a separate human decision.
