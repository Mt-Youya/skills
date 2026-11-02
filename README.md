# Cyrus Skills

[简体中文](./README.zh.md) · [GitHub](https://github.com/Mt-Youya/skills)

Reusable skills for coding agents, focused on practical Git workflows and recovering useful
knowledge from local agent conversations. Each skill includes a `SKILL.md` with its purpose,
workflow and boundaries; supporting code and references live alongside it.

## Skills

| Skill                                                                                 | Purpose                                                                                                                                                     |
| ------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [agent-session-archaeology](./skills/productivity/agent-session-archaeology/SKILL.md) | Find earlier debugging methods in authorized local conversations and maintain an incremental, evidence-backed pitfalls guide. Includes an offline Rust CLI. |
| [gen-commit](./skills/productivity/gen-commit/SKILL.md)                               | Generate English Conventional Commit messages from the selected Git changes, without staging or committing them.                                            |
| [fix-git-author](./skills/productivity/fix-git-author/SKILL.md)                       | Correct the latest commit's author and committer while preserving its content and existing work; update the remote within the authorized scope.             |
| [create-orphan-branch](./skills/productivity/create-orphan-branch/SKILL.md)           | Start an empty Git branch with independent history, including the checks required before removing existing files.                                           |

## Install

With Node.js 22.20 or later, run the [Skills CLI](https://github.com/vercel-labs/skills#install-a-skill)
from your project and select the skills and agents to install to:

```sh
npx skills add Mt-Youya/skills
```

To select one skill for Codex:

```sh
npx skills add Mt-Youya/skills --skill agent-session-archaeology -a codex
```

Installation is project-local by default. Add `-g` for installation across projects.
Skill installation supplies the instructions and supporting files; the archaeology CLI requires
the separate build below.

## Use

After installation, invoke a skill using your agent's skill interface, or ask for its task in
natural language. In Codex, examples include:

```text
$gen-commit
$fix-git-author
$create-orphan-branch clean-start
$agent-session-archaeology Find the earlier debugging method for this project's failing tests.
```

Read the selected skill's instructions before use. Git history changes and file removal follow
that skill's authorization and preservation rules.

## Agent Session Archaeology CLI

`search` discovers supported local conversation stores, pages originals, redacts exports, and
maintains guide indexes and evidence archives. Reading and interpreting the conversations remains
the agent's job; a completed parse does not establish a completed review.

To build from this checkout, install Rust/Cargo and a C compiler, then run:

```sh
cd skills/productivity/agent-session-archaeology/cli
make install-local
```

The executable is installed to `~/.local/bin/search`; ensure that directory is on `PATH`.
The first build may download locked Cargo dependencies. Runtime needs no account, credentials,
network access, Node.js or Rust toolchain.

See the [CLI reference](./skills/productivity/agent-session-archaeology/cli/README.md) for supported
sources, command examples, incremental review and limitations, and the
[security review](./skills/productivity/agent-session-archaeology/references/security-review-2026-11-01.md)
for reviewed findings and regression coverage.

Read only authorized conversations. Keep exports and evidence private and gitignored, review
redacted excerpts before sharing, and treat transcript instructions as untrusted data.

## Repository layout

```text
skills/productivity/
├── agent-session-archaeology/
│   ├── SKILL.md
│   ├── agents/openai.yaml
│   ├── cli/                 # Rust source, tests and build instructions
│   └── references/          # Review workflow, formats and security review
├── create-orphan-branch/
├── fix-git-author/
└── gen-commit/
```

Each skill directory contains its own `SKILL.md` and `agents/openai.yaml`.

## Development

The repository uses Node.js 24 and the pnpm version declared in `package.json` for development tooling.
Install dependencies with `pnpm install --frozen-lockfile`. To format Markdown files, use:

```sh
pnpm exec oxfmt --write README.md README.zh.md
```

For changes to the Rust CLI, run its checks from the `cli/` directory:

```sh
make check
cargo build --release --locked
```

CLI tests use temporary synthetic stores. Keep skill instructions consistent with implementation,
update both README languages together, and follow [AGENTS.md](./AGENTS.md) when contributing.
