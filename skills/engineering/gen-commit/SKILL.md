---
name: gen-commit
description: Generate an English Conventional Commit message from the current Git staged or unstaged changes. Use when the user invokes $gen-commit or asks for a commit message without committing.
---

# Generate Commit Message

Generate text only. Never run `git commit`.

1. Run `git diff --staged` and `git diff`. If both are empty, report that there are no changes and stop.
2. Run `git log --oneline -10` to match the repository's existing style.
3. Analyze staged and unstaged changes. If only one has content, analyze that set. If both have content, distinguish their scopes; when they cannot be represented honestly by one message, provide separate candidates.
4. Produce an English Conventional Commit message:

   ```text
   <type>: <short summary>
   ```

   Choose `type` from `feat`, `fix`, `docs`, `refactor`, `style`, `test`, `chore`, `perf`, `ci`, or `build`. Use an imperative, lowercase summary of no more than 50 characters.

5. If the user includes `--detail`, `-d`, `--verbose`, or `-v`, append a blank line followed by `- ` bullets that state each change and its reason.

When the changes are unrelated, offer multiple complete candidate messages. Do not add commentary beyond the requested message or messages.
