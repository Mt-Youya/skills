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

The Rust CLI probes `%APPDATA%`, `$XDG_CONFIG_HOME`/`~/.config`, and macOS Application Support. An explicit `SESSION_ARCHAEOLOGY_HOME` isolates fixture stores and suppresses host APPDATA/XDG paths. The table describes platform conventions, not proof that every product/platform format has been tested.

Normalize both path separators and existing local path aliases before ownership checks. In particular, macOS `/var` and `/private/var` can name the same directory. Preserve foreign Windows drive paths when inspecting copied records on another OS; never infer ownership from a slug.

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

A session directory may hold both `session.v3` and `session.v4`. Historical samples showed **v4 = v3 plus one `session/end-seed` record**, identical otherwise; verify actual files rather than assuming every version pair has that relationship. Take the highest version.

### Multi-frame zstd (the easiest trap to fall into)

The file is **several independent zstd frames concatenated**, one written per append:

- Historical observations: `zlib.zstdDecompressSync(wholeFile)` returned only the first frame (a 208–283-byte session header) without errors; streaming showed the same behavior.
- Earlier records report 58 frames in a 103 KB sample and 5582 in an 8.3 MB sample. The implementation review did not retest those samples or every Node version. Do not treat historical observations as guarantees for the current runtime.

The bundled Rust CLI uses the zstd library's streaming decoder across concatenated frames. It never scans compressed bytes for frame magic or invokes an external decoder. Interrupted/corrupt decoding is reported in diagnostics and makes the inventory incomplete. A bounded page does not verify decompression to EOF. Record original byte/line counts and tail events, then reconcile against the registry before judging coverage.

### Event types

Known samples begin with `{"type":"session", "version":N, "id":"...", "cwd":"...", "createdAt":<ms>}`.

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

The Rust CLI enumerates `workspaceStorage/chatSessions/*.jsonl` for Code, Code - Insiders, VSCodium, Cursor, Windsurf, and Trae, parsing only the patch layout below. This does not cover every product's entire chat storage. CatPaw IDE is absent from the CLI's supported product list; native Cursor Agent, other extensions, databases, and remote stores must not be considered supported on that basis.

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

These are historical locations for manually locating authorized originals. The bundled Rust CLI reports these tools as unsupported; it does not contain the formerly documented external accelerator or adapters for them. Confirm actual metadata and format versions before reading.

| Source  | Historical file-location clues                                                                                                         | Ownership and review notes                                                                                                                                                                                                |
| ------- | -------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| MAgent  | `~/.magent/sessions/<id>/chat_history.jsonl`, `events.jsonl`, `updates.jsonl`, `recap_requests/*.json`                                                   | Use `info.cwd`/`git_root_dir` in the same directory's `summary.json` as clues; summaries cannot replace conversations. Group the files as one session and recover omitted tool results from updates/recap/terminal output |
| CatPaw  | `transcript.txt` or `agent-transcripts/*.jsonl` under `~/.catpaw/projects` and `~/.catpaw/sessions`; application-root `CatPaw IDE/User/workspaceStorage` | Encoded directories are discovery clues only; confirm ownership with actual cwd/workspace URI and the user's task. List unrecognized formats separately and do not claim full coverage                                    |
| CatDesk | `.jsonl`/`.log`/`.json` under `~/.catdesk/sessions`, `~/.catdesk/logs/conversations`, and `~/.catdesk/logs/saved_sessions`                               | Accepting files under these rules does not mean every payload becomes a complete conversation. Confirm against actual metadata/content and deduplicate logs and saved copies                                              |

The implemented sources are `claude-code`, `dsh`, `codex`, and `copilot`. Doctor reports separate
unsupported probes for MAgent, CatPaw and CatDesk; their absence from a probe or from a scan does
not establish that they have no sessions. Read manually supplied originals only within scope.

### Original evidence and paging coordinates

- `sessions export --full` retains recognized Claude tool inputs/results, Codex function calls and
  outputs, and DSH arguments/results. It does not truncate DSH results, but cannot recover omitted
  history, unknown event fields, attachment pixels or encrypted reasoning. Verify against originals.
- `read --file FILE --from 1 --to 200` does not run discovery. `lines[].line` uses positive,
  inclusive original text line numbers; the next page begins at 201, not displayed message 201.
- Plain UTF-8 text, JSONL and `.zst`/`.zstd` text are read as numbered raw lines. Compressed line
  numbers refer to decompressed text. Credential values are redacted, but injected context,
  duplicate mirrors, inline images and JSON-encoded tool strings are not semantically rewritten.
- VS Code messages reconstructed from patches have line 0 and an explicit derived-data note.
  Page the original patch stream to establish the actual source ranges; never cite line 0 as a
  historical transcript coordinate.
- Invalid JSON lines and read/decode failures are reported separately. An incomplete scan/export
  returns available data with exit 1. EOF parsing is never completed semantic review.


## Ownership resolution order

1. **`cwd` recorded inside the session** (DSH first line / every Claude message / Codex `session_meta` / Copilot `workspace.json`) — highest authority, and match by **prefix**, not equality, because a session may have started in a subdirectory of it.
2. **The tool's own registry** (`~/.dsh/storages/workspace.json`, `tables.workspaces[*].sessionIds`) — for reconciliation and fallback.
3. **Content matching** — last resort only, and **both separators must be tried**: `norm()` produces forward slashes while transcripts quote backslashes.

DSH session IDs may appear with or without a `session-` prefix; normalize that prefix only inside DSH registry comparisons, never across different tools.
