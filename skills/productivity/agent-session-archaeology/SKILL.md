---
name: agent-session-archaeology
description: Recover earlier coding-agent debugging methods and maintain an evidence-backed pitfalls guide from authorized local conversations. Use for a current pitfall, a targeted historical lookup, or incremental session review. The companion offline Rust CLI discovers Claude Code, Codex, DSH and VS Code-family chatSessions; report unsupported sources and distinguish parse coverage from semantic review.
---

# Agent Session Archaeology

Use the installed `sarch` Rust CLI for deterministic discovery, paging, redaction,
evidence packing and index validation. The agent interprets actual conversations and writes the
guide. No account, token, endpoint, network or user config is required at runtime.

**Default to incremental updates.** Reuse the guide, inventory, evidence archive and review
progress. Do not ask whether to start over. Review from scratch only when explicitly requested;
asking to cover all sessions does not discard prior progress. With no previous state, establish
the first baseline directly.

## Verify the CLI first

```sh
command -v sarch
sarch --json doctor
```

`doctor` probes paths without reading transcript bodies. It lists implemented and unsupported
sources; missing data, unknown formats, permission failures and damaged records are different
states. The CLI uses built-in multiframe zstd decoding, not external `zstd`/SQLite executables.
MAgent, CatPaw and CatDesk remain unsupported by discovery; read authorized originals manually
when needed, and never claim automatic coverage for them.

If the command is missing, use this skill's `cli/` directory (set `$SKILL` to the actual installed
skill directory): `make -C "$SKILL/cli" install-local`. Rust/Cargo and a C compiler are needed to
build; the installed binary needs no Node or Rust runtime. Ensure `~/.local/bin` is on PATH.
Command contracts, JSON examples, exit codes and build checks are in [cli/README.md](cli/README.md).
Do not invent a project-root Rust accelerator or fall back to retired Node scripts.

## Route the request

- **Record a current pitfall:** use only the current conversation; do not discover historical
  stores. Read `guide inventory`, match existing facts/IDs, and save a small private excerpt when
  no export exists. Archive that evidence even if it is short enough to quote inline.
- **Find an earlier debugging method:** inspect existing guide sources/inventories, resolve an
  exact source/ID, then page the authorized original around the evidence. Reuse known filenames
  instead of rescanning. State whether the review is targeted or covers a complete session.
- **Update/review all sessions:** reuse prior filtering and semantic progress. Discover only when
  scope/indexes change; process new files, changed contents and unread ranges, including appended
  records in older files. Establish a finite inventory before claiming full coverage.

Before historical review or guide writing, read [references/review-workflow.md](references/review-workflow.md)
for coverage records, continuous reading, deduplication, four-stage entries and human-language rules.
Read [references/transcript-formats.md](references/transcript-formats.md) only for a source format,
ownership signal, version difference or parser failure relevant to the current task.

## Discover, resolve, read

Use `--json` for machine-readable output: success is `{ "ok": true, "data": ... }`; failures have
`ok: false` and `error`, and partial scans can retain `data`. Inspect `diagnostics`, `registry`,
`incomplete`, original ranges and omissions. Exit 0 is not semantic review completion.

```sh
# Discovery reads local stores: authorize their scope first.
sarch --json sessions scan --project /work/project
sarch --json sessions resolve --project /work/project --source codex --id session-id
# Exact-file read-only escape hatch, without rediscovery; default pages contain at most 200 lines.
sarch --json read --file /private/authorized-session.jsonl.zstd --from 1 --to 200
```

Ownership comes from recorded `cwd` (including subdirectories) and source registries, never a
lossy directory slug. Known external ownership stops further parsing; buffered readers may read
ahead, and unknown ownership needs inspection. `--content` also reads external session bodies:
use it only with real cross-project clues and authorization, then manually confirm candidates.
Normalize optional `session-` prefixes only within DSH registry comparisons.

Full review requires reading the finite scope continuously, including user corrections, tool
arguments/results, errors and retries. Keywords, program EOF counts and summaries only locate
clues. Preserve actual original line/event coordinates. Patch-derived workspace entries use line
0 as an explicit derived marker; they are not original transcript line numbers.

Export into a private gitignored directory with `sessions export --project DIR --out DIR --full`;
use `--dry-run` to preview. Exports are redacted before writing; then run `redact --out DIR --check`
and inspect unrecognized/short credentials and personal paths without printing their values.
`--full` includes retained tool data, not deleted/compacted history or every possible source field.
`coverage.json` is a parse inventory, never an assertion of completed semantic review.

## Maintain the guide and its evidence

Respect the authorized output directory; commands default to `docs/avoiding-pitfalls-guide/`.
A request to create or update the guide authorizes those file writes; do not ask again to choose
the update mode. Keep skill/supporting instructions in English, but guide titles and prose in the
user's language. Machine metadata keys `batch`, `severity`, `status`, `source`, `reference` stay English.

1. Read `sarch --json guide inventory --guide DIR`. Match existing failure facts and
   source coordinates before allocating a new ID from `nextIds`; matching findings keep their ID.
2. Create only populated topic chapters and number files consecutively from `01`. No fixed eight
   positions or empty placeholders: four findings categories can use `01/02/03/04`. Compact legacy
   gaps and repair all repository links to renamed files; preserve IDs, anchors, entry order and
   prose. Add later topics at the end.
3. Every independent pitfall needs real, trimmed, redacted evidence in **one `evidence.gz`**. Inline
   quotations do not replace it. References reuse the independent target's evidence. Use only the
   relevant actual excerpt, often 20–80 lines or less; never whole-session archives or invented
   session IDs/coordinates. Save current-conversation excerpts privately when no export exists.
4. Append with `evidence add --guide DIR --id VER-01 --segment FILE:FROM-TO` (repeatable). Existing
   IDs retain old segments and deduplicate identical source/range/SHA tuples. Use `--dry-run` to
   preview. Add `[evidence.gz](./evidence.gz) → VER-01` to the topic entry. Backfill missing old
   evidence from available originals; if unavailable, retain the entry and report incomplete work.
5. Read back the affected sections, rebuild the marked README index and pass validation:

```sh
sarch evidence show --guide /work/project/docs/avoiding-pitfalls-guide --id VER-01
sarch guide index --guide /work/project/docs/avoiding-pitfalls-guide
sarch --json guide check --guide /work/project/docs/avoiding-pitfalls-guide
```

`guide check` requires consecutive populated chapters, a current index and a nonempty readable
archive with valid SHA/range evidence for every independent ID. Missing evidence is failure;
size-budget overflow exits 2. Never report the guide complete while these checks fail.
