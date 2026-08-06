# Composite Nodes

Composite nodes are reusable nodes built from Zebflow pipelines.

They should feel like native nodes to users. The implementation detail is that they call inner function pipelines.

Composite node packages must include enough definition metadata for the UI, DSL, validation, MCP, and documentation surfaces to understand the node without hand-written prompts.
