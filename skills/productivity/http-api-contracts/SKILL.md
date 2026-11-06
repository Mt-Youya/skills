---
name: http-api-contracts
description: Standardizes HTTP API implementations, machine-readable contracts, and real regression tests using RFC 9110, OpenAPI, and JSON Schema 2020-12. Use whenever adding, changing, reviewing, or debugging HTTP endpoints, routes, request/response DTOs, API clients, reverse proxies, gateways, Webhooks, or SSE; also for status codes, authentication headers, OpenAPI/Swagger, JSON Schema, API standards, or frontend/backend contract consistency, even for requests such as "add an endpoint", "follow the spec", or "fix this API". Apply proportionally to the change; do not turn a small fix into a whole-service rewrite. Excludes pure UI styling, non-HTTP function interfaces, and ordinary database operations.
metadata:
  version: "1.1.0"
  language: "en"
  translations: "zh-CN: SKILL.zh.md"
  standards-baseline: "RFC 9110; OpenAPI 3.2.1; JSON Schema 2020-12"
  author: "Cyrus Doyle"
---

# HTTP API Contracts and Implementation Standards

[Chinese version](SKILL.zh-CN.md). English is the canonical entry point; the translation describes the same workflow and is not a separate skill. Authoring language does not override the user's or workspace's response-language instructions.

Require three kinds of evidence: correct HTTP behavior, valid machine-readable contracts, and real requests/responses that match those contracts. Passing a document's meta-schema does not establish runtime conformance; green tests do not establish complete endpoint coverage.

## Load resources as needed

- For every API task, read the [standards checklist](references/standards-checklist.md) and apply only the relevant items.
- For proxies, gateways, SSE, or streaming billing, also read [proxy and streaming boundaries](references/proxy-streaming.md).
- For Rust/axum, Cloudflare Fetch, or whole-service standardization, read [implementation lessons](references/implementation-lessons.md).
- Before citing a disputed requirement, upgrading versions, or resolving unknown behavior, read [authoritative sources and version policy](references/sources.md), then locate the relevant original text. Memory and search summaries are not normative evidence.

## 1. Establish scope instead of guessing the architecture

1. Read repository instructions and current changes; preserve the user's work. Inspect actual routes, deployment entry points, proxy layers, and existing tests.
2. Distinguish audit-only, a specific fix, a new endpoint, and full contract integration. Keep local tasks local; do not introduce a service-wide registry for one bug.
3. If ambiguity would produce materially different or large changes, show confirmed problems first, then ask about scope and breaking-change boundaries. Do not repeat questions already answered.
4. Preserve paths, fields, authentication mechanisms, and business semantics by default. Explain necessary incompatibilities, affected clients, and migration steps; obtain scope authorization rather than treating "standards" as unrestricted refactoring permission.
5. Identify the role: origin server, HTTP intermediary, application-protocol gateway, or client. Authentication challenges, Via, and forwarding obligations depend on that role.
6. Define acceptance criteria: reproducing input, status/headers/body, contract versions, client compatibility, business effects, and executable quality gates.

State a short plan and assumptions, then act. Do not announce conformance before gathering evidence.

## 2. Build an evidence-backed inventory

For affected operations, record actual method/path, handler, path/query parameters, request media types and DTOs, success/error statuses, response headers/body, authentication/authorization/CSRF, streaming behavior, and consumers.

Inspect the final public service, not just handlers: extractors, routers, authentication, proxies, error wrappers, and server encoders can add response branches or rewrite headers.

Classify findings:

- **Normative requirement:** identify the standard version, section, applicable role, and concrete code/input evidence.
- **Semantic or contract mismatch:** show which input produces which discrepancy; do not call it an RFC MUST without evidence.
- **Project policy or recommendation:** for example, error envelopes, Location policy, timeout budgets, or JSON-only successful responses.
- **Unverified risk:** reproduce framework behavior, platform capabilities, or external dependencies before presenting them as confirmed.

Do not automatically classify action-oriented POST paths, missing data envelopes, absence of Problem Details, non-422 validation errors, a 201 without Location, or a 429 without Retry-After as violations of these standards.

## 3. Establish one trustworthy contract source

Reuse the project's spec-first or code-first mechanism. If none exists, choose the smallest maintainable approach. Use standard generators and validators; do not write a partial JSON Schema engine.

