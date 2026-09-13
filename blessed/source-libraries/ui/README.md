# zeb/ui

shadcn/ui's components, on Zebflow's engine. A page imports one file per
component — `import { Button } from "zeb/ui/button"` — and the RWE compiler
inlines that file into the page like an `@/` import. Nothing is installed,
nothing loads at runtime, and the page's Tailwind scan sees the component's
classes. A project that wants to change one clones the file into
`shared/ui/` and switches that one import to `@/shared/ui/<name>`.

Contract for the theme it renders on: `docs/contracts/kinds/ui-theme`.
Guards: `tests/rwe/zeb_ui.rs` — run `cargo test --test rwe zeb_ui`.

## Rules for a component file (`0.1/src/<name>.tsx`)

1. **Imports: `zeb/react` and `zeb/ui/<sibling>` only.** No relative paths,
   no `zeb/use`, no npm. A cloned copy must resolve unchanged. Shared helpers
   live in a sibling file (`zeb/ui/hooks`) and are imported like a component.
   One exception, by name: `editor.tsx` imports `zeb/prosemirror`, its engine
   — a runtime library, loaded once and cached, not inlined into every page.
2. **Colour by role, never by palette.** `bg-primary`, `text-muted-foreground`,
   `border-input`, `ring-ring/50`. Never `bg-gray-900`, `text-white`,
   `bg-red-500`. `dark:` is allowed and means the `.dark` class.
3. **Every class string is a literal.** Variants are a plain object of
   strings joined by `cx` (there is no `cva`); a class assembled at runtime
   from pieces is invisible to the compile-time scan. If you must compute
   one, list every possible value in a hidden `<span tw-variants="…" />`.
4. **Only classes the engine compiles.** `tests/rwe/zeb_ui.rs::every_class_in_the_library_compiles`
   names the ones it does not. Known gaps versus upstream shadcn:
   `[&_svg]:…` and other `[&…]` arbitrary variants, `has-[>svg]:…`,
   `group-data-[…]`, `*:` children selectors. Drop them; do not fake them.
5. **shadcn's API, verbatim where the engine allows it.** Same export names,
   same `variant`/`size` values, same `data-slot` attributes, same sub-part
   names (`DialogContent`, `DialogHeader`…). Differences, and only these:
   - no `asChild` / `Slot`: offer `as="a"` (see button.tsx);
   - no Radix: behaviour is written with `zeb/react` — `useState`,
     `useEffect`, `useRef`, `useId`, and `createContext`/`useContext` for a
     compound family (Dialog ↔ DialogTrigger, Select ↔ SelectItem,
     RadioGroup ↔ RadioGroupItem), exactly as upstream composes them, so a
     shadcn snippet's nesting works unchanged; `createPortal` for overlays;
   - controlled + uncontrolled both work: `open`/`onOpenChange` when given,
     internal state otherwise (`defaultOpen`), like upstream.
6. **Behaviour is real.** A dialog traps focus, restores it on close, closes on
   Escape and on backdrop click, sets `aria-modal`. A menu opens on click,
   closes on outside click and Escape, moves with arrow keys. A select is a
   native `<select>` unless upstream's is a listbox, in which case it is a
   listbox with `role="listbox"`/`role="option"` and keyboard support.
   Client-only APIs (`document`, `window`) are touched inside effects or
   handlers only — the file is rendered on the server first.
7. **Named export + default export.** `export function Card(…)`,
   `export default Card`. Sub-parts are named exports of the same file.
8. **One component family per file**, named as upstream names it. Keep a
   short header comment: what it is, and any deliberate difference from
   upstream.

## What is here

