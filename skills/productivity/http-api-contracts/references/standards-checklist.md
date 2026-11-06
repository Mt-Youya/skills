# HTTP, OpenAPI, and JSON Schema Checklist

Execute only items relevant to the current change. Section numbers use RFC 9110 / OAS 3.2.1 / JSON Schema 2020-12 as the baseline; check the source text before citing it.

## HTTP Behavior and Errors

| Item | Checks and verification | Basis / notes |
| --- | --- | --- |
| Safety and idempotency | GET/HEAD must not perform business changes requested by the user; PUT/DELETE have idempotent intended effects; describe POST idempotency-key policy separately | RFC 9110 §9.2; logging/session-maintenance side effects do not automatically violate safe-method semantics |
| HEAD | Send no body; metadata corresponds to GET, but headers determined only by generating content may be omitted. If Content-Length is sent, it must match the GET byte count | §9.3.2, §8.6; not all headers must be literally identical |
| 204 | Empty body, with no Content-Length or Transfer-Encoding; raw HTTP checks must not rely only on `.text()` | §15.3.5, §8.6; also check RFC 9112 for HTTP/1.1 framing |
| 304 | Empty body; if Content-Length is sent, it may only equal the representation byte count of the corresponding 200 | §15.4.5, §8.6; do not apply GET caching semantics to POST |
| 401 | At least one applicable challenge; preserve multiple WWW-Authenticate values | §11.6.1, §15.5.2 |
| Authentication parsing | Schemes are case-insensitive; accept specification-defined spaces between scheme and credentials; preserve the token itself without arbitrary strip/lowercase | §11.1, §11.3, §11.4; check the scheme specification for the specific credential syntax as well |
| Cookie authentication | First identify the supported authentication flow; if no standard challenge is available, discuss status/authentication design. For a project-extension challenge, specify parameters, applicable resources, and client meaning | Do not add Basic/Bearer by default; Session in this implementation is an explicitly disclosed extension, not a universal standard answer; CookieSession is not a standard authentication scheme either and does not justify fabricating a challenge |
| 403 | Use the actual semantics when credentials are valid but authorization/CSRF/policy denies access | §15.5.4; behavior for disabling an active user requires a project decision |
| 405 | Must include the methods currently supported by the target resource; compare sets, without depending on order | §15.5.6, §10.2.1; do not hardcode globally when the router already provides a correct Allow |
| 400/413/415/422 | Check actual branches for input syntax/reading, oversized requests, media types, and unprocessable structure/instructions respectively | §15.5.1, §15.5.14, §15.5.16, §15.5.21; the specific division is contractual, and not all business validation must be 422 |
| 500/502/504 | Handle internal exceptions, invalid upstream responses received as a gateway, and timeouts waiting for upstream separately; connection-failure mappings must match the role and public contract | §15.6.1, §15.6.3, §15.6.5 |
| Length and type | Length is based on actual bytes, not characters; after transforming a body, do not retain old length/digest/encoding; media type and content encoding must correspond to the representation | §8.3, §8.4, §8.6 |
| Unknown APIs | Verify that unmatched paths, unsupported methods, and frontend navigation are not confused; preserve actual 404/405/redirect contracts | JSON for every API is not an RFC mandate; it is a client-consistency requirement |
| Error security | Do not swallow database, network, or extractor failures into incorrect statuses; do not expose internal causes or credentials | Authentication DB failures must not masquerade as "session expired" |

## OpenAPI Documents

- Declare explicit `openapi`, `info`, `jsonSchemaDialect`, and `paths` in the root object; a version string is not proof of generator capability.
- Check method/path against actual registrations, rather than merely comparing two manual lists; independent tests can probe a route's Allow and then verify legal operations.
- Keep `operationId` unique; declare every path-template parameter individually, and parameters with `in: path` must have `required: true`.
- Parameter typed values, serialization, default/clamp/duplicate query policies must match the actual extractors.
- The `requestBody` required flag and allowed media types must match the implementation; do not pass off nonstandard suffix wildcards as support for JSON `+json` media types.
- security: schemes within one requirement are AND, while separate requirements are OR; accurately distinguish Cookie, API Key, Bearer, anonymous CSRF, and administrator roles.
- Describe all response branches, including success, extraction failure, authentication, proxy failure, and business errors; do not declare body-required responses for 204 or HEAD.
- Use the actual `text/csv` media type and download headers for CSV, and `text/event-stream` for SSE. With OAS 3.2 support, `itemSchema` can describe stream items; do not invent provider events.
- Response content defines media-type matching, not descriptive text: exact matches take precedence over `type/*`, then `*/*`. `application/octet-stream` does not mean any type.
- Describe dynamic upstream media using accurate ranges and open binary/JSON boundaries; do not pass off 3.0 `format: binary` as a 2020-12 validation rule.
- Represent response headers as header objects and Content-Type through `content`; authentication challenges and Allow cannot be described only in the body.
- Resolve all `$ref` values with correct reference bases. The OpenAPI Meta-Schema does not necessarily validate Schema Objects; validate them separately.
- Official OpenAPI specification: under Objects and Fields, consult OpenAPI, Parameter, Operation, Security Requirement, Responses, Response, Media Type, and Schema Object.

## JSON Schema 2020-12

| Commonly misunderstood point | Correct handling |
| --- | --- |
| dialect and vocabulary | Declare the `$schema` actually implemented; do not declare a custom required vocabulary without implementing it. The standard validator's Draft/format/network configuration must be traceable |
| Boolean Schemas | `true` accepts all instances, and `false` rejects all; do not support only object Schemas |
| Required fields and null | `required` governs field presence, while a null branch of `type` governs nullability; optional and nullable are not the same |
| Defaults | `default` is an annotation and does not populate values; defaults in deserialization and the object after validation must be consistent |
| Unknown fields | When DTOs ignore fields by default, do not unilaterally set additionalProperties=false everywhere. If the project tightens this, explicitly identify breaking changes and consumer impact |
| Composition and additional fields | `allOf` does not automatically merge each object's additionalProperties; use unevaluatedProperties as needed and verify evaluation semantics for composition and branches |
| Tuples and arrays | 2020-12 uses prefixItems for prefix tuples and items for remaining items; do not mix in older array syntax |
| integer | Mathematical integers include 1.0 and 1e0; native JSON/Rust decoders may reject some lexical representations. Prefer alignment; disclose and test differences that must be retained |
| Native ranges | Do not rely only on format:int64; specify minimum/maximum. JavaScript safe integers differ from the full i64 range; do not silently convert monetary amounts to floating point |
| Character length | minLength/maxLength count characters, not UTF-8 bytes; Rust `.len()` or signature lengths cannot be transferred directly |
| format | An annotation by default; enabling assertion is an explicit policy. Merely declaring format for email/date-time/uuid does not justify claiming strict validation |
| $id / $ref / $defs | Distinguish identifiers, scopes, and reference locations; moving generated artifacts must not break references. Do not recursively fetch arbitrary external URLs for convenience |
| Business constraints | Database existence, authorization, live balances, overflow, byte-based billing, and general cross-field arithmetic remain in the business layer. Use if/then/else or oneOf for expressible conditions; do not invent keywords and treat them as standard |

Acceptance must include at least missing fields, null, extra fields, Unicode, integer boundaries, defaults, boolean Schemas, branches, and references. Generating examples from a Schema and validating them against that same Schema cannot be the sole proof of correctness.