- Pin the OpenAPI version and schema dialect. This skill's reference baseline is OAS 3.2.1 and Draft 2020-12; upgrades require generator, validator, and client compatibility checks, not merely changing a version string.
- Model requests and responses separately. Optional deserialization fields are not necessarily omitted during serialization; required fields, nullability, unknown fields, and defaults must match implementation behavior.
- For SQL projections and hand-built JSON, check actual fields and serialization. A DTO declaration does not guarantee SELECT columns exist. Do not expose sensitive internal models to simplify schema generation.
- Describe paths, parameters, media types, security AND/OR, response statuses, and important headers. Automatic HEAD, CSV, and SSE must not masquerade as ordinary JSON.
- Represent genuinely open boundaries honestly. Provider pass-through may use open schemas with explicit local/provider validation responsibilities. Do not invent full provider DTOs or use empty schemas to hide missing fixed local responses.
- Preserve the reference bases of `$ref`, `$id`, `$defs`, and components. Configure supported dialects and an explicit network/trust policy for remote references.
- Validate OpenAPI document structure, schema meta-schemas, and real instances separately; report all three.
- Update clients and consumer tests. Production response validation is not a default requirement; tests/development validation can avoid buffering large bodies or SSE in production.

If the implementation cannot yet be fully described, report the specific gap and conditions rather than claiming completion.

## 4. Correct protocol boundaries without bypassing business rules

- A 401 requires a `WWW-Authenticate` challenge applicable to the resource. Scheme names are case-insensitive; credentials must not be arbitrarily rewritten. Cookie authentication is not exempt, but must not falsely advertise Basic/Bearer support.
- Preserve actual `Allow` on 405. Separate unknown API routes from browser navigation; redirects are a contract decision, not a reason for a JSON client to unexpectedly receive a frontend page.
- HEAD has no response content. A 204 has no content or Content-Length, including zero. A 304 has no content; Content-Length, if present, describes the corresponding 200 representation.
- Frameworks may add length headers after handlers. When needed, normalize outside finalized routing and verify raw HTTP, rather than blindly stacking middleware.
- Distinguish syntax, size, media type, structural, and business validation. Choose statuses and envelopes according to actual semantics and contracts, not a universal 400 or 422 rule.
- Authentication infrastructure failure does not establish invalid credentials: do not map database faults to 401. Distinguish gateway 502/504 from local 500; excessive response size and similar policies need explicit decisions.
- Error wrapping must preserve challenges, Allow, cookies, and request IDs without exposing database details, secrets, or credential-bearing URLs.
- Preserve original request bytes until the relevant consumer needs them. Extra serialization can alter billing, signatures, idempotency hashes, or evidence.
- Do not change reserve/release/review/settle or Webhook idempotency semantics to satisfy HTTP tests. Include business effects in regression assertions.

## 5. Close the verification loop

Write a minimal reproducer or regression first, implement, then verify through the final service.

1. **Protocol regression:** relevant statuses, multiple 401 challenges, 405 method sets, HEAD, 204/304, media types, limits, path/query extraction, and response bytes.
2. **Contract regression:** meta-schemas, references, actual DTO serialization/deserialization, valid/invalid/missing/null/unknown fields, and boundary values.
3. **Real HTTP:** exercise actual routes and proxies; validate successful and error responses. Schema-generated samples supplement, not replace, actual data. Test arrays with at least one nonempty instance.
4. **Business regression:** authentication/roles/CSRF, SQL projections, repeated writes, concurrency, billing, or signature evidence. Use framework-isolated test databases, verify they differ from the source database, and avoid machine-specific usernames or addresses.
5. **Delivery gates:** project tests, type checks, builds, applicable configuration dry-runs, edited-file diagnostics, formatting, line limits, and diff checks.
6. **Independent review:** focus on runtime/declaration mismatches, streaming backpressure, multi-value headers, and actual success responses. Give parallel workers non-overlapping files, integrate, and rerun; do not overwrite another worker's tests.

Separate online tests from offline gates. Report missing databases, services, or tools as blockers rather than silently skipping failures and claiming success. Do not invent execution counts, performance measurements, or validation evidence.

## 6. Report delivery accurately

Respect the user's and workspace's response-language requirements; the skill's default English authoring language does not change them. Briefly report:

- Actual changes and endpoint/directory scope.
- Standard versions and the distinction between normative fixes and project policies.
- Breaking changes and client adjustments.
- Executed verification, pass counts, and coverage numerator/denominator.
- Ignored online tests, unverified platform constraints, and remaining gaps.

Without authorization, do not commit, deploy, rewrite Git history, or add project READMEs/design documents. Automatic skill selection depends on the host's discovery paths and description; a skill is not an unbypassable enforcement gate.
