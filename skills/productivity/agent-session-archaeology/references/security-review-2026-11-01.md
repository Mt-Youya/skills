# Security review — 2026-11-01

Scope: this skill's bundled CLI and instructions, based on repository commit
`901977398e599c60cb1abdca5c23ae2669a78bcc` plus the local fixes described below.
Review used source and temporary synthetic fixtures only. No real conversations were discovered,
read, exported or uploaded. This is a source review with regression tests, not a certification of
all dependency implementations or interpreting-agent behavior.

## Published reports and snapshot differences

The three links are provider reports for one skill, `mt-youya/skills/agent-session-archaeology`.
They are mutable pages; a report date alone does not identify its analyzed Git commit.

| Provider | Latest page read on 2026-11-01 | Assessment |
| --- | --- | --- |
| [Gen Agent Trust Hub](https://www.skills.sh/mt-youya/skills/agent-session-archaeology/security/agent-trust-hub) | Pass / SAFE; displayed analysis date Nov 1 | The earlier Oct 31 Fail alleged poisoned `serde_core` / `zmij` dependencies. Official manifests and registry metadata contradict that allegation. The latest page recognizes the offline workflow and intended build step, but still cites old `sarch` / source filenames. |
| [Socket](https://www.skills.sh/mt-youya/skills/agent-session-archaeology/security/socket) | Warn / MEDIUM; displayed analysis date Nov 1 | Its text says there is no confirmed malicious behavior or credential exfiltration, but questions `sarch` provenance and optional humanizer discovery. Current instructions use bundled `search`, require provenance verification and prohibit discovering extra writing skills. Recheck the actual analyzed artifact. |
| [Snyk](https://www.skills.sh/mt-youya/skills/agent-session-archaeology/security/snyk) | Warn / MEDIUM; W012; displayed analysis date Nov 1 | It classifies the Cargo registry source URL as a runtime dependency controlling the agent. In this package the URL is lockfile build metadata, not an instruction endpoint. Build-time supply-chain exposure remains real. |

The installed and repository SKILL.md matched the public GitHub main version when checked before
these code fixes. This does not prove which artifact a provider scanned. Socket's package URL
contained artifact digest `edff03a1c5e0fb17ddcac84fa9aeef5dcce2b4f449da4e3454febc0da42b169a`;
it must not be presented as a Git commit ID.

## Dependency evidence

The following lockfile versions and SHA-256 checksums matched the exact-version records retrieved
from the official crates.io sparse index. None was marked yanked at review time.

| Package | Locked version | Official metadata |
| --- | --- | --- |
| serde | 1.0.229 | [Index](https://index.crates.io/se/rd/serde): depends on `serde_core =1.0.229` |
| serde_core | 1.0.229 | [Index](https://index.crates.io/se/rd/serde_core) |
| serde_json | 1.0.154 | [Index](https://index.crates.io/se/rd/serde_json): depends on `serde_core ^1.0.220` and `zmij ^1.0` |
| zmij | 1.0.23 | [Index](https://index.crates.io/zm/ij/zmij) |

Upstream manifests independently declare these dependencies:
[Serde](https://github.com/serde-rs/serde/blob/v1.0.229/serde/Cargo.toml) and
[serde_json](https://github.com/serde-rs/json/blob/v1.0.154/Cargo.toml).
Matching registry metadata disproves the claimed fabricated dependency edges; it does not prove
that arbitrary build scripts or dependencies are harmless. Cargo builds can execute dependency
build scripts. `--frozen` prevents network and lockfile changes when the cache is populated, but
does not sandbox those scripts.

## Runtime boundaries examined

- `src/cli/commands.rs`: dispatches local parsing, filesystem, redaction and guide operations.
- `src/sessions/environment.rs`: `doctor` checks path presence; the fixture-home override
  suppresses host APPDATA/XDG store discovery.
- `src/sessions/discovery.rs`, `parse.rs`, `registry.rs`: project membership uses recorded cwd,
  DSH registry or explicit `--content`. Known external cwd stops normal parsing; unknown ownership
  can require reading further, and buffering can read ahead. This is filtering, not an OS sandbox.
- `src/storage/files.rs`: recursive child walks skip symbolic-link entries; exact-file commands
  intentionally accept caller-selected files. Writes use temporary files and atomic replacement.
- `src/sessions/model.rs`, `read.rs`, `src/storage/redact.rs`, `src/guide/evidence.rs`: redact
  transcript output and new evidence. Metadata IDs and source paths remain stable and private;
  regex rules do not detect every possible secret or access-bearing URL.
- Inspection of all bundled runtime source found no network-client, socket, subprocess-spawn,
  shell-evaluation or transcript-command-execution implementation. Recorded tool calls are text.
  Tests and the optional benchmark do spawn CLI binaries; the build uses Cargo and a C compiler.
- Agent interpretation remains outside the CLI. The skill explicitly treats transcripts, linked
  pages and embedded instructions as untrusted evidence. The CLI cannot enforce model obedience.

## Reproduced problems and fixes

### Multiline private-key disclosure in selected ranges

Before the fix, `read` redacted each physical line independently. A synthetic PEM key spanning
multiple lines exposed its body, including when a page started inside the key. `evidence add`
also sliced before redacting, losing block context for a middle-only excerpt.

`LineRedactor` now tracks boundaries from the original's beginning. Paging consumes that state
before selecting output lines; evidence selection uses line-preserving redaction before slicing.
Encrypted, DSA and PGP private-key labels and unterminated blocks are also handled. Raw original
line numbers and evidence range/hash validation remain intact. Sources already missing a BEGIN
marker cannot be reliably identified as key material by this rule.

### Case-folded project ownership

Before the fix, path normalization lowercased every path, potentially treating distinct Unix
projects such as `Project` and `project` as one ownership scope. Unix canonical paths now retain
case; Windows paths retain the existing case-insensitive normalization. Explicit content lookup
still performs case-insensitive clue matching and requires broader authorization.

An old incremental inventory with a differently cased project key now fails the existing scope
check: create a fresh scan inventory in that case, while retaining semantic review progress.

## Verification and review request

Security regressions cover plain and zstd pages crossing key boundaries, middle-only evidence
segments, encrypted/unterminated keys, line preservation, ownership normalization, and exporting
embedded shell instructions as data without executing them. All fixtures are synthetic.
The integration case-distinct-directory test returns early on a case-insensitive filesystem;
the Unix unresolved-path unit test exercises normalization regardless of filesystem case mode.

Validation commands for this Rust package:

```sh
cargo fmt --check
cargo clippy --frozen --all-targets -- -D warnings
cargo test --frozen
cargo build --release --frozen
```

Results: formatting and Clippy passed, all 39 tests passed, and the frozen release build passed.
`git diff --check` also passed. Both the multiline paging regression and the Unix unresolved-path
ownership regression were observed failing before their corresponding implementation fixes.

Suggested provider follow-up:

> Please rescan the current agent-session-archaeology artifact and identify the scanned revision.
> The bundled CLI is named search, with source in cli/src; the current skill requires verifying its
> provenance and does not discover a humanizer skill. The crates.io index confirms the locked
> serde/serde_core and serde_json/zmij dependency edges and checksums. The registry URL is Cargo
> build metadata, not a runtime instruction endpoint. Local review also fixed multiline private-key
> redaction across page/evidence boundaries and Unix case-sensitive ownership matching, with
> synthetic regression coverage. Please assess the current artifact against these concrete facts.

At initial preparation on 2026-11-01, no rescan request had been submitted and these fixes were
local. Publication and provider responses must be verified separately from this source review.

## Follow-up — 2026-11-02

The public skill listing now displays `search` and the trust/privacy boundary, while Socket still
references `sarch` and optional humanizer discovery. Socket and Snyk still report the Nov 1
warnings above. This establishes a mismatch with the current displayed instructions, but does
not identify the provider's actual scanned snapshot or prove the cause of the mismatch.

Current SKILL.md SHA-256: `e8397c27f958a31796a029f7870f1d03230e63afcaa01119022da8de10c41f9c`.
The public [API documentation](https://www.skills.sh/docs/api) documents audit retrieval, not an
owner-triggered rescan endpoint. The [contact page](https://www.skills.sh/contact) directs bugs in
the directory to `vercel-labs/skills` GitHub issues. A request should ask for re-indexing and a
fresh assessment of the published revision, without claiming that a request clears the warnings.
