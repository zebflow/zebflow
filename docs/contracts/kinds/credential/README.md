# Credential

Status: pending review

This contract defines one stored credential record: the durable, secret-bearing
value a project holds so a node can authenticate. It is **not** the credential
*type* definition, which a `NodeBundle` declares and the platform registry
merges.

Its review will cover the record shape, secret storage and encryption at rest,
rotation, the platform and project scopes, and the rule that a credential value
is never distributed.

## Why it exists

Recorded during the distribution review. Credential values are durable, survive
upgrades, and are stored through the `DataAdapter` with no contract governing
them. Project transfer excludes them today, which is correct behaviour arrived
at by omission rather than by a stated rule.

Distribution states the rule: a package may declare the credential *kinds* it
needs, and never carries a secret. This kind owns the other side of that rule —
what a stored credential is, and what protects it.
