---
name: create-orphan-branch
description: Safely create an empty Git orphan branch with no inherited history. Use when the user invokes $create-orphan-branch or asks to start a branch from a clean history.
---

# Create Orphan Branch

Create a completely empty Git orphan branch that does not inherit history from any existing branch.

## Usage

```bash
/create-orphan-branch <branch-name>
```

Example:

```bash
/create-orphan-branch v2
```

## Goal

Create a new Git branch with:

- no parent commit
- no inherited commit history
- an empty working tree
- a first clean commit
- optional push to remote

## Instructions

You are helping the user create a fully independent Git branch.

The target branch name is provided by the user as the first argument.

If no branch name is provided, ask the user for the branch name before running commands.

## Safety Rules

Before deleting files, always explain that:

- `git switch --orphan <branch-name>` creates a new branch without history
- the working tree may still contain files from the previous branch
- `git rm -rf .` removes tracked files from the new orphan branch
- `git clean -fd` removes untracked files
- the user should make sure they do not need the current untracked files

Do not delete files outside the Git repository.

Do not run destructive commands until the current repository status has been checked.

## Steps

### 1. Check repository status

Run:

```bash
git status
git branch --show-current
git remote -v
```

If there are uncommitted changes, warn the user and recommend committing or stashing first:

```bash
git add .
git commit -m "save work before orphan branch"
```

or:

```bash
git stash push -u -m "save work before orphan branch"
```

### 2. Create the orphan branch

Replace `<branch-name>` with the requested branch name:

```bash
git switch --orphan <branch-name>
```

If `git switch` is unavailable, use:

```bash
git checkout --orphan <branch-name>
```

### 3. Remove inherited tracked files

Run:

```bash
git rm -rf .
```

If this returns:

```bash
fatal: pathspec '.' did not match any files
```

that is okay. It means there are no tracked files to remove.

### 4. Remove untracked files

Run only after confirming the user understands it deletes untracked files:

```bash
git clean -fd
```

If ignored files also need to be removed, ask before using:

```bash
git clean -fdx
```

### 5. Create the first file

Create a README file:

```bash
echo "# <branch-name>" > README.md
```

### 6. Commit the new empty-history branch

Run:

```bash
git add README.md
git commit -m "init <branch-name>"
```

### 7. Push to remote

If the repository has a remote named `origin`, push with upstream tracking:

```bash
git push -u origin <branch-name>
```

## Full Command Template

```bash
git switch --orphan <branch-name>
git rm -rf . || true
git clean -fd
echo "# <branch-name>" > README.md
git add README.md
git commit -m "init <branch-name>"
git push -u origin <branch-name>
```

## Notes

An orphan branch is useful when the user wants a completely new version of a project without keeping the previous commit history.

It is different from:

```bash
git switch -c <branch-name>
```

because `git switch -c` creates a normal branch based on the current branch.

It is also different from deleting files on a normal branch, because the commit history would still remain.
