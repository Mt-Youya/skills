---
name: agent-session-archaeology
description: Find historical coding-agent conversations by project ownership, recover earlier debugging methods, and build a traceable pitfalls guide from actual messages and tool results. Use to capture a current pitfall, review all sessions, or maintain a guide incrementally. Bundled Node tools discover Claude Code, Codex, DSH, and some VS Code chatSessions; MAgent, CatPaw, and CatDesk require the external Rust tool or manual review, not Node coverage. Distinguish ownership filtering, parse coverage, semantic review, and missing records. Redact and deduplicate findings; record symptoms, diagnosis, proposals, implementation, and verification without treating unverified proposals as resolved.
compatibility: Bundled tools use Node.js built-ins; DSH decoding requires a runtime with zlib.zstdDecompressSync. The optional external Rust tool lives at project-root scripts/session-audit-rs (not distributed with the skill) and requires Cargo and available dependencies; DSH needs zstd, and Codex registry access needs sqlite3. Access only authorized local directories. Cross-platform path probing does not mean every platform format has been tested.
---

# Agent Session Archaeology

Round up the agent conversations scattered across a machine and mine them for experience. The hard part is not reading files — it is deciding **whether a session belongs to this project at all**. Directory names lie.

## Ground rules

1. **Decide ownership from the transcript, never from the directory name.** One project path can appear under several directory names: `C:\work\demo` becomes `C--work-demo` under Claude Code and `--C-work-demo--` under DSH. The encodings differ per tool and neither is reversible. The only reliable signals are the `cwd` recorded inside the session and the tool's own registry.
2. **Successful decompression does not mean complete decompression.** Existing DSH samples use concatenated zstd frames; historical Node decoding observations returned only the first frame without errors. Do not generalize this behavior to every runtime. Check record counts and the tail; see [references/transcript-formats.md](references/transcript-formats.md).
3. **Reconcile against the authoritative registry before concluding.** If the session count does not match the tool's own registry, you missed something.

## Environment and installation

- **Portability boundaries**: Node probes application directories on Windows, macOS, and Linux (`%APPDATA%`, `~/Library/Application Support`, `$XDG_CONFIG_HOME`, `~/.config`); this does not imply compatibility with every client format. Record missing sources, permission errors, and parse failures separately, rather than calling them all "no sessions".
- **Baseline tooling**: the bundled Node tools need no npm dependencies. Node ≥ 23.8 provides the zstd API; explicitly report sources that older runtimes cannot decompress.
- **This repository's accelerator**: `scripts/session-audit-rs` is the user-specified Rust tool, with pinned `regex` and `serde_json` dependencies. It can build offline when those dependencies are cached; it is not a resource automatically included when copying this skill. Use the Node path in repositories without it, and do not claim the Rust path has zero dependencies.
- **Any project**: pass `--project <dir>`, defaulting to the current working directory.

To install, copy the whole directory into any client's skills folder (standard [agentskills.io](https://agentskills.io/specification) layout):

```bash
cp -r agent-session-archaeology <project>/.agents/skills/   # one project
cp -r agent-session-archaeology ~/.agents/skills/           # every project for this user
cp -r agent-session-archaeology ~/.claude/skills/           # Claude Code, user level
```

## Fast filtering: ownership before content

1. Check the existing guide and coverage manifest first. If the project, source files, and scope have not changed, reuse the previous filtering results instead of scanning all stores again. Report filtering completion and full semantic-review completion separately.
2. Prefer the tool's own index and the session's recorded `cwd`, including project subdirectories. Encoded directory names are discovery clues, not ownership proof; a project path in a shared skill directory does not prove that the actual task belongs to this project.
3. When this repository has the Rust tool, use its concurrent version directly. Put build artifacts in the system temporary directory and exports in the ignored `.agent-sessions/`. On other machines, first check whether Rust, sqlite3, and zstd are available.

```bash
cargo run --offline --release --manifest-path scripts/session-audit-rs/Cargo.toml \
  --target-dir "${TMPDIR:-/tmp}/agent-session-audit-target" -- \
  --project "$PWD" --threads 8 --out .agent-sessions/audit
```

