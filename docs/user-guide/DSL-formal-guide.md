# Zebflow DSL Formal Guide

This guide defines the intended **stable mental model** for the Zebflow DSL.

The DSL should be:

- concise
- guessable
- explicit
- easy for humans to review
- easy for LLMs to generate
- cleanly compilable into one JSON IR

## 1. What the DSL Is

Zebflow DSL is the **text form of the Zebflow flowchart language**.

It is not a general-purpose language like Rust or JavaScript.
It is a backend composition language for:

- sequence
- choice
- coordination
- node configuration

The same backend logic should be representable as:

1. flowchart UI
2. DSL
3. JSON IR

These three forms must mean the same thing.

## 2. Authoring Modes

Zebflow DSL should have two authoring modes:

### 2.1 Sequence mode

Use this for fast, simple, straight-line flows.

Example:

```zf
| trigger.manual
| script -- "return { ok: true };"
| web.response --template pages/ok.tsx
```

This means:

```text
[a] -> [b] -> [c]
```

Sequence mode is only for flows that are naturally linear.

### 2.2 Graph mode

Use this when the flow is non-sequential:

- if
- match
- 1-M fan-out
- M-1 multi-entry
- collect
- foreach
- reduce

Example:

```zf
[a] db.query -- "SELECT * FROM users WHERE id = 42"
[b] http.request --service profile
[c] http.request --service orders
[d] logic.collect
[e] web.response --template pages/user-overview.tsx

[a] -> [b]
[a] -> [c]
[b] -> [d]
[c] -> [d]
[d] -> [e]
```

Graph mode uses simple graph notation like:

```text
[a] -> [b]
[b:true] -> [c]
[m:default] -> [x]
```

Graph mode is the full topology form.
Sequence mode is just the fast shorthand for linear chains.

## 3. Triggers and Execution Frame

Every pipeline starts from a **trigger node**.

Common trigger nodes:

- `trigger.webhook`
- `trigger.schedule`
- `trigger.manual`

Trigger nodes:

- have no input pins
- create the first payload of the run
- define the trigger context available later in expressions

Example:

```zf
[a] trigger.webhook --path /users/:id --method GET
[b] pg.query --params-expr "[$trigger.params.id]" -- "
SELECT *
FROM users
WHERE id = $1
"
[c] web.response --template pages/user.tsx

[a] -> [b]
[b] -> [c]
```

Important rule:

- `$input` changes as the flow moves from node to node
- `$trigger` stays as the original immutable trigger snapshot for the whole run

## 4. Design Doctrine

The DSL should feel closer to:

- `kubectl`
- `git`
- `cargo`
- `bash`

and less like:

- a template DSL
- a workflow toy
- a string interpolation trick

So the surface should be:

- graph-based
- command-first
- explicit about literal vs expression vs body

## 5. The Three Value Modes

Every DSL value should be one of these:

### 5.1 Literal

Used as-is.

```zf
--method POST
--path /checkout
--collect none
--dispatch parallel
```

### 5.2 Expression

Used when a value must be evaluated.

Expression-ness should be explicit in the flag name.

Examples:

```zf
--expr "$input.total > 1000"
--expr "$input.type == 'billing'"
--output-path-expr "$item.slug + '/index.html'"
--step-expr "{ total: $acc.total + $input.item.amount }"
```

Rule:

- `--x` means literal config
- `--x-expr` means evaluated expression
- list flags may be written in either style:
  - repeated: `--cases create --cases update`
  - compact: `--cases create,update`

### 5.3 Body

A body is full node-owned text or code.

Examples:

```zf
[a] script -- "
let total = 0;
for (const row of input.rows) total += row.amount;
return { total };
"
```

```zf
[a] script -- "
let kind = 'billing';
let label = `${input.type}:${kind}`;
return { kind, label };
"
```

```zf
[a] pg.query -- "
SELECT id, email
FROM users
WHERE active = true
"
```

Bodies are not expressions.
They belong to the node.

## 6. Expression Scope

These are the stable runtime values available inside DSL expressions.

### 6.1 General scope

| Name | Meaning |
|---|---|
| `$input` | Current node input payload |
| `$nodes` | Completed node outputs, addressed by node ID |
| `$trigger` | Immutable trigger snapshot |

### 6.2 Foreach scope

These are only present on runs emitted by `logic.foreach`.

| Name | Meaning |
|---|---|
| `$item` | Current foreach item |
| `$index` | Current foreach item index |
| `$count` | Total emitted item count |

### 6.3 Reduce scope

This is only present inside `logic.reduce` expressions.

