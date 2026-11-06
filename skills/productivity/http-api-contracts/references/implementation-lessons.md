# Lessons from This Implementation and Common Pitfalls

This is a list of lessons, not an architecture to copy into every project. It arose from aligning a Rust/axum API origin and a TanStack Start/Cloudflare Workers proxy with HTTP, OAS, and Schema at the same time; by default, apply only the items relevant to the current task.

## Transferable Principles

| Finding in this implementation | How to avoid it next time |
| --- | --- |
| The user said "follow the specification," but local fixes and a full contract were not distinguished first | Confirm the actual defect first, present scope options and breaking-change boundaries, and do not let a single complaint trigger a repository-wide refactor |
| Errors originated along multiple independent paths, so fixing one function could not cover the whole service | Trace back from the externally served interface to all error sources: business logic, extractors, routing, authentication, gateways, and Worker |
| Auth DB failures were all turned into 401 | Separate a missing session from a failed query; a regression with a valid Cookie + closed pool must return 5xx |
| Several locations recognized only the exact Bearer prefix | Unify scheme parsing and test case variants/1+SP while preserving the token's original value; reuse the existing authentication boundary rather than adding another single-purpose framework layer |
| The SQL ApiKey projection lacked group_id, which tests with no records did not expose | Test nonempty data and actual serialization results; DTO derive cannot guarantee that SELECT columns exist |
| Json/Path/Query rejection defaults were plain text, while business errors were JSON | Unify the external contract or describe the branches accurately; verify media types, syntax, size, structure, and business errors while preserving original statuses and key headers |
| axum added length automatically after handler/middleware processing | Check the source for the exact version and test the final service; for 204, wrap the final Router in a service if necessary, rather than treating removal in the handler as completion |
| Meta-Schema validation passed, but a real text/plain provider error had no content declaration | Verify media matching for real HTTP responses; meta-validation proves only that the document structure is valid |
| All Schema-generated examples passed, which did not prove that the API returned the same fields | Keep validation of real success/error responses independent of DTO serialization tests; use nonempty records for arrays and report coverage with a numerator/denominator such as 53/58 |
| Numeric format was only an annotation, while Rust i32/i64 had real ranges | Provide actual range assertions and test differences between mathematical integers and native lexical forms, as well as JS precision differences |
| Rust string len measured bytes, while Schema maxLength measured characters | Preserve and explain byte-based business validation; do not disguise a byte limit as a character limit |
| The proxy copied all headers, lost multiple challenges, and allowed a URL path to replace the origin | Verify standard header containers, dynamic Connection filtering, trusted sources, and URL origin constraints together |
| The SSE producer's send await bypassed the select deadline | Bound every send path, including heartbeats/errors; test task and permit lifetimes when the downstream does not read |
| malformed usage after valid usage did not clear the old evidence | Test event sequences and accounting, not just a single-frame parser; distinguish sticky-invalid evidence from normal events without count fields |
| SSE was buffered to validate successful JSON | In production, validate requests only as needed; response contracts can be validated in tests/development, and SSE must not be read in full by default |
| Parallel agents created test files with the same name, overwriting the previous test suite | Allocate exclusive paths before writing and collect file lists; when the test count drops, check for lost tests instead of celebrating a speedup |
| Completion was announced before regressions had run | After aggregating subtasks, rerun the final full quality gates; distinguish compilation passing, tests passing, and network tests not executed |
| Isolated tests were tied to the local username/host | Obtain configuration from DATABASE_URL and the framework's test pool, verify actual database != source database, and do not modify production environments |
| Test formatting was compressed to fit within 500 lines | Apply the default formatter, then split shared HTTP support, domain fixtures, and media tests by responsibility; do not cut arbitrarily or hide everything on one line |

## Applicability Notes for Rust / axum

- For code-first work, consider Schemars with explicit `draft2020_12()` and generate separate deserialize/serialize Schemas; do not use a mode that supports only OpenAPI 3.0 and then change the version number.
- For the engine, consider the jsonschema library with Draft202012 support, explicitly decide format assertion and network policies for references, and compile and cache at startup/first use rather than rebuilding for each request.
- Validate the original request bytes and then restore the body so business billing and signatures still consume the same bytes; set size/time bounds before reading and arrange cheap auth according to the endpoint's authorization needs to avoid pointless repeated DB validation.
- Expose only the necessary `pub(crate)` access for handler DTO/private modules to support the contract; do not make all internal structures public to generate Schema.
- `get(...)` usually supports HEAD automatically, and MethodRouter may already set Allow. These facts depend on the version; confirm them with final-route tests first.
- Use an outer wrapper on the returned service only when the framework's final length handling actually interferes with 204; preserve make-service/ConnectInfo and run a production build, rather than testing only the internal Router.
- Check reqwest compression features, automatic decoding, and header removal against the pinned version; the encoding observed for JSON/SSE must be consistent with the accounting evidence.
- LengthLimitError differs from a general body transport error; follow the error source to find the actual limit error rather than reporting all read failures as 413.

## Applicability Notes for Fetch / Worker / Clients

- Test request streams, duplex, abort, multivalue Set-Cookie copying, and encoding behavior in the actual Workers runtime; a Node mock cannot guarantee identical Cloudflare capabilities.
- Do not fabricate Via in a Fetch environment without the incoming protocol version; record the limitation and, if necessary, have an edge layer that can obtain the version implement it. Lack of observability is not an exemption clause.
- If validating a new response is genuinely necessary, copy Headers while retaining the body stream; when nothing changes, the original response can be returned. Copying must not combine Cookie values.
- When an error status has no success body, handle it according to the status contract; a 204 client returns undefined instead of attempting JSON parsing. For other JSON successes, do not swallow parse failures into an empty object and report false success.
- Do not modify query scope, themes/i18n, UI appearance, or similar concerns incidentally. Verifying the SSR build and platform dry-run needed for the proxy does not authorize deployment.

## Policies That Must Not Be Generalized

- This project's public `/api/openapi.json`, 422 for JSON structural errors, 404 for unknown APIs, and rejection of upstream redirect are specific decisions, not the only URL/status policies prescribed by the specifications.
- The Session challenge is a disclosed project extension; next time, check existing authentication schemes and available standards first rather than copying it mechanically.
- Unknown fields in upstream JSON remain open, and dynamic provider fields must not be invented locally; tightening them requires an explicit protocol and compatibility authorization.
- Sending requests without trustworthy usage to accounting review is this project's policy; do not add an accounting system to other proxies that have no billing functionality.