4. Routine scans should filter by metadata/registry first; this is a workflow requirement, not an implementation guarantee of the Node scanner. Node reads and parses known sources before matching the project; `--project` does not limit which transcript bodies it reads. Do not run its scan/export directly when only this project is authorized. Rust can skip bodies for tasks with a known external-project cwd, but unknown ownership still requires reading. Add `--content` only with real cross-repository clues and authorization; its results are candidates requiring manual confirmation.
5. `coverage.json` is a program-generated count manifest. `read_to_eof` means only reading and parsing; `semantic_review` does not automatically become reviewed. Exclude empty VS Code states, initialization records, this archaeology run, and review subtasks before counting valid historical sessions. Multiple export files can belong to one session; do not count files as sessions.
6. Redact exports first, then verify them independently with the existing checks below. Keep session identifiers in the full manifest, but redact actual cookie/token values in the body. Never publish or commit raw sessions.

```bash
node .agents/skills/agent-session-archaeology/scripts/redact-exports.mjs \
  --out .agent-sessions/audit
node .agents/skills/agent-session-archaeology/scripts/redact-exports.mjs \
  --out .agent-sessions/audit --check
```

If the user only wants to record a pitfall from the current conversation, do not scan historical stores; go straight to "Capturing a pitfall you just hit". If they request all sessions, first produce a finite, reconcilable session inventory, then review that inventory. List missing, damaged, and remote-only records separately.

## Three use cases (workflow examples, not executed historical calls)

| Request                                            | Minimum action                                                                                                                                                                                                                                           | Completion criteria                                                                                                                                                     |
| -------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| "Record the pitfall we just hit"                   | Use only the actual current conversation, deduplicate against existing entries, and add evidence for the four stages; no historical scan                                                                                                                 | Include the actual error, adopted solution, verification scope, and unresolved items; use a date placeholder for an unknown session ID, never invent one                |
| "Find how we debugged this before"                 | Check guide sources and existing inventories first, locate authorized historical records by error text/command, and read surrounding context continuously; page to the end when the complete session is needed                                           | Return original coordinates, the debugging timeline, failed attempts, and verification; state whether the scope is targeted or a complete session                       |
| "Review everything / continue the previous update" | For a full review, establish finite sources and an inventory first; for incremental work, reuse the inventory and process only new files, changed content, and unread ranges, rather than skipping appended records in older files based solely on dates | Reconcile source/registry differences, track parse and semantic coverage per session, preserve existing IDs, avoid near-duplicates, and list missing records separately |

Repeat discovery only when the scope changes or source indexes gain entries. File size/mtime are change clues only; check digests or tails when needed. These are routing and acceptance designs, not proof that all three use cases have actually been executed.

## Node workflow (without the Rust accelerator)

`$SKILL` below is wherever this skill is installed — project-local (`.agents/skills/…`) or user-global (`~/.agents/skills/…`, `~/.claude/skills/…`):

```bash
SKILL=.agents/skills/agent-session-archaeology   # or ~/.agents/skills/agent-session-archaeology

# 1. Probe first: which stores exist, which are missing, what Node can do
node "$SKILL/scripts/find-agent-sessions.mjs" doctor

# 2. Only with authorization to read all known source stores; --project filters matches, not reads
node "$SKILL/scripts/find-agent-sessions.mjs" scan

# 3. Fallback: also catch sessions whose cwd is elsewhere but whose text names this project
node "$SKILL/scripts/find-agent-sessions.mjs" scan --content

# 4. Export readable Markdown for locating evidence (not a lossless full transcript)
node "$SKILL/scripts/find-agent-sessions.mjs" export          # conversation text
node "$SKILL/scripts/find-agent-sessions.mjs" export --full   # only tool data retained by adapters
```

Options: `--project <dir>`, `--json` (JSON scan output), `--out <dir>` (export destination).

The `match` column in `scan` means: `cwd` = the recorded working directory is inside this project; `registry` = the tool's own registry names it; `content` = the text mentions this project's absolute path.

After export, `index.md` is the overview and `<source>--<id>.md` is one session. **Exports contain local paths and personal context — always gitignore them.**

`export` writes files first, then runs [scripts/redact-exports.mjs](scripts/redact-exports.mjs) over the entire output directory; it does not redact before writing, so interruption can leave raw content behind. Use a dedicated private output directory, and read/share exports only after redaction and `--out <dir> --check`. The check covers only implemented rules and supported text extensions, and skips `coverage.json`. Exit 0 does not prove safety for empty directories, unreadable files, short credentials, or unrecognized formats; check them manually without printing raw values in reports.