| Name | Meaning |
|---|---|
| `$acc` | Current accumulator |

### 6.4 What each scope is for

#### `$input`
Use when you want the payload currently entering this node.

```zf
--expr "$input.type == 'billing'"
--expr "$input.total > 1000"
```

#### `$nodes`
Use when you want to reference a specific already-completed node by graph ID.

```zf
--output-path-expr "$nodes.lookup.rows[0].slug + '/index.html'"
```

Rule:

- `$nodes.<node_id>` is only for nodes that have already completed in this run
- it is not a generic “all upstream topology” reference
- it is a completed-node output map

#### `$trigger`
Use when you want original trigger data that should stay stable across the whole flow.

```zf
--expr "$trigger.params.id != ''"
--expr "$trigger.auth.sub"
```

Typical trigger fields:

- `$trigger.auth`
- `$trigger.params`
- `$trigger.query`
- `$trigger.headers`

#### `$item`, `$index`, `$count`
Use inside `logic.foreach` downstream work.

```zf
--output-path-expr "$item.slug + '/index.html'"
--expr "$index == 0"
```

#### `$acc`
Use only inside `logic.reduce`.

```zf
--init-expr "{ total: 0 }"
--step-expr "{ total: $acc.total + $input.item.amount }"
```

### 6.5 Scope rules

- `$input` is the default working payload for the current node
- `$nodes` is for explicit cross-reference to completed nodes
- `$trigger` is original trigger state, not current payload
- there is no general `$ctx` scope in the DSL
- `$acc` is not a general expression variable; it belongs only to `logic.reduce`
- there is no general `$output` scope in the DSL

## 7. Body Language Style

The DSL should stay strict, but body languages should stay native.

For **JS-like bodies** such as `script`:

- prefer `'...'` for ordinary strings
- use `` `...` `` for interpolation
- avoid `"` by convention unless needed

For **non-JS bodies**, follow the native language:

- SQL uses SQL quoting
- JSON uses JSON quoting
- HTML/TSX uses template-native quoting

The rule is:

- strict DSL
- native body languages
- one style guide per body language, not one fake quote rule for everything

## 8. Core Ontology

The stable first-principle model should be:

1. **Sequence**
- sequential dependency is the basic supported operation

2. **Choice**
- `if`
- `match`

3. **Coordination**
- `1-M` auto fan-out
- `M-1` auto multi-entry, independent by default
- `collect`
- `foreach`
- `reduce`

4. **Execution policy**
- dispatch policy belongs here, not in the coordination ontology
- examples:
  - `dispatch=sequential`
  - `dispatch=parallel`

5. **Resilience / feedback**
- explicit resilience layer, for example `retry`

This separation matters:

- sequence / choice / coordination describe the graph semantics
- execution policy describes how work is scheduled
- resilience describes what happens under failure or repetition

## 9. Core Logic Vocabulary

Current core logic set:

- `if`
- `match`
- `collect`
- `foreach`
- `reduce`

The graph semantics also include:

- `1-M` auto fan-out
- `M-1` auto multi-entry, independent by default

Execution policy is separate from this vocabulary.

Resilience is also separate:

- `retry`

## 10. Core Logic Examples

### 10.1 `if`

```zf
[a] trigger.webhook --path /tickets --method POST
[b] logic.if --expr "$input.type == 'billing'"
[c] db.query -- "
SELECT *
FROM invoices
WHERE customer_id = $input.customer_id
"
[d] db.query -- "
SELECT *
FROM tickets
WHERE customer_id = $input.customer_id
"
[e] web.response --template pages/billing-result.tsx
[f] web.response --template pages/general-result.tsx

[a] -> [b]
[b:true] -> [c]
[b:false] -> [d]
[c] -> [e]
[d] -> [f]
```

### 10.2 `match`

```zf
[a] trigger.webhook --path /events/stripe --method POST
[b] logic.match --expr "$input.type" --cases invoice.paid --cases customer.subscription.deleted --default default
[c] script -- "
return { kind: 'invoice' };
"
[d] script -- "
return { kind: 'subscription' };
"
[e] script -- "
return { kind: 'other' };
"

[a] -> [b]
[b:invoice.paid] -> [c]
[b:customer.subscription.deleted] -> [d]
[b:default] -> [e]
```

Short form is also valid:

```zf
[b] logic.match --expr "$input.type" --cases invoice.paid,customer.subscription.deleted --default default
```

### 10.3 `1-M` auto fan-out

```zf
[a] db.query -- "SELECT * FROM users WHERE id = 42"
[b] http.request --service profile
[c] http.request --service orders
[d] http.request --service invoices

[a] -> [b]
[a] -> [c]
[a] -> [d]
```

