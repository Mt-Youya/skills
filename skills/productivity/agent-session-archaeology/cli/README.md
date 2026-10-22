# sarch

Offline Rust CLI for discovering project-owned agent conversations, exporting redacted evidence,
and maintaining an incremental pitfalls guide. It replaces the four Node scripts previously
bundled with this skill. Conversation interpretation and guide prose remain the agent's job.

## Install and verify

Build from this directory with Rust/Cargo and a C compiler (the bundled zstd library is compiled
and linked into the executable). Cargo downloads locked dependencies on the first build.

```sh
make install-local
# ~/.local/bin must already be on PATH, or add it to your shell configuration.
cd /tmp
command -v sarch
sarch --help
sarch --json doctor
```

`PREFIX=/another/prefix make install-local` selects another installation prefix. Runtime needs no
Node, Cargo, external `zstd`, SQLite executable, network access, credentials or config file.
The current install target builds for the host; precompiled releases for other platforms are not
published by this repository.

## Command surface

| Command | Read/write contract |
| --- | --- |
| `doctor` | Probe locations only; report offline/auth-not-required state and unsupported sources |
| `sessions scan --project DIR [--content]` | Inventory ownership-matched sessions and original-file diagnostics |
| `sessions resolve --project DIR --id ID [--source SOURCE]` | Resolve an exact stable ID to one original file; fail on ambiguity |
| `sessions export --project DIR [--out DIR] [--full] [--dry-run]` | Export redacted Markdown plus index and parse coverage; reuse unchanged files |
| `sessions read --file FILE --from N [--to N] [--limit 200]` | Page an exact authorized original without rediscovery |
| `read --file FILE ...` | Raw read-only escape hatch; same paging and redaction as `sessions read` |
| `redact [--out DIR] [--check] [--dry-run] [--quiet]` | Redact supported text files, including credentials beside binary bytes; leave `coverage.json` IDs intact |
| `evidence add --id ID --segment FILE:FROM-TO [--segment ...]` | Redact and append new excerpts to one gzip archive, deduplicating identical source/range/SHA tuples |
| `evidence show --id ID` | Read one archive section without creating files |
| `evidence list` | List section IDs, titles and excerpt coordinates |
| `evidence remove --id ID [--id ID] [--dry-run]` | Remove only explicitly named sections |
| `evidence unpack [--out FILE] [--dry-run]` | Write a readable archive copy; default destination is the system temp directory |
| `evidence check [--budget-mb 8]` | Require a nonempty readable archive and valid evidence for each independent pitfall |
| `guide inventory` | Read IDs, source metadata, references, topic counts and `nextIds` |
| `guide index [--check] [--dry-run]` | Rebuild only the marked README region, or check drift without writing |
| `guide check [--budget-mb 8]` | Check populated/consecutive chapter filenames, README drift and evidence coverage |

Use `--help` after each command for full flags. `--json` is global and works before or after the
subcommand. Guide/evidence commands accept `--guide DIR`, defaulting to
`docs/avoiding-pitfalls-guide`; `redact` defaults to `.agent-sessions` under the working directory.
`evidence add` retains the legacy `--session FILE --lines FROM-TO`, `--title`, `--anchor` and
`--budget-mb` flags. Repeating an existing ID appends evidence; it does not replace old sections.
`--quiet` is accepted for redaction compatibility; summaries contain counts, never credential values.

## JSON and exit policy

This is a CLI envelope, not API pass-through. `--json` writes one JSON document to stdout;
diagnostics belong in that document or on stderr, never as extra stdout lines.

```json
{"ok":true,"data":{"mode":"offline","auth":{"required":false,"source":"not_required"}}}
```

```json
{"ok":false,"error":{"code":1,"message":"missing required evidence.gz"}}
```

Partial scans/exports and failed checks can also include `data` so callers can inspect diagnostics.
Every exported session has `semantic_review: "not_reviewed"`; parsing to EOF never asserts that a
human or model has understood the complete conversation. A missing registry session, read/decode
failure or malformed JSON makes scans/exports exit 1 with `incomplete: true` and available results.

Exit 0 means success, including empty discovery results. Exit 1 means an operational or validation
failure (including `redact --check` finding credentials and stale indexes). Exit 2 means argument
errors or an exceeded guide size budget. `--help` and `--version` remain human-readable and exit 0.
Without `--json`, metadata commands print readable JSON data and `evidence show` prints its text.

Examples for each family (paths are placeholders, not claims of executed audits):