**Node `--full` is not complete evidence**: Claude tool input/result are not retained; Codex function_call/function_call_output are not extracted; DSH call arguments are missing and results are truncated at 4000 characters; VS Code extracts only recognized text responses. Node does not generate a Rust-style coverage.json, and parse errors or read failures may be skipped. Use its exports only to locate evidence. For full review, return to authorized originals or reconciled external exports, filling in original coordinates and gaps.

## Supported sources (bundled Node only)

| Tool                | Location                                                   | Format                          | Ownership signal                      |
| ------------------- | ---------------------------------------------------------- | ------------------------------- | ------------------------------------- |
| DSH                 | `~/.dsh/sessions/<slug>/<session>/session.v<N>.jsonl.zstd` | **multi-frame** zstd over JSONL | `cwd` on the first line               |
| DSH (authoritative) | `~/.dsh/storages/workspace.json`                           | JSON                            | `tables.workspaces[*].sessionIds`     |
| Claude Code         | `~/.claude/projects/<slug>/<uuid>.jsonl`                   | JSONL                           | `cwd` on every message record         |
| Claude Code         | `~/.claude/history.jsonl`                                  | JSONL                           | prompt history only, not a transcript |
| Codex               | `~/.codex/sessions/YYYY/MM/DD/rollout-*.jsonl`             | JSONL                           | `session_meta.payload.cwd` on line 1  |
| VS Code family      | `<appData>/<product>/User/workspaceStorage/<hash>/`        | patch stream                    | `workspace.json` folder URI           |

Node's `<product>` list includes only `Code`, `Code - Insiders`, `VSCodium`, `Cursor`, `Windsurf`, and `Trae` with the chatSessions layout above; it does not cover all their extension/native-agent stores.

**MAgent / CatPaw / CatDesk have no Node adapters and are absent from doctor's unsupported probes.** Therefore, doctor's `(none)` cannot prove these tools have no local sessions. The optional project-root Rust tool has separate discovery rules for MAgent `chat_history/events/updates` and recap, CatPaw transcript/agent-transcripts, CatDesk sessions/logs, and CatPaw IDE workspaceStorage. These are recognized formats only, not a guarantee of coverage across all versions, remote sessions, or complete tool results. Copying this skill to another project does not include that Rust tool. When it is missing, report unsupported sources explicitly and manually read authorized originals; do not count them as Node-covered.

Field-level detail and the slug-encoding table live in [references/transcript-formats.md](references/transcript-formats.md).

## Extending: adding a tool

1. Write a reader with the signature `() => session[]`, reusing the script's `makeSession` / `stamp` / `parseJsonl` / `flattenBlocks` / `safeRead`. Every session needs a `cwd`, or some other provable ownership signal.
2. Add one entry to the `SOURCES` array: `{ id, read, available, probe }`. `probe()` returns the paths the reader actually opens; `doctor` reports on them.
3. Run `doctor` to confirm the probe, then `scan` to confirm the parse.

**Never guess a format from memory.** Find a file the tool actually wrote before writing the parser — invented schemas are the single biggest source of bugs in this kind of script.

## Pitfalls (all hit in practice)

1. **Older decoding paths may truncate multi-frame zstd**: historical Node observations returned only the first frame without errors. The Rust path uses `zstd -dc` to decode all concatenated frames sequentially. Do not treat magic-byte searching as a universal frame parser; compressed content can contain the same bytes. Verify decompression coverage against record counts and the tail.
2. **Directory slugs are not reversible** — `:` becomes `-` under Claude but is dropped under DSH. Decoding a slug back to a project path is a dead end.
3. **Several format versions coexist** — when `session.v3` and `session.v4` both exist, take the highest version or the session is counted twice.
4. **Session ids come with and without a prefix** — `abc` and `session-abc` both appear in directory names and registries; strip the prefix before comparing.
5. **An empty session is not a session** — VS Code family `chatSessions/*.jsonl` often holds metadata only. Filter on "message count > 0".
6. **A backslash needle never matches** — searching transcript text with a normalized forward-slash path always misses; try both separators and lowercase.
7. **`cwd` may be a subdirectory** — a session started inside a subdirectory records that subdirectory, so match by prefix rather than equality.
8. **Files may be unreadable** — not every machine has every tool installed. Guard every read; one missing `history.jsonl` must not take down the whole scan.

