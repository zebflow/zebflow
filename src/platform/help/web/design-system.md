# Design System — `components/ui/`

**Never write raw `<button>`, `<input>`, `<label>`, `<select>` with manual class names.** Always use the ui/ components.

---

## Available Components

```tsx
import Button from "@/components/ui/button";
import Input from "@/components/ui/input";
import Field from "@/components/ui/field";
import Label from "@/components/ui/label";
import Textarea from "@/components/ui/textarea";
import Checkbox from "@/components/ui/checkbox";
import { Select, SelectTrigger, SelectContent, SelectItem } from "@/components/ui/select";
import { Card, CardHeader, CardTitle, CardContent, CardDescription, CardFooter } from "@/components/ui/card";
import { Dialog, DialogTrigger, DialogContent, DialogHeader, DialogTitle, DialogDescription } from "@/components/ui/dialog";
import { Tabs, TabsList, TabsTrigger, TabsContent } from "@/components/ui/tabs";
import Badge from "@/components/ui/badge";
import Separator from "@/components/ui/separator";
import Toggle from "@/components/ui/toggle";
import Alert from "@/components/ui/alert";
import { DropdownMenu, DropdownMenuTrigger, DropdownMenuContent, DropdownMenuItem, DropdownMenuSeparator } from "@/components/ui/dropdown-menu";
import Kbd from "@/components/ui/kbd";
import Markdown from "@/components/ui/markdown";
import CodeEditor from "@/components/ui/code-editor";
```

Full list: `button`, `input`, `field`, `label`, `textarea`, `checkbox`, `toggle`, `select` (+ SelectTrigger/Content/Item), `card` (+ header/title/content/footer/description), `dialog` (+ sub-parts), `tabs` (+ TabsList/Trigger/Content), `badge`, `separator`, `alert`, `dropdown-menu` (+ sub-parts), `kbd`, `markdown`, `code-editor`

---

## Rules

- `inline style=` attributes in TSX → **WRONG**
- Raw `<button class="...">` → **WRONG**
- `<Button variant="primary">` from ui/ → **CORRECT**

---

## For User Project Templates (`shared/ui/`)

Install via MCP: `install_ui_components names=["button","card","dialog"]`

Import from: `@/shared/ui/button`, `@/shared/ui/card`, etc.

The `@/components/ui/` path is the **platform studio** component library — not automatically available in user project templates unless you copied them there.

---

## Tailwind + `cx()` + `tv()`

`cx()` — global for joining class names conditionally:

```tsx
<div className={cx("rounded-lg p-4", isActive && "bg-sky-900", disabled && "opacity-50")}>
```

`tv()` — variant maps for components with many permutations:

```tsx
const badge = tv({
  base: "inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium",
  variants: {
    variant: {
      default: "bg-slate-800 text-slate-200",
      success: "bg-green-900 text-green-200",
      danger:  "bg-red-900 text-red-200",
    },
  },
  defaultVariants: { variant: "default" },
});
```
