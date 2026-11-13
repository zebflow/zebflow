# Webhook signatures from any vendor

## What this builds

A webhook route that accepts a vendor's deliveries only when they carry a valid
signature. The vendor signs each delivery with a secret you both know; Zebflow
rebuilds what the vendor signed, signs it with the same secret, and compares.
A delivery that does not match is refused with `401` before any node runs, so
the pipeline only ever sees real deliveries.

```
trigger.webhook --route /hooks/vendor --method POST --auth hmac --credential vendor-hooks
```

The route names the credential. The credential holds the whole check: what is
signed, where the signature and timestamp are, and the secrets. A new vendor is
a new credential, not new code.

---

## The credential (kind `hmac`)

Credentials → New → HMAC. Pick the vendor if it is listed; every
field stays editable. Otherwise choose **Custom** and copy the settings from the
vendor's documentation. The same settings, as the API and MCP save them:

```json
{
  "secrets": { "current": "the-signing-secret", "previous": "" },
  "algorithm": "sha256",
  "signed": "{timestamp}.{body}",
  "signature": { "from": "header:X-Vendor-Signature", "prefix": "v1=", "encoding": "hex" },
  "timestamp": { "from": "header:X-Vendor-Timestamp", "unit": "seconds", "tolerance": 300 }
}
```

| Setting | Meaning |
| --- | --- |
| `secrets.current` | The signing secret. Required. |
| `secrets.previous` | Optional. While the vendor moves to a new secret, the old one here keeps deliveries passing. A delivery passes when it matches either. |
| `secret_encoding` | `text` (default), `base64` or `hex`: how the secret is written. A text secret is used byte for byte, spaces included. Standard Webhooks secrets are base64. |
| `secret_prefix` | Removed from the secret before use, e.g. `whsec_`. |
| `algorithm` | `sha256` (default), `sha512` or `sha1`. |
| `signed` | The text the vendor signs, as a template (below). |
| `signature.from` | Where the signature is: `header:Name`, or `part:key` (below). |
| `signature.prefix` | Text before the signature, e.g. `sha256=` or `v1=`. A delivery whose signature lacks it is refused as `HMAC_PREFIX_MISSING`. |
| `signature.encoding` | `hex` (default) or `base64`. |
| `signature.split` | When one header carries several signatures, what separates them (a space for Standard Webhooks). Each is tried. |
| `timestamp` | Optional. `from` as above, `unit` `seconds` or `milliseconds`, `tolerance` in seconds (default 300, at most 86400). A delivery further from now than the tolerance is refused, so a copied delivery cannot be replayed later. The timestamp must be part of `signed` (`{timestamp}`, or its own `{header:…}` / `{part:…}`), otherwise a copy could pass with a new time and the credential is refused. |
| `parts` | Optional, for a header made of `key=value` pieces such as `t=…,v1=…`: `from` (`header:Name`), `separator` (default `,`), `assign` (default `=`). Then `part:key` reads one piece. |
| `preset` | Which preset filled the form, for the reader. It changes nothing. |

A credential that only signs or checks text in a pipeline
(`crypto.signature.sign` / `crypto.signature.verify`) needs only `secrets`
(and `secret_encoding` / `secret_prefix` when the key is encoded); those nodes
use `secrets.current`. `signed` and `signature` are needed only for
`--auth hmac`.

### The signed text

`signed` is literal text with these pieces; nothing else is allowed, and a
mistake is refused when the credential is saved, not when a delivery arrives.

| Piece | Value |
| --- | --- |
| `{body}` | The request body, the raw bytes as delivered, never parsed. |
| `{timestamp}` | The timestamp exactly as sent. Needs `timestamp`. |
| `{header:Name}` | Any request header's value. |
| `{part:key}` | One piece of the `parts` header. |
| `{method}` | `POST`, `GET`, … |
| `{path}` | The path the vendor called. |
| `{url}` | The full address the vendor called: the scheme from the proxy's `X-Forwarded-Proto` (else `http`), the `Host` the request was routed by, the path and the query. It must equal the address the vendor has. |

`{{` and `}}` write a literal brace.

---

## Presets

| Vendor | Signed | Signature | Timestamp |
| --- | --- | --- | --- |
| GitHub | `{body}` | `X-Hub-Signature-256`, after `sha256=`, hex | — |
| Meta (Instagram, WhatsApp, Facebook, Messenger) | `{body}` | `X-Hub-Signature-256`, after `sha256=`, hex | — |
| Shopify | `{body}` | `X-Shopify-Hmac-Sha256`, base64 | — |
| Slack | `v0:{timestamp}:{body}` | `X-Slack-Signature`, after `v0=`, hex | `X-Slack-Request-Timestamp` |
| Zoom | `v0:{timestamp}:{body}` | `x-zm-signature`, after `v0=`, hex | `x-zm-request-timestamp` |
| Stripe | `{timestamp}.{body}` | every `v1` part of `Stripe-Signature`, hex | its `t` part |
| Paddle Billing | `{timestamp}:{body}` | the `h1` part of `Paddle-Signature` (`;` between parts), hex | its `ts` part |
| Square | `{url}{body}` | `x-square-hmacsha256-signature`, base64 | — |
| Standard Webhooks | `{header:webhook-id}.{timestamp}.{body}` | `webhook-signature`, every `v1,` entry, base64; secret `whsec_` + base64 | `webhook-timestamp` |
| Svix | as Standard Webhooks, `svix-` header names | | |

A preset fills the form; the saved credential keeps its own copy, so a later
change to a preset never changes a check that is already live.

---

## Testing with a real delivery

Once the credential is saved, its form shows **Test with a delivery**: paste
the headers and body from the vendor's dashboard and press Check. It answers
whether the delivery passes, or which step fails. Nothing is stored. A pasted
delivery is older than any tolerance, so the time window is ignored by default;
the timestamp is still read and still signed.

## When a delivery is refused

The vendor gets `401` with `{"ok": false, "error": "<CODE>: <sentence>"}`, and
the pipeline's error list in the registry shows the same code and sentence. No
run starts.

| Code | Means |
| --- | --- |
| `HMAC_HEADER_MISSING` | The signature header, the parts header, or a header `signed` uses is absent. |
| `HMAC_TIMESTAMP_MISSING` | `timestamp` is set but the delivery carries none. |
| `HMAC_TIMESTAMP_INVALID` | The timestamp is not a whole number. |
| `HMAC_TIMESTAMP_STALE` | The timestamp is further from now than the tolerance. |
| `HMAC_PREFIX_MISSING` | No signature starts with `signature.prefix`. |
| `HMAC_SIGNATURE_UNREADABLE` | The signature is not valid hex or base64. |
| `HMAC_SIGNATURE_MISMATCH` | Everything was read, and no signature matches either secret. Usually the wrong secret, a different `signed` text, or a body changed on the way. |

Neither the refusal nor the error list ever holds the secret, a signature, or
the signed text. The signature and timestamp headers are not in the pipeline's
`webhook.headers`, and the pipeline receives the parsed body, not the raw bytes.

## Not covered

- Public-key signatures (Ed25519: Discord, Standard Webhooks `v1a`).
- Vendors that sign the URL with sorted form fields (Twilio) or put the
  signature inside the body.
- Refusing a repeated delivery by its event id. The timestamp window limits
  replay; a pipeline that must act once per event keeps the id itself.
