# Transcript formats (field level)

SKILL.md covers _how to use_ the tooling; this file covers _what the formats look like_, for when a parser needs writing or debugging.

## Contents

- [Platform path conventions](#platform-path-conventions)
- [Path slug encodings](#path-slug-encodings)
- [DSH](#dsh)
- [Claude Code](#claude-code)
- [Codex](#codex)
- [VS Code family](#vs-code-family)
- [MAgent / CatPaw / CatDesk (external tooling boundaries)](#other-sources-and-external-tooling-boundaries)
- [Ownership resolution order](#ownership-resolution-order)

---

## Platform path conventions

Command-line tools keep sessions in `~/` dot-directories, identical on every platform. GUI clients move theirs around, so those locations **must be probed, never hard-coded**.

| Purpose         | Windows                           | macOS                                         | Linux                                |
| --------------- | --------------------------------- | --------------------------------------------- | ------------------------------------ |
| Home            | `%USERPROFILE%`                   | `/Users/<name>`                               | `/home/<name>`                       |
| App config/data | `%APPDATA%` (Roaming)             | `~/Library/Application Support`               | `$XDG_CONFIG_HOME` or `~/.config`    |
| Local data      | `%LOCALAPPDATA%`                  | `~/Library/Application Support`               | `$XDG_DATA_HOME` or `~/.local/share` |
| CLI dot-dirs    | `~/.claude`, `~/.codex`, `~/.dsh` | same                                          | same                                 |
| VS Code family  | `%APPDATA%/Code/User/...`         | `~/Library/Application Support/Code/User/...` | `~/.config/Code/User/...`            |

Node's `appDataRoots()` prefers the current platform and also probes `%APPDATA%`, `$XDG_CONFIG_HOME`/`~/.config`, and macOS Application Support. It does not probe `%LOCALAPPDATA%` or `$XDG_DATA_HOME` as VS Code roots. The table above describes platform conventions, not a claim that the script probes every directory or that every platform has been tested.

One more platform trap: on Windows, `path.resolve()` prepends the current drive letter to a POSIX-absolute path, turning `/home/x/proj` into `C:\home\x\proj`. Cross-platform paths (shared configs, WSL, transcripts copied from elsewhere) are already absolute and should be left as-is.

---

## Path slug encodings

For a given project path, each tool encodes a different directory name. The rules differ and **neither is reversible**:

| Tool        | Input path       | Encoded as        | Rule                                    |
| ----------- | ---------------- | ----------------- | --------------------------------------- |
| Claude Code | `C:\work\demo`   | `C--work-demo`    | `:` → `-`, `\` → `-` (no collapsing)    |
| DSH         | `C:\work\demo`   | `--C-work-demo--` | `:` dropped, `\` → `-`, wrapped in `--` |
| Claude Code | `/home/dev/demo` | `-home-dev-demo`  | `/` → `-` as well                       |

Because a path can itself contain `-`, decoding is ambiguous. **Never reconstruct a project path from a slug.**

---

## DSH

### Layout

```
~/.dsh/sessions/<slug>/<session-dir>/session.v<N>.jsonl.zstd
~/.dsh/storages/workspace.json          # authoritative registry
```

A session directory may hold both `session.v3` and `session.v4`. Measured: **v4 = v3 plus one `session/end-seed` record**, identical otherwise. Take the highest version.

### Multi-frame zstd (the easiest trap to fall into)

The file is **several independent zstd frames concatenated**, one written per append:

- Historical observations: `zlib.zstdDecompressSync(wholeFile)` returned only the first frame (a 208–283-byte session header) without errors; streaming showed the same behavior.
- Earlier records report 58 frames in a 103 KB sample and 5582 in an 8.3 MB sample. The implementation review did not retest those samples or every Node version. Do not treat historical observations as guarantees for the current runtime.

Full scans/exports in the external Rust tool use `zstd -dcq` to decode concatenated frames and check the subprocess exit status, without guessing frame boundaries by scanning magic bytes. Paging with a finite `--to` stops the subprocess early and does not verify decompression to EOF. **The bundled Node tool still scans magic bytes and attempts to decompress subsequent slices**; this is not a general correctness guarantee. Compressed content can contain those same bytes, and failed slices are ignored. Record original byte counts, decoded record counts, and tail events, then reconcile against the registry before judging coverage.

### Event types

The first line is always `{"type":"session", "version":N, "id":"...", "cwd":"...", "createdAt":<ms>}`.

| `type`                                 | Key fields                                         | Use                                            |
| -------------------------------------- | -------------------------------------------------- | ---------------------------------------------- |
| `session`                              | `id`, `cwd`, `createdAt`, `agentPreset`            | **ownership signal**, session metadata         |
| `user/message`                         | `data.content[]` (block array), `data.role`        | user input                                     |
| `assistant/message`                    | `data.message.content[]`, `data.turn`, `data.step` | assistant output (includes `reasoning` blocks) |
| `tool/call`                            | `data.name`, `data.arguments` (JSON string)        | tool invocation                                |
| `tool/result`                          | `data.message.content[].text`, `data.toolCallId`   | tool result                                    |
| `session/title`                        | `data.title`                                       | session title                                  |
| `compaction/summary`                   | `data.summary`                                     | context-compaction summary                     |
| `turn/start`, `step/start`, `step/end` | `data.turn`, `data.step`                           | turn boundaries                                |
| `session/end-seed`                     | —                                                  | new in v4, session end marker                  |

Timestamps: top-level `time` (ms epoch); the `session` line uses `createdAt`.

---

## Claude Code

### Layout

```
~/.claude/projects/<slug>/<uuid>.jsonl
~/.claude/history.jsonl                 # global prompt history, carries `project`
```

### Line types

| `type`                                                       | Notes                                                                                            |
| ------------------------------------------------------------ | ------------------------------------------------------------------------------------------------ |
| `user` / `assistant`                                         | the conversation; `message.content` is a string or a block array, **every record carries `cwd`** |
| `mode`, `permission-mode`                                    | session mode                                                                                     |
| `file-history-snapshot`, `file-history-delta`                | file snapshots                                                                                   |
| `attachment`                                                 | attachments                                                                                      |
| `system`                                                     | system events                                                                                    |
| `ai-title`                                                   | session title                                                                                    |
| `cost-state`, `queue-operation`, `last-prompt`, `atis-latch` | metadata                                                                                         |

### Content block types

`text` (body), `tool_use` (`name`/`input`), `tool_result` (`content`), `thinking`.

Tool results appear as `tool_result` blocks inside **`user` records** — do not treat them as user input.

---

## Codex

### Layout

```
~/.codex/sessions/YYYY/MM/DD/rollout-<ISO time>-<uuid>.jsonl
~/.codex/sessions/YYYY/MM/DD/rollout-<ISO time>-<uuid>.jsonl.zst
```

### Line types

The first line is `type=session_meta`; `payload.cwd` is the **ownership signal** and `payload.id` is the session id.

| `type`                                                           | Notes                                                                                                         |
| ---------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------- |
| `session_meta`                                                   | first line: cwd, id, cli_version, base_instructions                                                           |
| `response_item`                                                  | `payload.type=message`, `payload.role` in developer/user/assistant, `content[]` of `input_text`/`output_text` |
| `event_msg`                                                      | `payload.type` such as `task_started`, `item_completed` (`item.type` = `UserMessage`/`AgentMessage`)          |
| `turn_context`, `world_state`, `token_usage_record`, `compacted` | metadata                                                                                                      |

`event_msg.item_completed` and `response_item` may duplicate the same message, but the latter also contains `function_call` and `function_call_output`. Full review must retain actual tool arguments and results, not just extract messages. When omitting repeatedly injected system/developer prompts, record their types as well.

---

## VS Code family

Node enumerates `workspaceStorage/chatSessions/*.jsonl` for Code, Code - Insiders, VSCodium, Cursor, Windsurf, and Trae, parsing only the patch layout below. This does not cover every product's entire chat storage. CatPaw IDE is absent from Node's product list; native Cursor Agent, other extensions, databases, and remote stores must not be considered supported on that basis.

### Layout

```
%APPDATA%/<product>/User/workspaceStorage/<hash>/workspace.json
%APPDATA%/<product>/User/workspaceStorage/<hash>/chatSessions/<uuid>.jsonl
```

`workspace.json`'s `folder` looks like `file:///c%3A/work/demo` and needs URL-decoding.

### Patch stream

Each line is one JSON object:

- `{"kind":0,"v":{...}}` — the initial full object (`v.requests[]` is the conversation)
- `{"kind":1,"k":[path...],"v":value}` — patch by path
- any other `kind` — treat as unknown and skip

**Many sessions have an empty `requests` array** — the file exists but holds no conversation. Always filter on "message count > 0".

---

## Other sources and external tooling boundaries

The following was statically checked against the discovery rules in project-root `scripts/session-audit-rs/src/main.rs`. **It is neither a capability of the bundled Node tools nor a claim of actual session-reading tests.** Copying the skill does not include the project-root Rust tool.

| Source  | Paths/files recognized by the external Rust tool                                                                                                         | Ownership and review notes                                                                                                                                                                                                |
| ------- | -------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| MAgent  | `~/.magent/sessions/<id>/chat_history.jsonl`, `events.jsonl`, `updates.jsonl`, `recap_requests/*.json`                                                   | Use `info.cwd`/`git_root_dir` in the same directory's `summary.json` as clues; summaries cannot replace conversations. Group the files as one session and recover omitted tool results from updates/recap/terminal output |
| CatPaw  | `transcript.txt` or `agent-transcripts/*.jsonl` under `~/.catpaw/projects` and `~/.catpaw/sessions`; application-root `CatPaw IDE/User/workspaceStorage` | Encoded directories are discovery clues only; confirm ownership with actual cwd/workspace URI and the user's task. List unrecognized formats separately and do not claim full coverage                                    |
| CatDesk | `.jsonl`/`.log`/`.json` under `~/.catdesk/sessions`, `~/.catdesk/logs/conversations`, and `~/.catdesk/logs/saved_sessions`                               | Accepting files under these rules does not mean every payload becomes a complete conversation. Confirm against actual metadata/content and deduplicate logs and saved copies                                              |

Node's SOURCES contains only `claude-code`, `dsh`, `codex`, and `copilot`; UNSUPPORTED_PROBES also omits the three tools above. Doctor's `(none)` applies only to its listed probes. Without the external tool, report these sources as unsupported/unverified and manually review authorized originals; do not call them nonexistent.

### Original evidence and paging coordinates

- Node `--full` is a display option; it does not recover data discarded by the parser. Claude tool input/result and Codex function_call/output are absent from exports; DSH arguments are missing and tool results are truncated at 4000 characters; VS Code retains only recognized response text. Verify complete timelines against originals.
- External Rust `--file <file> --from 1 --to 200` enters the single-file branch before discovery, using positive, inclusive original line numbers. The next page is `201..400`, not displayed message 201.
- The pager parses JSON one line at a time. Use ordinary read-only paging for plain text/multi-line JSON/Markdown; `.zst`/`.zstd` line numbers refer to decompressed JSONL. It omits injected context, encrypted reasoning, and some page-local exact mirrors; cross-page duplicates still need reconciliation.
- External Rust `readable_value` recursively displays objects/arrays and strings that parse as JSON objects/arrays. `embedded_json_line` also expands valid JSON objects after exact `<file>.jsonl[:|-]<line>[:|-]` locators, including `.jsonl.zst`/`.jsonl.zstd`, while retaining the locator. This makes nested tool output readable, not a byte-identical export or proof that the embedded file was independently read. It marks `aggregated_output`/`formatted_output` as omitted only when exactly equal to sibling `stdout`; differing output remains. These are statically checked implementation properties, not new runtime or decompression tests.
- Node parseJsonl ignores invalid lines, and safeRead can return an empty value; these do not automatically produce auditable error coverage. Rust parsing/export also does not mean the agent has semantically reviewed the content. Retain files, original ranges, unread ranges, and omissions; do not confuse them with exported physical lines.

## Ownership resolution order

1. **`cwd` recorded inside the session** (DSH first line / every Claude message / Codex `session_meta` / Copilot `workspace.json`) — highest authority, and match by **prefix**, not equality, because a session may have started in a subdirectory of it.
2. **The tool's own registry** (`~/.dsh/storages/workspace.json`, `tables.workspaces[*].sessionIds`) — for reconciliation and fallback.
3. **Content matching** — last resort only, and **both separators must be tried**: `norm()` produces forward slashes while transcripts quote backslashes.

Session ids appear with and without a `session-` prefix in both directory names and registries; strip it before comparing.
