# Expression Reference

String settings may contain `{{ expression }}`. Zebflow resolves expressions
before a node runs.

## Roots

- `$input` is the payload entering the current node.
- `$trigger` is the original trigger data, including auth, parameters, query,
  and headers.
- `$nodes.id` is the output of a completed upstream node with that graph ID.

Examples:

```text
{{ $input.customer.id }}
{{ $trigger.auth.sub }}
{{ $nodes.lookup.rows[0].email }}
```

When the whole setting is one expression, the result keeps its JSON type. When
an expression appears inside other text, its result becomes text.

Only statically named upstream node references are supported. This lets graph
validation know which earlier outputs must be kept. Dynamic node name lookup is
not part of the stable expression contract.

Related source:

- `src/pipeline/expr/scanner.rs`
- `src/pipeline/expr/resolver.rs`
- `src/pipeline/engines/basic.rs`