| file | exports |
|---|---|
| button.tsx | Button, buttonVariants |
| input.tsx | Input |
| textarea.tsx | Textarea |
| label.tsx | Label |
| field.tsx | Field, FieldLabel, FieldDescription, FieldError, FieldGroup, FieldLegend, FieldSeparator, FieldSet, FieldContent, FieldTitle |
| checkbox.tsx | Checkbox |
| switch.tsx | Switch |
| radio-group.tsx | RadioGroup, RadioGroupItem |
| toggle.tsx | Toggle, toggleVariants |
| toggle-group.tsx | ToggleGroup, ToggleGroupItem |
| slider.tsx | Slider |
| input-otp.tsx | InputOTP, InputOTPGroup, InputOTPSlot, InputOTPSeparator |
| native-select.tsx | NativeSelect, NativeSelectOptGroup, NativeSelectOption |
| input-group.tsx | InputGroup, InputGroupAddon, InputGroupButton, InputGroupText, InputGroupInput, InputGroupTextarea |
| button-group.tsx | ButtonGroup, ButtonGroupSeparator, ButtonGroupText |
| hooks.tsx | useClickAway, useEscape, useFocusTrap, useControllable, useAnchoredPosition, isVNode, composeEventHandlers |
| dialog.tsx | Dialog, DialogTrigger, DialogContent, DialogHeader, DialogFooter, DialogTitle, DialogDescription, DialogClose |
| alert-dialog.tsx | AlertDialog, AlertDialogTrigger, AlertDialogContent, AlertDialogHeader, AlertDialogFooter, AlertDialogTitle, AlertDialogDescription, AlertDialogAction, AlertDialogCancel |
| sheet.tsx | Sheet, SheetTrigger, SheetContent, SheetHeader, SheetFooter, SheetTitle, SheetDescription, SheetClose |
| drawer.tsx | Drawer, DrawerTrigger, DrawerContent, DrawerHeader, DrawerFooter, DrawerTitle, DrawerDescription, DrawerClose |
| popover.tsx | Popover, PopoverTrigger, PopoverContent |
| tooltip.tsx | Tooltip, TooltipTrigger, TooltipContent |
| hover-card.tsx | HoverCard, HoverCardTrigger, HoverCardContent |
| dropdown-menu.tsx | DropdownMenu, DropdownMenuTrigger, DropdownMenuContent, DropdownMenuGroup, DropdownMenuLabel, DropdownMenuSeparator, DropdownMenuShortcut, DropdownMenuItem, DropdownMenuCheckboxItem, DropdownMenuRadioGroup, DropdownMenuRadioItem |
| context-menu.tsx | ContextMenu, ContextMenuTrigger, ContextMenuContent, ContextMenuGroup, ContextMenuLabel, ContextMenuSeparator, ContextMenuItem, ContextMenuCheckboxItem, ContextMenuRadioGroup, ContextMenuRadioItem |
| select.tsx | Select, SelectTrigger, SelectValue, SelectContent, SelectGroup, SelectLabel, SelectItem |
| sonner.tsx | toast, Toaster |
| tabs.tsx | Tabs, TabsList, TabsTrigger, TabsContent |
| accordion.tsx | Accordion, AccordionItem, AccordionTrigger, AccordionContent |
| collapsible.tsx | Collapsible, CollapsibleTrigger, CollapsibleContent |
| scroll-area.tsx | ScrollArea, ScrollBar |
| resizable.tsx | ResizablePanelGroup, ResizablePanel, ResizableHandle |
| alert.tsx | Alert, AlertTitle, AlertDescription |
| badge.tsx | Badge, badgeVariants |
| card.tsx | Card, CardHeader, CardTitle, CardDescription, CardAction, CardContent, CardFooter |
| separator.tsx | Separator |
| skeleton.tsx | Skeleton |
| kbd.tsx | Kbd, KbdGroup |
| avatar.tsx | Avatar, AvatarImage, AvatarFallback |
| progress.tsx | Progress |
| empty.tsx | Empty, EmptyHeader, EmptyTitle, EmptyDescription, EmptyContent, EmptyMedia |
| spinner.tsx | Spinner |
| aspect-ratio.tsx | AspectRatio |
| table.tsx | Table, TableHeader, TableBody, TableFooter, TableRow, TableHead, TableCell, TableCaption |
| breadcrumb.tsx | Breadcrumb, BreadcrumbList, BreadcrumbItem, BreadcrumbLink, BreadcrumbPage, BreadcrumbSeparator, BreadcrumbEllipsis |
| pagination.tsx | Pagination, PaginationContent, PaginationItem, PaginationLink, PaginationPrevious, PaginationNext, PaginationEllipsis |
| item.tsx | Item, ItemMedia, ItemContent, ItemActions, ItemGroup, ItemSeparator, ItemTitle, ItemDescription, ItemHeader, ItemFooter |
| code-block.tsx | CodeBlock, tokenize |
| editor.tsx | Editor |
| editor-render.tsx | DocumentView, renderDocumentHtml, documentText, safeHref, EDITOR_CLASSES |
