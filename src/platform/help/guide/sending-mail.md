# Sending mail

Zebflow sends mail through a relay you already have — Fastmail, Postmark, SES,
your university's SMTP, or a local sink while you develop. `n.mail.send` hands
one message over and stops there.

**It is not a mail server.** No queue, no retries, no DKIM signing. Whether a
message reaches an inbox depends on the relay's reputation and on the SPF,
DKIM and DMARC records of the domain you send from — the relay's documentation
is the place that explains those, and it is the same work whether or not
Zebflow is involved.

## 1. Store the relay as a credential

Project settings → Credentials → **SMTP**:

| Field | Value |
|---|---|
| Host | `smtp.fastmail.com` |
| Port | `587` for STARTTLS, `465` for implicit TLS |
| User / Password | the relay login, usually an app password |
| From | `Researchsite <no-reply@research.example>` — the default sender |
| TLS Mode | `starttls` (default), `tls`, `starttls-insecure`, or `none` |

The last two are not for the open internet:

- **`starttls-insecure`** encrypts, then does not check the certificate. It is
  for a mail server you run yourself before it has one a public authority
  signed — a fresh mailbourne, whose STARTTLS is self-signed on first boot.
  Such a server rightly refuses a password over a plain connection, so without
  this mode the only choices would be no encryption or no test. Traffic is
  protected from someone listening and not from someone interposing, so use it
  over a tunnel, a VPN, or localhost, and move to `starttls` the moment a real
  certificate exists.
- **`none`** disables encryption entirely and exists for a localhost test sink.
  Never point it at a real relay: the password would cross in the clear.

The secret is encrypted at rest like every credential
(`guide/credential-encryption`), and the node reads it by id — the password
never appears in a pipeline definition, a trace, or the node's output.

## 2. Send

```
register pipelines/account/activate --
  | trigger.webhook --path /account/activate --method POST
  | n.sekejap.query --collection users --filter "email = $.email"
  | n.mail.send --credential relay
                --to $.email
                --subject "Activate your Researchsite account"
                --text $.message
  | web.response
```

`--to`, `--subject`, `--text`, `--html`, `--from` and `--reply-to` each take
either a literal or a `$.path` into the flowing payload — the same convention
`n.auth.token.create` uses for claims. `$.deep.name` reads a nested field; a
path that matches nothing resolves to empty rather than to the literal text,
so a typo cannot be posted as an address.

Give both `--text` and `--html` and the message goes out as
`multipart/alternative`: the reader's client picks. One of the two is required.

The output is `{ "sent": true, "to": …, "subject": … }` — what was sent and to
whom, never the credential.

## Refusals

| Code | Meaning |
|---|---|
| `FW_NODE_MAIL_CREDENTIAL_MISSING` | no credential with that id in this project |
| `FW_NODE_MAIL_CREDENTIAL_KIND` | the credential exists but is not kind `smtp` |
| `FW_NODE_MAIL_ADDRESS` | the recipient, from, or reply-to is not a valid mailbox — refused before any connection is opened |
| `FW_NODE_MAIL_CONFIG` | neither `--text` nor `--html` was given |
| `FW_NODE_MAIL_SEND` | the relay refused or was unreachable; the message carries the relay's own reason |

## Testing without a relay

Run a local sink — [Mailpit](https://github.com/axllent/mailpit) is one binary
with a web inbox:

```bash
mailpit                     # SMTP on 1025, inbox on http://localhost:8025
```

Then a credential with host `127.0.0.1`, port `1025`, TLS mode `none`, and no
user. Every message your pipelines send lands in the browser inbox instead of
in someone's mailbox, which is where you want them while you are still writing
the wording.

## Running your own server instead of a relay

Nothing above assumes a third party. [mailbourne](https://github.com/mailbourne/mailbourne)
is a mail server that accepts submission on 587 with `AUTH` and delivers
direct to the recipient's MX, so the whole path can be yours:

```
n.mail.send ──AUTH over STARTTLS──▶ mailbourne ──direct to MX──▶ the inbox
```

On the server: `mailbourne domain add <domain>` mints the DKIM key and prints
the DNS to publish, `mailbourne account add zebflow@<domain>` creates the login
this credential uses, and `mailbourne serve --port 587` opens the door. Then
the credential above points at it.

Two things that will bite:

- mailbourne **refuses `AUTH` before TLS**, so `none` cannot reach it. Until
  `mailbourne cert obtain` has fetched a real certificate, the mode is
  `starttls-insecure` and the connection should not cross the open internet —
  bind the server to localhost and reach it over an SSH tunnel or a VPN.
- Sending mail anyone accepts needs SPF, DKIM and DMARC published for the
  domain, and a PTR record from whoever owns the IP. `mailbourne domain show`
  prints the records; the PTR is set in the hosting provider's panel.
