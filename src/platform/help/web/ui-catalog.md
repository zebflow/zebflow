# UI Component Catalog — cloning from `zeb/ui`

Project pages import components from `zeb/ui` with no install
(`help("web/ui")`). The catalog is for the project that wants to **own** one:
a clone copies the component's source into the project repo.

Cloned components live at `shared/ui/<name>.tsx` under the repo source root
and are imported with the `@/shared/ui/` alias.

## Workflow

```
# 1. See what is available and what is already cloned
list_ui_catalog()

# 2. Clone one or several components
install_ui_components(names=["dialog"])

# 3. Switch the import in the page that should use the copy
import { Dialog, DialogContent, DialogHeader } from "@/shared/ui/dialog"   // the clone
import { Button } from "zeb/ui/button"                                     // still the library
```

`install_ui_components` is idempotent — an existing file is skipped unless
`overwrite=true`. The clone carries a first line naming where it came from;
its `zeb/ui/*` imports keep resolving to the library, so it works unchanged.

## What is in the catalog

Exactly the files of `zeb/ui` — `list_ui_catalog()` reads them from the
library, so the list and the descriptions are the library's own. Every one
is rendered with its variants at `/dev/design-system/ui`.

## Notes

- Colours are theme roles (`bg-primary`, `text-muted-foreground`), defined by
  the project's `globals.css` — a cloned component follows the same theme.
- No npm, no Radix: behaviour is written with `zeb/react` hooks and
  `zeb/ui/hooks`. A clone needs nothing the library did not already ship.
- The studio's own UI is a different kit (`@/components/ui/`), not available
  to project pages.