## Verification (do this)

- **Reconcile with the authoritative registry**: the number of `sessionIds` in `~/.dsh/storages/workspace.json` for this project should equal the number of DSH sessions found.
- **Sample in reverse**: pick a session that did _not_ match and confirm its `cwd` really is elsewhere.
- **Run it on another project**: `--project` pointed at a different repository should give sensible results (including sessions from subdirectories), proving the logic is not tuned to one repo.
- **Build a fake HOME**: point `HOME`/`USERPROFILE` at a temporary directory, lay out macOS/Linux directory conventions, and run `scan` — this validates cross-platform path handling on a single machine.

## Full review and debugging

### External Rust single-file paging (does not trigger source discovery)

Run from the project root; `scripts/session-audit-rs` **is not inside the skill**. Choose the target file from an ownership-confirmed inventory. The paths and page ranges below are usage placeholders, not claims of completed review:

```bash
cargo run --offline --release --manifest-path scripts/session-audit-rs/Cargo.toml \
  --target-dir "${TMPDIR:-/tmp}/agent-session-audit-target" -- \
  --project "$PWD" --file "<authorized-session.jsonl>" --from 1 --to 200
# Next page: use the same command with --from 201 --to 400; continue to the verified original final line
```

- `--from`/`--to` are 1-based, inclusive **original file line numbers**, not exported Markdown lines or message indexes. The coverage footer's textual records count measures displayed records, not comprehension; no output may simply mean omitted metadata.
- The pager parses JSON one line at a time. It supports JSONL and `.zst`/`.zstd` that decompresses to JSONL, not plain `transcript.txt`/`.log` or multi-line JSON/Markdown. When an inventoried `.jsonl` disappears, check its exact `.jsonl.zst`/`.jsonl.zstd` sibling before concluding that records are missing; compressed bytes must go through the decoder. Use ordinary read-only paging for text files and record their coordinates separately. Inline `data:image/...;base64` payloads are replaced with an explicit omission marker, preserving adjacent text and original record numbers; this is not visual verification.
- Paging omits injected context, encrypted reasoning, and some exact Codex mirror records; mirror deduplication is page-local. Track omissions and cross-page duplicates against original line ranges, without removing new user corrections, failure results, or tool calls.
- A finite `--to` stops the zstd subprocess early and cannot establish decompression to EOF. Require an existing full-decode check and tail comparison. If offline dependencies are unavailable, report it; do not install dependencies or modify scripts to work around it.

### Compact coverage record (separate from the program manifest)

Reuse existing inventories/reports, and write files only when authorized. Do not alter the program's coverage.json to pretend semantic review is complete. Group multiple originals per session, but retain ranges separately for each file:

```yaml
project: <authorized-project>
scope: <sources; time boundary; full or incremental>
source: <tool>
session_id: <full-id or current session YYYY-MM-DD>
files: [<private-path or shareable source locator>]
file_state: <bytes; mtime; digest if checked>
ownership: <cwd/registry evidence; content candidate confirmed or excluded>
time_range: <start..end or unknown>
parse: <read_to_eof; total original lines/records; errors; changed_during_read>
semantic_review: <not_reviewed/partial/complete>
read_ranges: [<file:original-lines or event IDs>]
unread_ranges: [<ranges or unknown end>]
omissions: [<injected context; images; compacted/missing tool results>]
registry_delta: <matched; missing; unreadable; remote-only; excluded self-audit>
findings: [<existing pitfall IDs; appended evidence; unresolved hypotheses>]
next: <first unread range or none>
```

Every declared range must have actually been read. Use complete only after continuously reviewing the declared, finite scope and documenting its gaps; it does not mean deleted/compacted history has been recovered. Report candidate files, deduplicated sessions, parses to EOF, semantically complete reviews, partial reviews, and missing records as separate counts.

