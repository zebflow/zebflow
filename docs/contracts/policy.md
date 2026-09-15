# Policy

Status: **survey** — every claim checked against the code on 2026-09-15. The
security policy of an instance: what Zebflow guarantees an operator and a
project, and what Settings → Policy will carry — judged by what happens once
someone's account, laptop or token is in the wrong hands.

## 0. The assumption

**One of the people is compromised.** Not "if": a member's phone is lost, a
password is reused, an editor's laptop is left open. Every rule below is
judged by one question — *what does that one account reach, and what stops it
reaching more?* — never by "we keep attackers out".

## 1. Identities and what each holds

```
platform session   opaque random token · cookie zebflow_session · HttpOnly · SameSite=Strict
                   (Lax only on the office vouch landing) · Secure unless bound to loopback
                   (ZEBFLOW_COOKIE_SECURE overrides) · 24 h · in memory: a restart signs everyone out
MCP session        Bearer token per project · capabilities allow-listed per session (project.read,
                   templates.write, pipelines.execute …) · rotation epoch · auto-reset · revocable
project JWT        n.auth.token.create / n.auth.token.verify · algorithm pinned by the credential,
                   never by the token header (RS256 verifier refuses an HS256 token) · exp always
                   checked · aud checked when set · roles claim as string or array
passwords          argon2 (platform users) · bcrypt available to pipelines (n.crypto)
```

Rules: a token verifies against exactly one credential; two areas that must
never meet use two credentials (`auth_member` / `auth_participant` in
RESEARCHSITE) — the wrong token is not "unauthorised", it is not a token. A project
reads only its own credentials (`owner/project` is the store's key).

## 2. Secrets at rest and in traces

- Credential values are sealed with a versioned data key wrapped by the
  instance key at `<data-root>/platform/credential-key` (mode 0600, or
  `ZEBFLOW_CREDENTIAL_KEY`); every ciphertext names its key; new writes use
  the current key; `POST /api/platform/credentials/reencrypt` rotates
  (`kinds/credential`, `infra/secrets/keyring.rs`). An instance restored
  without its key file refuses to start rather than run with unreadable
  secrets.
- Every value that leaves the credential store is registered per project and
  masked in invocation records before they are written
  (`kinds/invocation-record` rule 3, `services/credential.rs`); a node does
  not have to remember to redact.
- Uploads are typed by magic bytes and the browser's type together
  (`n.fs.save`, `infer`); ZIP-based disguises are refused.

## 3. What a compromised piece can reach

| Compromised | Reaches | Stopped by |
|---|---|---|
| a project user's JWT | that project's routes for its roles, until `exp` | short `exp`; roles in the token; the other area's credential |
| an MCP token | the tools its capabilities allow, in one project | capability list; rotation; revoke in Settings |
| a platform session | the Studio as that user, ≤ 24 h | logout removes it; restart clears all; Strict cookie stops cross-site use |
| a running `n.script` | nothing outside the sandbox: no net, no fs, no env, 1 s | `confinement.md` §1 |
| a node bundle | what its capabilities *declare* — disclosure, not a ceiling | `confinement.md` §0, open |
| the SQLite catalog file alone | no credential value (sealed) | §2 — the key file is separate |

## 4. Not found on 2026-09-15 — the open list

- **Login throttling.** No lockout, delay or per-IP limit on `/login` or on
  `auth.token.create` flows was found. A project must add its own (RESEARCHSITE:
  five wrong codes → ten minutes; codes expire in ten).
- **Security response headers.** No `Strict-Transport-Security`,
  `Content-Security-Policy`, `X-Frame-Options`, `X-Content-Type-Options` are
  emitted; the generated proxy configs (`addressing.md` §6) are where HSTS
  belongs today, CSP is the RWE's to add.
- **CSRF.** No token; the defence is `SameSite=Strict` on the platform cookie,
  which holds for browsers that honour it. Project cookies set by pipelines
  carry whatever the pipeline set.
- **Password policy.** No minimum length or breach-list check on platform
  users.
- **Platform audit trail.** Pipeline runs are recorded (invocation records);
  Studio actions by a user (role changes, credential edits, settings) are not
  written anywhere a reviewer can read after the fact.
- **Session revocation by an admin.** A user can log out; an operator cannot
  end another user's sessions short of restarting.

## Evidence

`web/mod.rs` `session_cookie_header_same_site`, `SESSION_TTL_SECS`,
`logout_submit`; `services/mcp_session.rs`; `nodes/basic/auth_token_verify.rs`
(algorithm pin, `set_audience`); `infra/secrets/keyring.rs`;
`adapters/data/sqlite.rs` keyring report and reencrypt;
`services/credential.rs` confidential registry; `nodes/basic/fs_save.rs`;
`confinement.md`. Absences in §4 are grep results on the same day, not a
design decision — each becomes a row in §1–3 when it lands.
