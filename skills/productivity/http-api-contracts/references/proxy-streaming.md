# Proxy, Gateway, and Streaming Boundaries

Load only for proxy/gateway/streaming tasks. Complying with RFC 9110 also requires distinguishing unchanged forwarding from application-layer transformations; do not copy all upstream headers downstream without examining them.

## Request Forwarding

1. Build the target origin from trusted configuration, then assign path/query explicitly; when using `new URL(path, origin)`, check `//host`, absolute URLs, userinfo, and network-access boundaries. Decide whether to preserve a configured base path according to the contract.
2. Parse all `Connection` field values, remove the headers named by their case-insensitive tokens, then remove Connection and the applicable fixed hop-by-hop fields. Maintaining only an allowlist is insufficient for dynamic Connection options (RFC 9110 §7.6.1).
3. Preserve multiple values with standard HTTP header containers. Do not lose WWW-Authenticate values through plain objects/a single get, and do not parse Set-Cookie by splitting on commas.
4. Host/authority must correspond to the target, not the browser-facing site. Whether to forward client Cookie/Authorization depends on the role; a model gateway uses the upstream's own credentials and must not leak platform keys.
5. Forwarded/X-Forwarded-* must come only from trusted proxy configuration, with an explicit trust chain; do not accept user-forged IPs in local or direct connections, or when trusted edge headers are missing.
6. After re-encoding the body, remove the old Content-Length, Content-Encoding, and any representation validators that are no longer valid. Do not modify original bytes before Webhook signature verification or byte-based billing.
7. Propagate appropriate cancellation/abort signals and a bounded request budget; do not buffer streaming requests in full by default.
8. Check Via by role (RFC 9110 §7.6.3): request obligations for proxies/HTTP-to-HTTP gateways differ from those for origins. Use the actual received-protocol and a node name that may be anonymized; if Fetch does not expose the incoming HTTP version, do not fabricate 1.1 or claim full Via compliance on that basis. Report platform limitations and possible implementation paths.

## Response Forwarding

- Preserve necessary end-to-end headers, multivalue challenges, Allow, Retry-After, and Cookie; remove dynamic and fixed hop-by-hop headers. Do not retain a 401/405 status while dropping its required headers.
- A transparent proxy must not arbitrarily modify upstream WWW-Authenticate; an application gateway uses platform credentials, so upstream authentication failure does not imply failure of the user's credentials. First decide whether to preserve the upstream status or map it to a gateway error, and update the contract accordingly.
- Specify media ranges: an OpenAPI declaration containing only application/octet-stream does not automatically cover a provider's text/plain response.
- Choose unchanged compressed forwarding or processing after decoding. When JSON/SSE must be observed, use standard client decoding; after decoding, do not retain encoding headers or the old length. Fail explicitly on corrupt or unsupported encodings.
- "All non-streaming successes must be valid JSON" is a JSON gateway policy, not a universal RFC requirement; do not mistakenly convert a binary proxy into JSON-only behavior.
- Reconstructing a response must not combine Set-Cookie values, corrupt statuses/headers, or introduce an incorrect fixed length. Preserve legal HEAD/304 representation lengths and remove length entirely for 204.
- Explain request IDs and protocol envelopes in errors according to local/upstream boundaries. Update client error wrappers accordingly, but do not rewrite unknown upstream error bytes merely to unify formatting.

## SSE and Accounting

- After committing response headers, the status cannot be changed to 502/504. Transport interruptions must remain observable failures; do not fabricate DONE/terminal events or swallow exceptions into normal success.
- Handle normal HTTP EOF without protocol termination, complete termination without trustworthy usage, corrupt billing evidence, and actual transport failure separately; do not treat every semantic anomaly as a TCP interruption.
- Observing usage must not rewrite unrelated provider frames. When protocol policy permits deliberately hiding usage, handle only that field/event and preserve content and tool deltas.
- Bound backpressure: queue capacity, read timeouts, the total deadline, heartbeat sends, event sends, and the final error send must all be constrained by budgets. An await inside a select branch is not automatically protected by the other branches.
- Release concurrency permits on every exit path; decide settlement, release, or reconciliation based on trustworthy evidence, and do not fabricate zero usage because the downstream disconnects.
- New invalid usage must not silently reuse old valid usage; ordinary events without usage must not clear trustworthy counts. Define sticky-invalid rules for illegal/duplicate termination or malformed usage and test sequences.
- Timeout/interruption errors must not leak URL credentials into logs; server task exits and accounting-operation failures must both be visible.

## Minimal Real Regression Matrix

- Three scheme case variants, single/multiple spaces, and nonempty token precedence, while retaining assertions on upstream capture counts.
- Upstream 401 with two challenges, 405 Allow, headers dynamically named by Connection, and multiline headers.
- gzip/br/deflate/zstd or project-supported encodings, corrupt/unknown encodings, and length and media type after decoding.
- Text/vendor/binary responses validated against the correct OpenAPI media ranges, rather than only passing helper unit tests.
- Large bodies, truncation, connection failures, read timeouts, total timeouts, and upstream redirect policy.
- Interruption after events have been emitted: the client receives content chunk by chunk, then observes a transport error; do not assert only `.text().is_err()` and omit body-integrity checks.
- Downstream remains connected without reading: after the queue fills, the total deadline remains effective, the task ends, the permit is released, and accounting reaches the correct state.
- valid usage → invalid usage → normal event → terminal, plus duplicate termination; untrustworthy old settlement evidence is not reused.
- Validate HEAD/204/304 framing through raw HTTP; ordinary clients may suppress illegal bodies and cannot replace wire-level checks.

