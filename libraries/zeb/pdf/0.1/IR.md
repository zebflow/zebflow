# Zeb PDF IR

`zeb/pdf` uses a canonical document IR as the source of truth for authored PDF documents.

This IR is intended to be shared across:

- ZebFlow authoring/builders
- browser-side PDF rendering in `libraries/zeb/pdf`
- Rust-side ingestion and writing in `pdfwrangler`

## Design rules

1. Authored structure is canonical.
2. Extraction metadata is optional.
3. Engines may add metadata, but they should not fork the structural model.
4. Interchange should use JSON.

## Core document shape

At the top level:

```json
{
  "type": "document",
  "version": "zeb-pdf-ir/1",
  "meta": {},
  "settings": {},
  "styles": {},
  "children": []
}
```

## Canonical node families

Structural/source-of-truth nodes:

- `document`
- `page`
- `text`
- `image`
- `table`
- `row`
- `cell`
- `container`
- `columns`
- `float`
- `absolute`
- `relative`

Allowed forward-compatible authored nodes:

- `paragraph`
- `heading`
- `list`
- `list_item`
- `math_inline`
- `math_block`

Low-level draw nodes:

- `line`
- `rect`

## Optional extraction metadata

Extracted nodes from `pdfwrangler` may add:

- `bbox`
- `page_index`
- `confidence`
- `engine`
- `reading_order`
- `source`
- `warnings`

These fields are optional and must not be required for authored IR.

## Interop contract

- ZebFlow builder/DSL compiles to canonical IR.
- `zeb/pdf` renders canonical IR to PDF in the browser.
- `pdfwrangler` may:
  - read PDF into extracted IR
  - render canonical IR to PDF
  - export canonical/extracted IR to JSON

## Validation

The machine-readable schema for this contract is:

- [ir.schema.json](./ir.schema.json)

