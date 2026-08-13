# Pipeline Language Reference

Zebflow supports pipe DSL, graph DSL, JSON graph, and the visual editor. They
describe the same pipeline graph.

## JSON Graph

```json
{
  "kind": "zebflow.pipeline",
  "version": "0.1",
  "id": "hello",
  "entry_nodes": ["start"],
  "nodes": [
    {
      "id": "start",
      "kind": "n.trigger.manual",
      "input_pins": [],
      "output_pins": ["out"],
      "config": {}
    }
  ],
  "edges": []
}
```

`kind` marks the file as a pipeline. `version` is the graph format version. Node
IDs are unique inside the graph. Every edge names a source node, source output
pin, target node, and target input pin.

## Pipe DSL

Pipe DSL writes a simple chain from left to right. Each line starts with `|`.
The next node receives the previous node output.

## Graph DSL

Graph DSL gives nodes labels such as `[start]` and connects them with edge lines.
A named output pin appears after the source label. Use it for branches and match
cases.

## Validation

The parser must reject unknown nodes, duplicate IDs, missing required settings,
bad edge targets, and pins not declared by the node definition. Long script or
SQL bodies are plain content and must not be split as pipeline syntax.

The exact flags for each node come from the live node reference. Do not keep a
second hand written flag list here.

Related source:

- `src/pipeline/model.rs`
- `src/platform/shell/parser.rs`
- `src/platform/services/project.rs`
- `src/pipeline/nodes/basic/mod.rs`