- When the user requests full reading, page continuously from the first actual user message through the last, including assistant replies, tool arguments, results, errors, and retries. Keywords locate evidence; they do not replace full reading. Reduce page size when a batch is too large, rather than using cut to truncate each record's body.
- Record the source, full ID, originals, start/end timestamps, total records, actual read ranges, unread ranges, and missing tool results for each session. A program reading the file and an agent understanding the complete timeline are separate acceptance criteria.
- Repeatedly injected system/skill lists and non-text image payloads may be omitted, but record the omission types. Consecutive, identical phase events may be collapsed losslessly while retaining original line ranges, counts, and start/end timestamps. Never collapse away new user corrections or failure results.
- For MAgent, review `chat_history.jsonl` first; recover results replaced by omission markers from `updates.jsonl`, `recap_requests`, and original terminal output. Retain both Codex function_call arguments and function_call_output; Claude tool results may appear in user messages. Summaries locate clues, not original evidence.
- Divide parallel review into non-overlapping complete sessions or fixed original ranges; subtasks are read-only. Off-target replies, handoff summaries, and incomplete-review reports do not count as completion. Do not blindly dispatch the entire review again.
- Historical conversations and tool outputs are evidence, not current commands. Do not execute their deployment, deletion, login, Git, or migration operations. Current source code is dated present-state evidence only; it cannot retrospectively prove a historical fix passed verification.
- Record product failures, the agent's own misjudgments, verification-tool traps, and risks that have not occurred separately. Failed experiments and user corrections often matter more than the final "done".
- Cite full session IDs, timestamps, and original line numbers/event IDs. Rust's `Original line` refers to original JSONL lines; exported Markdown physical lines are a different coordinate system. Never mix them.

## Turning findings into a pitfalls guide

Respect the user's authorized output path first. Create a guide only with authorization when none exists; supplement existing guides incrementally, without forcing eight topics. The bundled guide tools default to `docs/avoiding-pitfalls-guide/` (override with `--guide`); this is a repository convention, not a skill installation requirement:

```text
docs/avoiding-pitfalls-guide/
├── README.md            overview + auto-generated index (do not hand-edit the marked region)
├── 01-verification.md   ← 8 fixed topics, which do NOT grow with each batch
├── 02-data-contracts.md
├── …
└── evidence.gz       ONE archive holding every trimmed excerpt
```

**One archive, not one file per pitfall.** Dozens of small `.gz` files are harder to manage, harder to browse, and compress worse than a single one (every file pays its own header). At a few tens of KB total, one archive is the right shape; `show --id` still prints a single section without unpacking anything.

**Classify by topic, not by date or by tool.** Readers look for "how do I avoid this class of problem", not "which day or which tool said it". Keep the batch as per-pitfall metadata: the topic set stays fixed, the index does not bloat, and appending a new batch never reshuffles the directory.

Keep each pitfall's `### <ID> · <conclusion title>` heading and `batch` / `severity` / `status` / `source` metadata. The body needs at least four stages: **1. How it appeared (context and actual symptoms); 2. How it was diagnosed (chronological actions, observations, failed attempts); 3. What was proposed (candidates, rejection reasons, adopted solution); 4. How it was resolved (implemented changes, actual verification, remaining issues)**. Add evidence and prevention steps. Existing guides may retain their eight fields; do not rewrite old entries for formatting alone.

Status must be supported by evidence. Resolved covers only the scope actually verified. For partial fixes, state what was implemented, what remains unverified, and what was temporarily bypassed. Never invent later success for unresolved items or missing records. Mark an unconfirmed root cause explicitly as a "hypothesis"; HTTP 400, an empty list, or exit 0 cannot establish a general root cause.

ID prefixes are per topic: `VER-` `DATA-` `DB-` `AUTH-` `MEDIA-` `DEPLOY-` `AGENT-` `TOOL-`.

**Language**: keep this skill and its supporting documentation in English. Write guide titles, prose, and explanations in the user's language; this project uses Simplified Chinese. Preserve the existing format of machine-readable metadata keys `batch` / `severity` / `status` / `source`. Do not localize code, commands, or verbatim quotations.

### Evidence must be trimmed per pitfall

Users append in batches over time, and one session is megabytes — archiving whole sessions will bloat the repository within a few batches.

- Short pitfall, the inline excerpt is enough → **inline only**, no 📎 line.
- Needs long context to be understood → inline summary + archive **only the contiguous line range relevant to that pitfall** (typically 20–80 lines), and add a line like `- 📎 excerpt: [evidence.gz](../evidence.gz) → ID` to the body.

