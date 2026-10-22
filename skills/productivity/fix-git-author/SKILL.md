---
name: fix-git-author
description: Correct the author and committer of the latest Git commit and, when authorized, update its remote branch safely. Use when the latest commit has the wrong name or email, including requests to reset its author or repair an already pushed commit. Do not use for bulk history rewrites.
---

# Fix Git Author

Repair only the latest commit. Preserve its message, tree, and all staged and unstaged work.

## 1. Inspect the target

- Read repository instructions and inspect `git status --short`, the current branch, remotes, upstream, and `git log -1 --format=fuller`.
- Check repository-local configuration and effective identities with `git var GIT_AUTHOR_IDENT` and `git var GIT_COMMITTER_IDENT`. Environment variables can override Git configuration.
- Use an identity supplied by the user. Otherwise use this skill's default: `Mt-Youya <dd257248@163.com>`. Set it only in this repository:

  ```sh
  git config --local user.name 'Mt-Youya'
  git config --local user.email 'dd257248@163.com'
  ```

  Substitute both values when the user specifies another identity. Verify both effective identities again; resolve conflicting environment overrides before amending.
- Stop if there is no commit or a merge, rebase, or cherry-pick is in progress. Resolve an ambiguous repository or target with the user. A local repair can run on detached HEAD, but publishing requires an explicit destination branch.

## 2. Record and repair

Record the original commit ID, tree ID, full message, and staged diff. If publishing is authorized, resolve the exact remote and destination ref and record its current object ID with `git ls-remote` before rewriting. For this latest-commit repair, an existing remote branch must point to the original commit; stop on divergence instead of overwriting unrelated history.

Run:

```sh
git commit --amend --only --reset-author --no-edit
```

`--only` without paths keeps staged changes out of the amended commit. `--reset-author` resets authorship and its timestamp; the committer comes from the effective identity. Keep normal hooks enabled. If a hook fails, diagnose the failure instead of silently bypassing it.

Verify the new author and committer, unchanged tree and message, and unchanged staged diff. Preserve unstaged and untracked files as well. If both identities are already correct, skip the rewrite to avoid needless commit-ID changes.

## 3. Publish within the authorized scope

An explicit request to execute this skill includes publishing to the established upstream. Honor narrower requests such as "local only". For other requests, push only when the user has authorized it. Do not guess a destination when there is no established upstream.

For an existing remote branch, use the object ID recorded before the amend:

```sh
git push --force-with-lease=refs/heads/<branch>:<recorded-remote-oid> <remote> HEAD:refs/heads/<branch>
```

Use the actual destination branch, which may differ from the local branch name. The explicit lease rejects concurrent remote changes. If rejected, inspect and report the new remote state; do not retry with a refreshed lease or plain `--force`.

For an explicitly authorized new remote branch, use a normal push. Read back the destination with `git ls-remote` and compare it to local HEAD before reporting success. A failed push leaves a local repair, not a completed remote update.

## Report

Report the old and new commit IDs, final author and committer, whether existing work was preserved, and whether the remote update was verified, skipped, or failed. State that rewriting a published commit changes its ID and collaborators may need to synchronize.