```sh
sarch --json doctor
sarch --json sessions scan --project /work/project
sarch --json sessions resolve --project /work/project --source codex --id session-id
sarch --json sessions export --project /work/project --full --dry-run
sarch --json redact --out /work/project/.agent-sessions --check
sarch --json evidence add --guide /work/project/docs/avoiding-pitfalls-guide \
  --id VER-01 --segment /private/excerpts/session.md:20-40 --dry-run
sarch --json evidence list --guide /work/project/docs/avoiding-pitfalls-guide
sarch --json guide inventory --guide /work/project/docs/avoiding-pitfalls-guide
sarch --json guide index --guide /work/project/docs/avoiding-pitfalls-guide --dry-run
sarch --json guide check --guide /work/project/docs/avoiding-pitfalls-guide
sarch --json read --file /private/authorized-session.jsonl.zstd --from 201 --to 400
```

Successful discovery data includes `sessions`, `diagnostics`, `registry` and `incomplete`. Session
rows retain legacy names such as `startedAt`, `endedAt`, `userMessages`; guide inventories retain
`independentCount`, `referenceCount`, `existingIds`, `nextIds`, `pitfalls` and complete `source` strings.
Exports return `files` with `path`, `bytes` and `changed`. Raw reads return numbered `lines`,
`read_to_eof` and `next_from`. Evidence operations return archive/section metadata; checks report
byte counts and `over_budget`. Never parse default display text when `--json` is available.

## Sources and boundaries

Implemented adapters cover Claude Code JSONL, Codex rollout JSONL, DSH's highest session version
with concatenated zstd frames and authoritative workspace registry, and VS Code-family
`workspaceStorage/chatSessions` patch streams. Platform paths follow HOME/USERPROFILE, APPDATA,
XDG_CONFIG_HOME and macOS Application Support conventions. Sources are probed independently.

Ownership comes from recorded `cwd`, matching DSH registry IDs, or explicit `--content` candidates.
Known external ownership stops further transcript parsing unless `--content` is given. Buffered
readers/decoders can read ahead, and unknown ownership requires inspection; authorize stores before
discovery. Registry ID prefix normalization applies only to DSH. Content candidates still need
manual confirmation. Directory slugs are never ownership proof.

MAgent, CatPaw, CatDesk, Gemini CLI, Continue, OpenCode and Zed are reported as unsupported, not
counted as covered by these adapters. A probe is not a format/version compatibility guarantee.
The former documentation's optional external Rust accelerator is not part of this CLI.

`--full` includes retained tool arguments/results, without truncating DSH results. It still cannot
recover deleted/compacted records, attachment pixels or fields that adapters do not recognize.
Codex mirror events may duplicate messages. Workspace patch reconstructions use line 0 as an
explicit marker for derived content, not an original transcript coordinate. Raw paging returns
original text and redacts credential values, but does not semantically filter injected context or
decode image payloads. A bounded compressed page does not prove full-stream decode to EOF.

Exports are private and must be gitignored. New exports/excerpts are redacted before writing.
Redaction preserves arbitrary non-UTF8 bytes in files and reports errors instead of calling
unreadable files clean. Its recognizable patterns cannot prove that all possible secrets were
removed; check short/unrecognized credentials and personal paths separately without printing them.
An export directory is an explicit transform target: existing supported text files there are also
redacted. Preview first when it contains unrelated files.

The gzip Markdown archive format remains compatible with old `evidence.gz` files, including legacy
spacing and SHA/range metadata. Excerpts containing reserved archive section/segment delimiters
are rejected instead of corrupting the archive. Budget validation measures direct guide files;
avoid whole-session archives. `evidence add` reports a budget overrun after a real write, matching
the former script; `--dry-run` estimates the proposed compressed archive without changing files.
`guide check` reports chapter gaps; the agent must rename chapters and repair repository links
before acceptance. The CLI never renames hand-written topic files implicitly.

The retired entrypoints map to the installed command as follows:

| Former script | Rust command |
| --- | --- |
| `find-agent-sessions.mjs doctor/scan/export` | `doctor` / `sessions scan` / `sessions export` |
| `redact-exports.mjs` | `redact` |
| `pack-evidence.mjs COMMAND` | `evidence COMMAND` |
| `build-guide-index.mjs --json` | `--json guide inventory` |
| `build-guide-index.mjs [--check]` | `guide index [--check]` |

## Development

```sh
make check
```

Tests use temporary simulated stores and never inspect real session history. For an isolated
manual fixture, set `SESSION_ARCHAEOLOGY_HOME=/private/fixture/stores`. This selects a separate
home-shaped store tree and suppresses host APPDATA/XDG roots without changing HOME/USERPROFILE.
No transcript or fixture is required at a path relative to the installed binary.

Design references: [Rust CLI overview](https://rust-lang.org/what/cli/) and
[Command Line Applications in Rust](https://rust-cli.github.io/book/index.html).
