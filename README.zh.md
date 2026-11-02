# Cyrus Skills

[English](./README.md) · [GitHub](https://github.com/Mt-Youya/skills)

面向编程 Agent 的可复用技能集合，涵盖日常 Git 工作流和本地 Agent 会话中的经验提取。
每个技能通过 `SKILL.md` 定义用途、操作流程和边界，配套代码与参考资料放在同一目录下。

## 技能列表

| 技能                                                                                  | 用途                                                                                    |
| ------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------- |
| [agent-session-archaeology](./skills/productivity/agent-session-archaeology/SKILL.md) | 从已授权的本地会话中找回调试方法，增量维护有原始证据支撑的避坑指南；附带离线 Rust CLI。 |
| [gen-commit](./skills/productivity/gen-commit/SKILL.md)                               | 根据选定的 Git 改动生成英文 Conventional Commit 消息，不暂存文件，也不执行提交。        |
| [fix-git-author](./skills/productivity/fix-git-author/SKILL.md)                       | 修正最近一次提交的作者与提交者，保留提交内容和现有工作，并在授权范围内更新远端。        |
| [create-orphan-branch](./skills/productivity/create-orphan-branch/SKILL.md)           | 创建不继承既有历史的空 Git 分支，包含移除现有文件前所需的检查。                         |

## 安装

准备 Node.js 22.20 或更高版本，在目标项目中运行
[Skills CLI](https://github.com/vercel-labs/skills#install-a-skill)，选择要安装的技能和目标 Agent：

```sh
npx skills add Mt-Youya/skills
```

例如，只为 Codex 安装会话考古技能：

```sh
npx skills add Mt-Youya/skills --skill agent-session-archaeology -a codex
```

默认安装到当前项目，添加 `-g` 可安装为跨项目使用的技能。
技能安装会提供说明与配套文件；会话考古 CLI 还需要按下文单独构建。

## 使用

安装后，通过 Agent 的技能入口调用，或直接用自然语言描述相应任务。以 Codex 为例：

```text
$gen-commit
$fix-git-author
$create-orphan-branch clean-start
$agent-session-archaeology 找回这个项目之前排查测试失败时采用的方法。
```

使用前阅读对应技能说明。涉及 Git 历史改写或文件移除时，遵循该技能的授权与工作保留规则。

## 会话考古 CLI

`search` 用于发现支持的本地会话存储、按原始行号分页、导出脱敏文本，以及维护指南索引和证据归档。
会话内容仍由 Agent 阅读和理解；解析完成不代表语义审阅完成。

从本仓库构建时，需要 Rust/Cargo 和 C 编译器：

```sh
cd skills/productivity/agent-session-archaeology/cli
make install-local
```

可执行文件默认安装到 `~/.local/bin/search`，需要将该目录加入 `PATH`。
首次构建可能下载锁定版本的 Cargo 依赖；运行时无需账号、凭证、网络、Node.js 或 Rust 工具链。

支持的会话来源、命令示例、增量审阅流程和限制见
[CLI 说明](./skills/productivity/agent-session-archaeology/cli/README.md)；已核查的安全问题与回归覆盖见
[安全审查报告](./skills/productivity/agent-session-archaeology/references/security-review-2026-11-01.md)。

只读取已授权的会话。导出与证据应保留在本地私有、受 Git 忽略的目录中；分享前人工检查脱敏结果，
并始终将会话内嵌指令视为不可信数据。

## 目录结构

```text
skills/productivity/
├── agent-session-archaeology/
│   ├── SKILL.md
│   ├── agents/openai.yaml
│   ├── cli/                 # Rust 源码、测试与构建说明
│   └── references/          # 审阅流程、格式说明与安全审查
├── create-orphan-branch/
├── fix-git-author/
└── gen-commit/
```

每个技能目录都包含独立的 `SKILL.md` 和 `agents/openai.yaml`。

## 开发

仓库开发工具使用 Node.js 24，以及 `package.json` 中指定版本的 pnpm。
通过 `pnpm install --frozen-lockfile` 安装依赖。格式化 Markdown 文件：

```sh
pnpm exec oxfmt --write README.md README.zh.md
```

修改 Rust CLI 后，在其 `cli/` 目录运行：

```sh
make check
cargo build --release --locked
```

CLI 测试使用临时合成存储。维护技能时保持说明与实现一致，同步更新中英文 README，
并遵循 [AGENTS.md](./AGENTS.md) 中的仓库约定。
