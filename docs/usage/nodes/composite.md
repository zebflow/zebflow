# Composite Nodes

A composite node gives one clear interface to a function pipeline made from
existing nodes.

Use it when several projects repeat the same pipeline logic, such as sending a
message, making an embedding, validating a file, or publishing a standard map.

## Required Definition

A composite node needs:

- a stable node kind, title, and description
- configuration fields and required credentials
- input and output schemas
- input and output pins
- examples
- clear errors and failure meaning
- the function pipeline that performs the work

The definition is not optional documentation. Installation should reject an
incomplete node package.

## Package Shape

All node bundles use `definition.json` as their package entry. A composite node
points to one or more function pipeline files. Icons and other package files are
optional unless required by the package definition.

## Runtime Meaning

The caller sees one node. Zebflow executes its function pipeline and returns the
declared output. The function receives only the declared input and context. Do
not depend on hidden state from the parent graph.

## Good Composite Design

- Keep one clear purpose.
- Use a small public interface.
- Validate required values before execution.
- Return structured output.
- Keep credentials outside prompts and source files.
- Avoid returning large intermediate data that callers do not need.
- Test the package after a clean install.

Exact package fields are the `NodeBundle` contract, [`docs/contracts/kinds/node-bundle/`](../../contracts/kinds/node-bundle/README.md), summarised for consumers in the help topic `guide/hub/nodes`.
