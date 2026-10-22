---
name: gen-commit
description: Generate an English Conventional Commit message from Git changes without staging or committing. Use when the user invokes $gen-commit or asks for a commit message.
---

# Generate Commit Message

Generate text only. Do not stage files, commit, amend, push, or change Git configuration.

## Inspect changes

1. Read `git status --short`, `git diff --staged`, and `git diff`. Status also reveals untracked files, which neither diff includes. Inspect relevant untracked text files before describing them; do not read secret files such as `.env`, credentials, or private keys, or reproduce sensitive values in messages.
2. Honor the user's requested files or scope. Otherwise, use staged changes when present; if nothing is staged, use unstaged changes and relevant untracked files. Do not silently include unstaged edits in a message for staged changes, including when the same file has both.
3. If the selected scope has no changes, report that briefly and stop. If conflicts remain in that scope, report them and stop rather than inventing a completed change.
4. Read `git log --oneline -10` when history exists to match established scope and wording conventions. Base all claims on the selected changes; do not infer implementation, verification, or issue numbers from filenames or history alone.

## Write the message

```text
<type>[optional scope][!]: <short summary>

[optional body]

[optional footer(s)]
```

- Choose `type` from `feat`, `fix`, `docs`, `refactor`, `style`, `test`, `chore`, `perf`, `ci`, `build`, or `revert` according to the actual change.
- Use an English imperative summary, starting lowercase. Aim for 50 characters for the full subject; keep it under 72 characters. Add a scope only when it identifies a meaningful area, following repository conventions.
- Mark an actual breaking change with `!` and explain the incompatibility and required migration in a `BREAKING CHANGE:` footer. Include that footer even without detail flags.
- When the user includes `--detail`, `-d`, `--verbose`, or `-v`, append a blank line and `- ` bullets explaining meaningful changes and their reasons. Do not invent motivation or claim checks that were not run.
- Use issue references only when supplied or verified; use `Closes` only when the change actually resolves the issue.
- For unrelated changes in the selected scope, output separate complete messages grouped by logical change. Do not imply that files have been staged or split into commits.

Return only the requested message or messages, without commentary or code fences. The no-change and conflict reports above are exceptions.

## When the user asks to commit

This skill does not execute commits. If the user explicitly asks to create the commit, hand off to the installed `git-commit` skill when available: discover it from the current skill catalog and read its `SKILL.md` only then. Do not hard-code a local installation path or assume a Markdown link automatically loads another skill.

The handoff applies only to the explicitly requested commit and files; it does not authorize unrelated staging or pushing. If `git-commit` is unavailable, say so and provide the generated message without attempting its workflow. Ordinary message generation requires no other skill.
