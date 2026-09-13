# zeb/prosemirror

The editing engine under `zeb/ui/editor`. ProseMirror's packages, Zebflow's
block schema, and `createEditor` — which wires them into an editor that
reports to callbacks and draws no UI of its own. Toolbars, menus, colours and
upload belong to the caller: `zeb/ui/editor` is the opinionated one; a project
building its own editor starts from the same schema so its documents still
render everywhere Zebflow renders documents.

```
build/entry.mjs        the source — schema, plugins, createEditor
build/package.json     exact ProseMirror versions (package-lock.json pins them)
build/build.sh         npm ci + esbuild → 0.1/runtime/prosemirror.bundle.mjs, updates manifest integrity
```

Upgrading ProseMirror: bump `build/package.json`, run `build/build.sh`, commit
the bundle and the manifest together.

Documents are ProseMirror JSON (`{ type: "doc", content: [...] }`). Store that;
render HTML from it with `zeb/ui/editor`'s `renderDocumentHtml` / `<DocumentView>`.
