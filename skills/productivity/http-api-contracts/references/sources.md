# Authoritative Sources and Version Policy

## Baseline

RFC 9110, OpenAPI 3.2.1, and JSON Schema 2020-12 were used when this skill was established. `latest` changes; pin supported versions for each project and state the version when quoting source text.

- [HTTP Semantics — RFC 9110](https://www.rfc-editor.org/rfc/rfc9110.html)
- [OpenAPI latest](https://spec.openapis.org/oas/latest)
- [OpenAPI 3.2.1](https://spec.openapis.org/oas/v3.2.1.html)
- [JSON Schema 2020-12 Overview](https://json-schema.org/draft/2020-12)
- [JSON Schema Core](https://json-schema.org/draft/2020-12/json-schema-core)
- [JSON Schema Validation](https://json-schema.org/draft/2020-12/json-schema-validation)
- [JSON Schema 2020-12 Meta-Schema](https://json-schema.org/draft/2020-12/schema)
- [OpenAPI 3.2 Official Document-Structure Meta-Schema](https://spec.openapis.org/oas/3.2/schema/2025-09-17)

Add sources as needed without expanding the original task: consult RFC 9112 for HTTP/1.1 message framing, RFC 9111 for HTTP caching, RFC 6750 for the Bearer scheme, the HTML Living Standard for SSE syntax, and RFC 9457 for Problem Details. These do not replace the user-specified baseline, and new standards must not silently change an existing protocol.

## Citation and Validation

1. When the user provides a URL, fetch that URL directly; do not use page IDs or link fragments as search keywords.
2. A failed fetch, empty content, or truncation does not mean the standard has been read. For disputed provisions, retry with another official format or a specific section; if that still fails, disclose the evidence gap.
3. Do not download every complete standard on every occasion. First locate relevant provisions using the baseline and checklist; fetch the source text only when needed for citation, a version upgrade, or unknown behavior.
4. Check library documentation against the exact version and feature; do not guess APIs from training memory. An OpenAPI 3.0 generator cannot be upgraded to 3.2 by changing the `openapi` value.
5. Use standard libraries to validate document structure, Schema against its Meta-Schema, and data instances, and record each result separately. Explicitly state whether Schema `format` is enforced and how `$ref` resolution is handled.
6. The official OAS document-structure Meta-Schema may not check embedded Schema Objects; the two kinds of meta-validation therefore cannot substitute for each other.
7. In CI, prefer version-pinned official fixtures or reliable offline resolution; external `$ref` sources must be reliable and restricted, without arbitrary network access during validation.
8. If official network tests are ignored by default, claim they passed only after running them separately. Record timeouts, unavailable tools, or network failures as unverified, not as assertions that the specification lacks support.
9. Checklists and explanations are not the full standards. Every MUST conclusion requires an applicable role, version, and source-text basis; distinguish optional provisions from policies explicitly.

