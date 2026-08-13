# Pipelines

A pipeline is a graph that receives an input, runs nodes, and returns or emits a
result.

```text
trigger -> read data -> transform -> respond
```

## Three Parts

- A trigger starts the run. It may be a webhook, schedule, function, manual run,
  WebSocket event, MCP call, or KV event.
- Nodes perform work. They can query data, run a script, call HTTP, process a
  file, publish a map, or call another function.
- Edges connect output pins to input pins. An edge controls where data goes.

## Input and Context

`input` is the value passed through edges. A node can replace it with a new
value.

`ctx` describes the run. It includes values such as the request ID, pipeline
identity, trigger details, and verified caller information. It stays available
even when nodes replace `input`.

Use `input` for business data. Use `ctx` for run identity and trigger facts.

## Draft and Active Source

Registering or saving a pipeline updates its source. Activation creates the live
runtime copy. A changed source can therefore be newer than the active copy.
Activate again after testing the new source.

## Authoring Forms

Zebflow supports:

- pipe DSL for a simple line of nodes
- graph DSL for branches and named pins
- JSON graph for the saved form
- the visual graph editor for direct editing

These forms describe the same graph. Use the visual editor or pipe DSL for
simple work. Use graph DSL or JSON when exact branches and pins matter.

## Before Activation

Check that:

- every node kind exists
- every required setting is present
- every edge uses valid pins
- referenced credentials and source files exist
- function input and output schemas are complete
- the pipeline passes a test run

Invalid graphs should fail before activation. A runtime only error for a graph
that could have been checked earlier is a platform defect.

## Large Data

Do not carry a large file or a large table as repeated JSON when a file or table
reference can represent it. Fan out can multiply one payload many times. Keep
large work bounded and pass references where possible.

Use the [pipeline language reference](../reference/pipeline-language.md) and the
generated [node reference](../reference/nodes.md) for exact syntax.
