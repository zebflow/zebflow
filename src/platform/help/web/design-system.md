# Design System — `components/ui/`

**Never write raw `<button>`, `<input>`, `<label>`, `<select>` with manual class names.** Use the ui/ primitives. Every one of them is rendered with its variants at `/dev/design-system`; a primitive missing from that page fails the build.

Colours are theme tokens (`bg-card`, `text-muted-foreground`, `text-destructive`), never palette classes — see `help("web/tailwind")` § Theme Tokens.

---

## Imports

One file per component. Sub-parts are their own files, default-exported;
the few multi-export files are named below.

```tsx
// actions
import Button from "@/components/ui/button";               // variant: primary | secondary | outline | ghost | destructive | link | live · size: lg | md | sm | xs | icon
import Toggle from "@/components/ui/toggle";               // a switch: label, checked, onChange, disabled
import Kbd from "@/components/ui/kbd";
import DropdownMenu from "@/components/ui/dropdown-menu";  // trigger={…} align="left|right"; items as children
import DropdownMenuItem from "@/components/ui/dropdown-menu-item";        // label, icon, variant="destructive", onClick
import DropdownMenuSeparator from "@/components/ui/dropdown-menu-separator";
import DropdownMenuContent from "@/components/ui/dropdown-menu-content";  // the panel alone, for a custom anchor
import ContextMenu from "@/components/ui/context-menu";    // items=[{ label, onSelect, variant?, disabled? } | { separator: true }]

// forms
import Input from "@/components/ui/input";
import Textarea from "@/components/ui/textarea";
import { Select, SelectOption } from "@/components/ui/select";  // native select; value + onChange
import Checkbox from "@/components/ui/checkbox";           // compact, mono label
import CheckboxField from "@/components/ui/checkbox-field"; // label + description
import Label from "@/components/ui/label";
import Field from "@/components/ui/field";                 // label + optional description tooltip + the control
import FolderPicker from "@/components/ui/folder-picker";  // owner, project, value, onChange
import MapPicker from "@/components/ui/map-picker";        // open, onOpenChange, value, onSave, onClear
import TraceCaptureFields from "@/components/ui/trace-capture-fields";

// overlays
import { Dialog } from "@/components/ui/dialog";           // open, onOpenChange
import DialogContent from "@/components/ui/dialog-content"; // size: sm | md | lg | xl | wide
import DialogHeader from "@/components/ui/dialog-header";
import DialogTitle from "@/components/ui/dialog-title";
import DialogDescription from "@/components/ui/dialog-description";
import DialogFooter from "@/components/ui/dialog-footer";
import ConfirmDialog from "@/components/ui/confirm-dialog"; // title, message, confirmLabel, variant="destructive", busy
import CommitDialog from "@/components/ui/commit-dialog";   // section, defaultMessage, onConfirm(message), onCancel
import HelpTooltip from "@/components/ui/help-tooltip";     // text
import Sonner from "@/components/ui/sonner";                // toasts=[{ id, msg, variant }]

// navigation
import Tabs from "@/components/ui/tabs";
import TabsList from "@/components/ui/tabs-list";
import TabsTrigger from "@/components/ui/tabs-trigger";    // label, active, onClick, disabled
import TabsContent from "@/components/ui/tabs-content";    // active
import { StudioTabNav, StudioTabLink } from "@/components/ui/studio-tab-nav";  // route tabs under a studio header
import Separator from "@/components/ui/separator";         // orientation="vertical"
import TreeView from "@/components/ui/tree-view";
import TreeItem from "@/components/ui/tree-item";          // isFolder, expanded, href, active
import HierarchyTree from "@/components/ui/hierarchy-tree"; // items=[{ id, label, children, … }]
import WebhookRouteTree from "@/components/ui/webhook-route-tree";  // pipelines grouped by webhook path
import RepoTree from "@/components/ui/repo-tree";          // owner, project, selected, onSelect, onAction
import RepoTreeRow from "@/components/ui/repo-tree-row";

// data
import Card from "@/components/ui/card";
import CardHeader from "@/components/ui/card-header";
import CardTitle from "@/components/ui/card-title";
import CardDescription from "@/components/ui/card-description";
import CardContent from "@/components/ui/card-content";
import CardFooter from "@/components/ui/card-footer";
import Badge from "@/components/ui/badge";                 // variant: default | secondary | destructive | outline
import Alert from "@/components/ui/alert";                 // variant: info | success | warning | error
import { StudioTable, StudioThead, StudioTh, StudioTd } from "@/components/ui/studio-data-table";
import FileKindIcon from "@/components/ui/file-kind-icon"; // name
import ColorSwatch from "@/components/ui/color-swatch";    // name, value

// markdown is a Zeb library, not a ui/ file
import { Markdown } from "zeb/markdown";                   // <Markdown content={text} />
```

---

## Rules

- `inline style=` attributes in TSX → **WRONG**
- Raw `<button class="...">` → **WRONG**
- `<Button variant="primary">` from ui/ → **CORRECT**
- `bg-red-500`, `text-gray-400`, `text-white` → **WRONG** — `text-destructive`, `text-muted-foreground`, `text-primary-foreground`

---

## For project pages: `zeb/ui`

Project pages do not use `@/components/ui/` — that is the studio's own kit.
They import from **`zeb/ui`**, the same shadcn-shaped components with no
install: `import { Button } from "zeb/ui/button"`. To own one, clone it
(`install_ui_components names=["dialog"]`) and import `@/shared/ui/dialog`.
See `help("web/ui")`.

---

## Tailwind + `cx()`

**Class discovery is automatic.** The compiler scans all string literals in your template source — conditional classes, `cx()` arguments, and components hidden during SSR (e.g. `<Dialog>` when closed) are all covered. No ghost span needed; `tw-variants` is only needed for pure runtime-computed strings with no literal form in code.

`cx()` — `import { cx } from "zeb/react"` for joining class names conditionally:

```tsx
<div className={cx("rounded-lg p-4", isActive && "bg-accent", disabled && "opacity-50")}>
```

Variant maps are plain objects of literal strings joined by `cx` — see
`help("web/tailwind")` § Variant maps. There is no `tv()` and no `cva`.
