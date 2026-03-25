# UI Component Catalog

Zebflow ships 38 shadcn-compatible Zeb React components that can be installed into any project.

Installed components live at `repo/pipelines/shared/ui/` and are imported with the `@/shared/ui/` alias.

## Workflow

```
# 1. See what's available and what's already installed
list_ui_catalog()

# 2. Install one or several components
install_ui_components(names=["button", "dialog", "input"])

# 3. Use in any template
import Button from "@/shared/ui/button"
import { Dialog, DialogContent, DialogHeader } from "@/shared/ui/dialog"
```

`install_ui_components` is idempotent — already-installed files are skipped unless `overwrite=true`.

## Full Component List

### Primitives
| Name | Description |
|------|-------------|
| `button` | Accessible button with variant and size props |
| `input` | Text input with consistent styling |
| `textarea` | Multi-line text input |
| `label` | Form label with peer-disabled support |
| `checkbox` | Checkbox with onCheckedChange API |
| `radio-group` | Radio group with single selection |
| `switch` | Toggle switch with checked/onCheckedChange |
| `slider` | Range slider with onValueChange |

### Display
| Name | Description |
|------|-------------|
| `badge` | Inline status badge with variants |
| `avatar` | Avatar with image and fallback |
| `progress` | Progress bar 0–100 |
| `skeleton` | Loading skeleton placeholder |
| `separator` | Horizontal or vertical divider |
| `kbd` | Keyboard shortcut display |
| `alert` | Alert banner with title and description |

### Layout
| Name | Description |
|------|-------------|
| `card` | Card with header, content, and footer |
| `table` | Styled HTML table with all sub-parts |
| `tabs` | Tab panels with internal active state |
| `accordion` | Collapsible accordion, single or multiple |
| `collapsible` | Simple open/close collapsible container |
| `scroll-area` | Styled scrollable container |

### Navigation
| Name | Description |
|------|-------------|
| `breadcrumb` | Breadcrumb nav with all sub-parts |
| `pagination` | Page pagination with previous/next |
| `toggle` | Pressable toggle button |
| `toggle-group` | Toggle group with single or multiple selection |

### Overlay
| Name | Description |
|------|-------------|
| `dialog` | Modal dialog with backdrop and close button |
| `alert-dialog` | Confirmation dialog, no outside-click dismiss |
| `sheet` | Slide-in panel from any edge |
| `drawer` | Bottom drawer sheet |
| `popover` | Anchored popover panel |
| `hover-card` | Content card shown on hover |
| `tooltip` | Tooltip shown on hover/focus |
| `dropdown-menu` | Dropdown menu with items, checkboxes, radios |

### Complex
| Name | Description |
|------|-------------|
| `select` | Custom select with item list |
| `sonner` | Toast notifications with queue |
| `input-otp` | OTP input with auto-advance slots |
| `calendar` | Month calendar with date selection |
| `data-table` | Table with sorting, filtering, pagination |

## Usage Example

```tsx
import Button from "@/shared/ui/button"
import { Card, CardHeader, CardTitle, CardContent } from "@/shared/ui/card"
import { Dialog, DialogContent, DialogHeader, DialogTitle } from "@/shared/ui/dialog"
import Input from "@/shared/ui/input"
import Badge from "@/shared/ui/badge"

export default function Page(input) {
  const state = usePageState(input.state ?? { items: [] });

  return (
    <Card>
      <CardHeader>
        <CardTitle>My Page</CardTitle>
      </CardHeader>
      <CardContent>
        <Badge variant="success">Active</Badge>
        <Input placeholder="Search..." />
        <Button variant="default" size="sm">Save</Button>
      </CardContent>
    </Card>
  );
}
```

## Notes

- Components use Tailwind CSS — the project must have Tailwind configured.
- All components are pure TSX with no external npm dependencies — they work in Zebflow's Deno SSR runtime.
- For the platform studio's own UI, the same components are available as `@/components/ui/{name}` (pre-installed, no install step needed).
- To override a component, install it (`overwrite=true`) and edit the file at `repo/pipelines/shared/ui/{name}.tsx`.
