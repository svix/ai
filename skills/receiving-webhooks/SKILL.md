---
name: receiving-webhooks
description: >-
  General guidelines for building a robust webhook receiver/handler:
  verifying signatures, raw-body access, replay protection, async processing, 
  idempotency/deduplication on the event ID, retries and endpoint auto-disabling. 
  Use whenever you write, review, or debug a handler that consumes incoming webhooks from any provider.
---

# Receiving Webhooks

A webhook is just an HTTP POST from a source you don't control. Treat every
request as untrusted until its signature is verified. These guidelines apply to
any handler consuming webhooks, regardless of which provider or platform sends
them.

Header names, secret formats, and tolerances vary by provider. The names below
are illustrative; always confirm the exact header names, signing scheme, and
replay window against your provider's documentation before writing code.

## The non-negotiables

1. **Verify the signature on every request.** An unverified webhook is an
   anonymous internet POST. Anyone who learns your URL can forge events. Only
   act on payloads that pass verification.
2. **Verify against the *raw* request body.** The signature is computed over the
   exact bytes sent. Any framework that parses JSON and re-stringifies it breaks
   verification. Read the unprocessed body.
3. **Return a `2xx` within seconds.** Anything else, including `3xx`
   redirects, is typically treated as a failure and retried. Push heavy work async.
4. **Be idempotent.** Retries mean the same event can arrive more than once.
   Dedupe on the provider's unique event/message ID.

## Handler shape

1. **Read the raw body.** Do not parse JSON before verification.
2. **Verify** the signature using the provider's official library (or its
   documented scheme), passing the signature headers and your signing secret. On
   failure, return `400`.
3. **Dedupe** on the unique event ID (see below). If already processed, return
   `2xx` without re-running side effects.
4. **Acknowledge fast.** Return `2xx` (e.g. `204`) immediately; do real work in
   a background job/queue if it can exceed a second or two.
5. **Branch on the payload** (event type) and process.

```ts
// Pseudocode: shape is the same across providers/languages.
// rawBody must be the RAW request body (string/bytes), not parsed JSON.
let payload;
try {
  payload = verifyWebhook(rawBody, req.headers, signingSecret);
} catch (err) {
  return res.status(400).send(); // verification failed, reject
}
// payload is now trusted; enqueue and ack
return res.status(204).send();
```

Prefer the provider's official verification library when one exists: it validates
the signature **and** the timestamp tolerance and handles replay protection for
you, so you don't reimplement crypto by hand.

## Signature headers and the secret

Most providers send some combination of:

| Concept | Typical header | Purpose |
|--------|---------|---------|
| Event/message ID | e.g. `x-*-id` | Unique identifier (**the dedupe key**) |
| Timestamp | e.g. `x-*-timestamp` | Send time, used for replay protection |
| Signature | e.g. `x-*-signature` | HMAC signature(s), often versioned |

- **Use the signing secret, not an API key.** The secret used to verify inbound
  webhooks is usually distinct from the API token you use to call the provider's
  management API.
- **Keep the secret server-side.** Never ship it in client bundles or commit it.

## Idempotency / deduplication

Retries and network blips cause the same event to be delivered more than once.
The provider's event/message ID header is **stable across retries of the same
message**, so use it as the dedupe key.

- Insert the event ID into a "processed events" table with a **unique
  constraint**.
- On a duplicate-key violation, return `2xx` and skip re-processing. Don't
  re-run side effects (charging a card, sending email, mutating state).
- Make the side effect and the "mark processed" write atomic (same transaction)
  where possible, so a crash mid-processing doesn't strand a half-applied event.

## Responding, retries, and auto-disable

- **Only `2xx` means success.** Every other code (often including `3xx`) is
  treated as a failure and retried, usually on an exponential-backoff schedule.
- **Respond within the provider's timeout** (commonly a few to ~15 seconds). If
  processing can take longer, ack first and work async.
- **Use `4xx` to reject bad/forged requests** (failed verification returns
  `400`); use `5xx`/timeouts only for genuine transient failures you want retried.
- **Endpoints often auto-disable** after sustained failure. Many providers notify
  you of this (e.g. via an operational/alert webhook or email). Keep your handler
  healthy to avoid silent disabling, and wire up failure notifications.

## Manual verification (only when no official library exists)

Prefer the provider's official library. If your language genuinely has none,
follow the provider's documented signing scheme exactly and **don't invent your
own**. A common HMAC scheme looks like:

1. Build the signed content by concatenating the documented fields (often
   `id.timestamp.body`) with the body unmodified.
2. Compute HMAC (commonly SHA-256), keyed by the signing secret (decode it first
   if the provider documents a base64/hex-encoded secret).
3. Compare against the signature header, stripping any version prefix (e.g.
   `v1,`). Your computed value must match one of the provided signatures.
4. **Constant-time comparison.** Never `==` on signatures (timing attacks).
5. **Reject stale timestamps.** Enforce the documented tolerance window yourself.


## Verification traps (debugging a failing handler)

- **Body parsed before verification.** Re-stringification changes the bytes. Use
  raw-body access.
- **Wrong secret.** Ensure you're using the correct signing secret
- **Secret in the wrong format.** Some providers expect the secret decoded from
  base64/hex, or with a documented prefix stripped.
- **Replaying old captured payloads with curl.** Verification rejects stale
  timestamps; trigger a fresh delivery (e.g. the provider's "resend") instead.
- **Reverse proxy stripping signature headers.** Check proxy config.
- **Clock skew.** Unsynced server time fails timestamp validation.


## Checklist

- [ ] Signature verified on every request (official library preferred)
- [ ] Verification runs against the **raw** body (no pre-parse)
- [ ] Failed verification returns `400`
- [ ] Handler returns `2xx` within the provider's timeout; heavy work is async
- [ ] Dedupe on the event ID with a unique constraint; duplicates return `2xx` no-op
- [ ] Signing secret used, kept server-side