Meaning:

- one output from `a`
- reused by many downstream nodes
- no explicit coordination node needed

### 10.4 `M-1` auto multi-entry

```zf
[a] db.query -- "SELECT * FROM users WHERE id = 42"
[b] http.request --service profile
[c] http.request --service orders
[d] http.request --service invoices
[e] script -- "
return input;
"

[a] -> [b]
[a] -> [c]
[a] -> [d]
[b] -> [e]
[c] -> [e]
[d] -> [e]
```

Meaning:

- `e` is entered independently by `b`, `c`, and `d`
- no implicit grouping happens

### 10.5 `collect`

```zf
[a] db.query -- "SELECT * FROM users WHERE id = 42"
[b] http.request --service profile
[c] http.request --service orders
[d] http.request --service invoices
[e] logic.collect
[f] web.response --template pages/user-overview.tsx

[a] -> [b]
[a] -> [c]
[a] -> [d]
[b] -> [e]
[c] -> [e]
[d] -> [e]
[e] -> [f]
```

Meaning:

- `e` groups multiple independent upstream results
- `f` runs once with grouped input

### 10.6 `foreach`

```zf
[a] trigger.manual
[b] logic.foreach --items-path /entries --dispatch parallel --concurrency 8
[c] web.static.generate --template pages/musiklib/lyric.tsx --site-root static/musiklib --output-path-expr "$item.slug + '/index.html'"

[a] -> [b]
[b:item] -> [c]
```

Meaning:

- one collection becomes many emitted runs
- dispatch policy is explicit on `foreach`

### 10.7 `reduce`

```zf
[a] trigger.manual
[b] logic.foreach --items-path /rows --dispatch seq
[c] script -- "
return { amount: input.item.amount };
"
[d] logic.reduce --init-expr "{ total: 0 }" --step-expr "{ total: $acc.total + $input.amount }"

[a] -> [b]
[b:item] -> [c]
[c] -> [d]
```

Meaning:

- `reduce` folds many emitted results into one ordered result

## 11. Why Each Core Logic Exists

### `if`
- binary decision
- choose one of two paths

Typical need:
- approve vs review
- found vs not-found
- success vs fallback

### `match`
- multi-case dispatch
- cleaner than chaining many `if` nodes

Typical need:
- ticket type routing
- webhook event routing
- mode / state dispatch

### `collect`
- group multiple independent upstream results so they are processed together

Typical need:
- build one response from several independent fetches
- combine several service outputs into one render step

### `foreach`
- one source emits many downstream runs
- may be plain collection emission or larger batch emission

Typical need:
- process many rows/items
- batch static generation
- big-data chunking by policy

### `reduce`
- accumulate many emitted results into one ordered result

Typical need:
- totals
- grouped summary
- final batch report

## 12. String Expression Examples

String-valued expressions deserve explicit examples because quoting is usually the confusing part.

```zf
--expr "$input.type == 'billing'"
--expr "$input.status != 'archived'"
--label-expr "$input.type == 'billing' ? 'finance' : 'general'"
--output-path-expr "$item.artist_slug + '/' + $item.song_slug + '/index.html'"
```

General rule:

- the whole expression is one DSL string
- string values inside the expression use normal expression-language quotes
- prefer single quotes inside the expression when the outer DSL value already uses double quotes

## 13. Mapping to JSON IR

The DSL must compile into one stable IR.

Conceptually:

- literal flag → literal config value
- `-expr` flag → expression config value
- body → node-owned body field
- sequence mode → linear graph shorthand
- graph edges → explicit dependency topology
- logic nodes → explicit graph semantics

If a DSL construct cannot map cleanly into JSON IR, it should not become part of the stable DSL contract.

## 14. Practical Summary

The stable rules should be:

1. The DSL is the text form of the flowchart language.
2. The DSL has two authoring modes:
   - sequence mode for linear chains
   - graph mode for non-sequential topology
3. Literal, expression, and body are separate concepts.
4. `--x-expr` marks evaluated expressions.
5. Sequence, choice, and coordination are the core logic layers.
6. Execution policy is separate from logic.
7. Resilience is separate from coordination.
8. JS-like bodies prefer `'...'` and `` `...` ``, while other body languages stay native.
9. The DSL must stay easy for humans, easy for LLMs, and cleanly mappable to one JSON IR.

## 15. Status

This guide defines the intended formal direction.

Some current implementation details may still be converging toward this model.
But this should be the target foundation for:

- DSL cleanup
- control-flow redesign
- flowchart UI semantics
- JSON IR stability