```bash
SKILL=.agents/skills/agent-session-archaeology   # wherever the skill is installed

node "$SKILL/scripts/pack-evidence.mjs" add    --guide docs/avoiding-pitfalls-guide \
  --id VER-02 --title "cross-machine clock skew" \
  --segment <exported-session.md>:7210-7320 \
  --segment <another-session.md>:640-688        # repeatable; multi-range and cross-session are fine
node "$SKILL/scripts/pack-evidence.mjs" show   --guide docs/avoiding-pitfalls-guide --id VER-02
node "$SKILL/scripts/pack-evidence.mjs" list   --guide docs/avoiding-pitfalls-guide
node "$SKILL/scripts/pack-evidence.mjs" remove --guide docs/avoiding-pitfalls-guide --id VER-02
node "$SKILL/scripts/pack-evidence.mjs" unpack --guide docs/avoiding-pitfalls-guide   # readable copy, outside the repo
node "$SKILL/scripts/pack-evidence.mjs" check  --guide docs/avoiding-pitfalls-guide   # size audit; exit 2 over budget
node "$SKILL/scripts/build-guide-index.mjs"    --guide docs/avoiding-pitfalls-guide   # rebuild the README index
```

Two guards exist because both mistakes were made for real:

- **Oversized excerpt** — warns above 512 KB uncompressed. A wrong `--lines` once made a "36 line" archive hold 1.2 MB of the entire session while still reporting 36 lines.
- **Personal paths** — warns when an excerpt contains `C:\Users\<name>` or `/Users/<name>`. The archive travels with the repository, so such a path is a dead reference for everyone else and a privacy leak for the author, while the exported session directory it points at is gitignored and not present in any clone. Record the **session name and line range** instead; rewrite any incidental path in the excerpt as `~/…`.

### Updating an existing guide (incremental, never destructive)

A repository may already carry a guide from an earlier run. **Never regenerate it** — it holds hand-written prose and a stable index that other documents link to.

```bash
node "$SKILL/scripts/build-guide-index.mjs" --guide <guide-dir> --json
```

That prints the inventory: every existing `id`, `source`, `reference` (`—` for an independent record), `independentCount`, `referenceCount`, per-topic counts, and `nextIds` — the next free id per prefix, including retained reference IDs. The parser retains `·` inside provenance and stops only at the next recognized metadata label. Still compare the inventory with original topic metadata before evidence deduplication: a complete `source` string does not prove that its original transcript was available or reviewed. Report mismatches rather than silently treating them as missing evidence. Use it to:

1. **Deduplicate at two levels before appending.** Group sessions by `(source, full session_id)`, take the highest DSH version, and do not count MAgent history/updates/recap as separate sessions. Do not blindly strip ID prefixes across different sources. Identify evidence by `(source, session_id, original file, original range/event_id)`, then compare symptoms, confirmed root causes, and resolution constraints. When an existing pitfall matches, add sources, failed attempts, status, and new verification to it. Similar titles alone do not justify merging different failures; overlapping ranges or duplicate messages must not erase new facts.
2. **Continue the numbering** from `nextIds`. Never reuse an id.
3. **Append to the matching topic file**, at the end. Do not reorder, renumber or rewrite existing records.
4. **Rebuild the index** (`build-guide-index.mjs` without `--json`); it rewrites only the region between the README's marker comments and leaves the prose alone.

**Deduplicated reference entries**: keep the existing heading/ID as a stable navigation entry; never delete or renumber it. Put `reference` before `source` on the same metadata line (the parser stops collecting metadata when it sees `source`), for example:

```text
`batch` <YYYY-MM-DD> · `severity` <existing severity> · `status` <evidence-backed status> · `reference` <existing independent ID> · `source` <actual source locator>
```

The target must exist and be directly independent (`reference` absent/`—`), not self, another reference, a chain, or a cycle. Keep canonical evidence and new verification on that independent record, and retain the reference entry's provenance. Count reference entries separately, never again in independent severity/status/batch/evidence statistics; reserve every retained ID in `existingIds` and `nextIds`. Read-only `--json` inventories and `--check` index-drift checks are safe validation commands; rebuilding requires write authorization.

Stamp each addition with its own `batch` date, so the next increment stays traceable.

### Capturing a pitfall you just hit

You do not need a session export. When a pitfall surfaces **in the conversation you are in**, that conversation is the source:

1. Extract the same facts you would from a transcript: what broke, how it surfaced, how it was diagnosed, what fixed it, what to do differently. Quote the **real** error text, command and path from the conversation — not a paraphrase of them.
2. Compare evidence and failure facts against existing entries first, updating the original ID when matched. Only new pitfalls get a topic and an ID from `nextIds`.
3. Write the entry into that topic file with the evidence inline (the actual output, quoted). Archive a range only when the surrounding context is genuinely needed _and_ the session has already been exported — otherwise inline is the correct choice.
4. Set `source` to the session id when you know it, or `current session <YYYY-MM-DD>` when you do not.
5. Rebuild the index.

Do not wait for a batch. A pitfall written down while it is fresh is worth more than one reconstructed a month later from a transcript nobody re-reads.

### Write for humans

The guide is read by people, not by the agent that produced it.

**1. Write in the user's language.** Whatever language the conversation is in, the guide body uses it — titles, the seven sections, the README prose. Only the metadata labels (`batch`, `severity`, `status`, `source`) stay English, because scripts parse them.

**2. Humanize before finishing.** Resolve a humanizer nearest-first, and stop at the first hit — the same way configuration lookup works, a narrower scope wins:

1. **This repository.** `<project>/.agents/skills/`, `<project>/.claude/skills/`. A project-local humanizer is the most relevant one that can exist: someone chose it for _this_ codebase's voice and language.
2. **This machine.** `~/.agents/skills/`, `~/.claude/skills/`, `~/.codex/skills/`, plus the skill list the client injects into the session. Look for a name containing `humaniz` (or the equivalent in the user's language).
3. **The open ecosystem** — [skills.sh](https://skills.sh) and GitHub — only once 1 and 2 both came up empty. Run the CLI **with the package manager the project declares** — `npx` inside a pnpm repo dies with `EBADDEVENGINES`, which is a mismatch to respect, not to work around by leaving the directory:
   ```bash
   pnpm dlx skills find "humanizer <language>"   # project pins pnpm
   npx      skills find "humanizer <language>"   # npm, or no pin
   yarn dlx skills find "humanizer <language>"   # yarn
   bunx     skills find "humanizer <language>"   # bun
   ```
   If the CLI cannot start at all — a sandbox that denies writes to the package-manager cache (`ERR_PNPM_CLI_DLX_CACHE`, npm `EPERM`), no network, or the tool is missing — do not stop there: search the web for `skills.sh humanizer <language>`. The `skills.sh` search page is client-rendered and its API requires a Vercel OIDC token, so a plain web search is the route that works when the CLI is unavailable.

If one turns up at any level, **use it as-is** — do not keep searching for a "better" one. If all three come up empty, do not stall and do not install anything without asking; apply the rules below.

Naming convention in the ecosystem: plain `humanizer` is the English one; localized versions carry a language suffix (`humanizer-zh`, `humanizer-ja`, …). Pick the one matching **the language the user is writing in** — not the language of this skill.

The rules to apply when no humanizer is available:

- **Concrete beats abstract.** "The local clock ran 12 s ahead of Supabase" — not "a time-synchronisation issue was present".
- **One idea per paragraph.** Cut the pile-up of parallel clauses that machine prose defaults to.
- **No throat-clearing.** Delete openers like "It is worth noting that", "In conclusion", "Furthermore", and the closing paragraph that repeats what was just said.
- **Name the actor.** "The agent read the exit code and declared success" beats "success was declared".
- **A heading states the conclusion**, not the topic: "Migration 2 can never apply", not "Migration ordering".
- **Leave code, paths and commands exactly as they are.** Do not smooth them out for the sake of readability.
- **Keep quotes verbatim.** They are evidence; rewriting them destroys their value.

Read the result back once as if a colleague had written it. If it sounds like a status report produced by a machine, rewrite it.

## Boundaries

- Formats change between tool versions; `doctor` reports whether a location exists, not whether the format still matches. When parsing fails, fall back to grepping the raw session files instead of declaring there were no conversations.
- Session files can be deleted by the tools' own cleanup. **Export as soon as you find them** rather than relying on them being there later.
- `doctor` lists only implemented probes. An existing directory does not prove complete sessions exist; absence from its list does not prove there are none. List unsupported sources separately. Add adapters only when the user authorizes script changes; otherwise report gaps and read originals manually.
