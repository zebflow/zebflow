import { useState } from "zeb";
import Button from "@/components/ui/button";
import Input from "@/components/ui/input";
import Badge from "@/components/ui/badge";
import Separator from "@/components/ui/separator";
import ColorSwatch from "@/components/ui/color-swatch";
import Label from "@/components/ui/label";
import Field from "@/components/ui/field";
import { Select, SelectOption } from "@/components/ui/select";
import Checkbox from "@/components/ui/checkbox";
import Kbd from "@/components/ui/kbd";
import Card from "@/components/ui/card";
import CardHeader from "@/components/ui/card-header";
import CardTitle from "@/components/ui/card-title";
import CardDescription from "@/components/ui/card-description";
import CardContent from "@/components/ui/card-content";
import CardFooter from "@/components/ui/card-footer";
import Dialog from "@/components/ui/dialog";
import DialogContent from "@/components/ui/dialog-content";
import DialogHeader from "@/components/ui/dialog-header";
import DialogTitle from "@/components/ui/dialog-title";
import DialogDescription from "@/components/ui/dialog-description";
import DialogFooter from "@/components/ui/dialog-footer";
import Tabs from "@/components/ui/tabs";
import TabsList from "@/components/ui/tabs-list";
import TabsTrigger from "@/components/ui/tabs-trigger";
import TabsContent from "@/components/ui/tabs-content";
import DropdownMenu from "@/components/ui/dropdown-menu";
import DropdownMenuTrigger from "@/components/ui/dropdown-menu-trigger";
import DropdownMenuContent from "@/components/ui/dropdown-menu-content";
import DropdownMenuItem from "@/components/ui/dropdown-menu-item";
import DropdownMenuSeparator from "@/components/ui/dropdown-menu-separator";
import { initDesignSystemBehavior } from "@/components/behavior/design-system";

export const page = {
  head: {
    title: "Design System · Zebflow",
    description: "Platform UI reference for platform developers and agents",
  },
  html: { lang: "en" },
  body: {
    className: "font-sans",
  },
  navigation: "history",
};


// ── Page-local helpers ────────────────────────────────────────────────────────

function SectionHeading({ title, description }) {
  return (
    <div className="mb-8">
      <h2 className="text-2xl font-bold tracking-tight text-[var(--studio-text)]">{title}</h2>
      {description ? (
        <p className="mt-1.5 text-sm text-[var(--studio-text-soft)]">{description}</p>
      ) : null}
      <div className="mt-5 h-px bg-[var(--studio-border)]" />
    </div>
  );
}

function SubHeading({ title }) {
  return (
    <h3 className="text-[0.68rem] font-bold uppercase tracking-widest text-[var(--studio-text-soft)] mb-4 mt-10 first:mt-0">
      {title}
    </h3>
  );
}

function DemoBox({ children, className }) {
  return (
    <div
      className={cx(
        "rounded-xl border border-[var(--studio-border)] bg-[var(--studio-panel)] p-6",
        className,
      )}
    >
      {children}
    </div>
  );
}

function CodeBlock({ code }) {
  return (
    <div className="relative mt-3" data-code-wrapper="true">
      <pre className="rounded-xl bg-[var(--studio-panel-2)] border border-[var(--studio-border)] px-5 py-4 overflow-x-auto text-[0.78rem] font-mono text-[var(--studio-text)] leading-6 whitespace-pre">
        <code data-code-block="true">{code}</code>
      </pre>
      <button
        data-copy-btn="true"
        className="absolute top-2.5 right-3 text-[0.65rem] font-mono text-[var(--studio-text-soft)] hover:text-[var(--studio-text)] bg-[var(--studio-panel-3)] border border-[var(--studio-border)] rounded px-2 py-1 transition-colors"
      >
        copy
      </button>
    </div>
  );
}

function RuleAlert({ children }) {
  return (
    <div className="rounded-lg border border-[var(--studio-border)] bg-[var(--studio-panel)] px-4 py-3 mb-5 flex gap-3 items-start">
      <span className="text-[var(--zf-color-brand-orange)] mt-px shrink-0">⚠</span>
      <p className="text-sm text-[var(--studio-text-soft)] leading-relaxed">{children}</p>
    </div>
  );
}

function ComponentRow({ name, importPath, description, children, code }) {
  return (
    <div className="mb-12">
      <div className="flex items-start gap-4 mb-3">
        <div className="flex-1">
          <h3 className="text-base font-semibold text-[var(--studio-text)]">{name}</h3>
          {description ? (
            <p className="text-sm text-[var(--studio-text-soft)] mt-0.5">{description}</p>
          ) : null}
        </div>
        <code className="shrink-0 text-[0.68rem] font-mono text-[var(--studio-text-soft)] bg-[var(--studio-panel-2)] border border-[var(--studio-border)] px-2 py-1 rounded whitespace-nowrap">
          {importPath}
        </code>
      </div>
      <DemoBox className={null}>{children}</DemoBox>
      {code ? <CodeBlock code={code} /> : null}
    </div>
  );
}

// ── Section: Tokens ───────────────────────────────────────────────────────────

function TokensSection() {
  return (
    <div>
      <SectionHeading
        title="Design Tokens"
        description="CSS custom properties defined in main.css and the studio theme. Use these vars everywhere — never hardcode hex values."
      />

      <SubHeading title="Brand Colors" />
      <div className="grid grid-cols-2 gap-4 sm:grid-cols-4 mb-10">
        <ColorSwatch name="--zf-color-brand-blue" value="#005b9a" />
        <ColorSwatch name="--zf-color-brand-blue-ink" value="#004a7a" />
        <ColorSwatch name="--zf-color-brand-orange" value="#ff5c00" />
        <ColorSwatch name="--zf-color-brand-orange-ink" value="#db4f00" />
      </div>

      <SubHeading title="Studio Theme (Dark)" />
      <RuleAlert>
        The studio vars are only available inside a <code>.project-studio-frame</code> wrapper. Platform
        developer pages wrap content in this class. Never hardcode their hex values — always use the var().
      </RuleAlert>
      <div className="grid grid-cols-2 gap-4 sm:grid-cols-3 lg:grid-cols-4 mb-10">
        <ColorSwatch name="--studio-bg" value="#0b1120" />
        <ColorSwatch name="--studio-panel" value="#111827" />
        <ColorSwatch name="--studio-panel-2" value="#172033" />
        <ColorSwatch name="--studio-panel-3" value="#1e293b" />
        <ColorSwatch name="--studio-border" value="#2b3648" />
        <ColorSwatch name="--studio-text" value="#e5edf7" />
        <ColorSwatch name="--studio-text-soft" value="#93a4ba" />
        <ColorSwatch name="--studio-accent" value="#ff5c00" />
      </div>

      <SubHeading title="Typography" />
      <div className="space-y-6 mb-10">
        <div>
          <p className="text-[0.68rem] font-mono text-[var(--studio-text-soft)] uppercase tracking-widest mb-2">
            --zf-font-display · Pathway Extreme + Roboto
          </p>
          <p
            className="text-3xl font-black tracking-tight text-[var(--studio-text)]"
            style={{ fontFamily: "var(--zf-font-display)" }}
          >
            ZEBFLOW Platform Studio
          </p>
        </div>
        <div>
          <p className="text-[0.68rem] font-mono text-[var(--studio-text-soft)] uppercase tracking-widest mb-2">
            --zf-font-sans · Roboto (body default)
          </p>
          <p className="text-base text-[var(--studio-text)]" style={{ fontFamily: "var(--zf-font-sans)" }}>
            The quick brown fox jumps over the lazy dog. 0123456789 AaBbCc
          </p>
        </div>
        <div>
          <p className="text-[0.68rem] font-mono text-[var(--studio-text-soft)] uppercase tracking-widest mb-2">
            --zf-font-mono · Roboto Mono (code + CLI)
          </p>
          <p className="text-sm text-[var(--studio-text)]" style={{ fontFamily: "var(--zf-font-mono)" }}>
            register blog-api | trigger.webhook --path /api --method GET | pg.query --credential main-db
          </p>
        </div>
      </div>

      <SubHeading title="Spacing & Radius" />
      <DemoBox className={null}>
        <div className="flex items-center gap-4 flex-wrap">
          <div className="flex items-center gap-2 text-sm text-[var(--studio-text-soft)]">
            <div className="rounded bg-[var(--studio-accent)] w-4 h-4" />
            <span className="font-mono text-xs">rounded (4px)</span>
          </div>
          <div className="flex items-center gap-2 text-sm text-[var(--studio-text-soft)]">
            <div className="rounded-lg bg-[var(--studio-accent)] w-4 h-4" />
            <span className="font-mono text-xs">rounded-lg (8px) — buttons</span>
          </div>
          <div className="flex items-center gap-2 text-sm text-[var(--studio-text-soft)]">
            <div className="rounded-xl bg-[var(--studio-accent)] w-4 h-4" />
            <span className="font-mono text-xs">rounded-xl (12px) — panels</span>
          </div>
          <div className="flex items-center gap-2 text-sm text-[var(--studio-text-soft)]">
            <div className="rounded-full bg-[var(--studio-accent)] w-4 h-4" />
            <span className="font-mono text-xs">rounded-full — pills, badges</span>
          </div>
          <div className="flex items-center gap-2 text-sm text-[var(--studio-text-soft)]">
            <div style={{ borderRadius: "var(--zf-radius-panel)" }} className="bg-[var(--studio-accent)] w-4 h-4" />
            <span className="font-mono text-xs">--zf-radius-panel (1.25rem) — modals</span>
          </div>
        </div>
      </DemoBox>
    </div>
  );
}

// ── Section: Components ───────────────────────────────────────────────────────

function ComponentsSection() {
  const [dialogOpen, setDialogOpen] = useState(false);
  const [activeTab, setActiveTab] = useState("tab1");

  return (
    <div>
      <SectionHeading
        title="Components"
        description="Import from @/components/ui/ — always check here before writing custom HTML."
      />

      <ComponentRow
        name="Button"
        importPath="import Button from '@/components/ui/button';"
        description="Primary interactive element. 6 variants × 4 sizes. Use label prop or children."
        code={`<Button variant="primary" label="Save" />
<Button variant="outline" label="Cancel" />
<Button variant="destructive" label="Delete" />
<Button size="sm" variant="ghost" label="Dismiss" />`}
      >
        <div className="flex flex-col gap-4">
          <div className="flex flex-wrap gap-2">
            <Button variant="primary" label="Primary" />
            <Button variant="outline" label="Outline" />
            <Button variant="secondary" label="Secondary" />
            <Button variant="ghost" label="Ghost" />
            <Button variant="destructive" label="Destructive" />
            <Button variant="link" label="Link" />
          </div>
          <div className="flex flex-wrap items-center gap-2">
            <Button size="xs" variant="outline" label="xs" />
            <Button size="sm" variant="outline" label="sm" />
            <Button size="md" variant="outline" label="md (default)" />
            <Button size="lg" variant="outline" label="lg" />
          </div>
          <div className="flex flex-wrap gap-2">
            <Button variant="primary" disabled label="Disabled" />
          </div>
        </div>
      </ComponentRow>

      <ComponentRow
        name="Input"
        importPath="import Input from '@/components/ui/input';"
        description="Text input. Supports placeholder, disabled, readOnly, type, value/defaultValue."
        code={`<Input placeholder="Your name" />
<Input type="password" placeholder="Password" />
<Input placeholder="Read-only" readOnly value="fixed value" />`}
      >
        <div className="flex flex-col gap-3 max-w-sm">
          <Input placeholder="Default input" />
          <Input type="password" placeholder="Password input" />
          <Input placeholder="Disabled" disabled />
          <Input placeholder="Read-only" readOnly defaultValue="fixed value" />
        </div>
      </ComponentRow>

      <ComponentRow
        name="Badge"
        importPath="import Badge from '@/components/ui/badge';"
        description="Inline status label or tag. 4 variants."
        code={`<Badge label="Default" />
<Badge label="Active" variant="secondary" />
<Badge label="Error" variant="destructive" />
<Badge label="Tag" variant="outline" />`}
      >
        <div className="flex flex-wrap gap-3">
          <Badge label="Default" />
          <Badge label="Secondary" variant="secondary" />
          <Badge label="Destructive" variant="destructive" />
          <Badge label="Outline" variant="outline" />
        </div>
      </ComponentRow>

      <ComponentRow
        name="Separator"
        importPath="import Separator from '@/components/ui/separator';"
        description="Visual divider. horizontal (default) or vertical."
        code={`<Separator />
<Separator orientation="vertical" />`}
      >
        <div className="space-y-5">
          <div>
            <p className="text-xs text-[var(--studio-text-soft)] mb-2">Horizontal</p>
            <Separator />
          </div>
          <div>
            <p className="text-xs text-[var(--studio-text-soft)] mb-2">Vertical</p>
            <div className="flex items-center gap-3 h-8">
              <span className="text-sm text-[var(--studio-text-soft)]">Left</span>
              <Separator orientation="vertical" />
              <span className="text-sm text-[var(--studio-text-soft)]">Right</span>
            </div>
          </div>
        </div>
      </ComponentRow>

      <SubHeading title="Form Controls" />

      <ComponentRow
        name="Label"
        importPath="import Label from '@/components/ui/label';"
        description="Field label. Mono uppercase, muted. Use with htmlFor to associate with an input."
        code={`<Label label="Email address" htmlFor="email" />`}
      >
        <div className="flex flex-col gap-3">
          <Label label="Email address" htmlFor="demo-email" />
          <Label label="Disabled field" className="opacity-50" />
        </div>
      </ComponentRow>

      <ComponentRow
        name="Field"
        importPath="import Field from '@/components/ui/field';"
        description="Label + input + optional hint slot. Composes Label with any input child."
        code={`<Field label="Username" id="username" description="Letters and numbers only.">
  <Input id="username" placeholder="jane_doe" />
</Field>`}
      >
        <div className="max-w-sm">
          <Field label="Username" id="demo-username" description="Letters and numbers only.">
            <Input id="demo-username" placeholder="jane_doe" />
          </Field>
        </div>
      </ComponentRow>

      <ComponentRow
        name="Select"
        importPath="import { Select, SelectOption } from '@/components/ui/select';"
        description="Native select with custom chevron. Works with controlled value + onChange."
        code={`<Select>
  <SelectOption value="us" label="United States" />
  <SelectOption value="id" label="Indonesia" />
  <SelectOption value="sg" label="Singapore" />
</Select>`}
      >
        <div className="max-w-xs">
          <Select>
            <SelectOption value="us" label="United States" />
            <SelectOption value="id" label="Indonesia" />
            <SelectOption value="sg" label="Singapore" />
            <SelectOption value="jp" label="Japan" />
          </Select>
        </div>
      </ComponentRow>

      <ComponentRow
        name="Checkbox"
        importPath="import Checkbox from '@/components/ui/checkbox';"
        description="Compact toggle for dark console/toolbar contexts. Forwards data-* attrs."
        code={`<Checkbox label="Enable feature" />
<Checkbox label="Auto-navigate" defaultChecked />`}
      >
        <div className="flex flex-col gap-3">
          <Checkbox label="Enable feature" />
          <Checkbox label="Auto-navigate" defaultChecked />
          <Checkbox label="Disabled" disabled />
        </div>
      </ComponentRow>

      <SubHeading title="Keyboard Shortcut" />

      <ComponentRow
        name="Kbd"
        importPath="import Kbd from '@/components/ui/kbd';"
        description="Physical key chip. Use children for key labels — compose multiple for chords."
        code={`<Kbd>⌘</Kbd><Kbd>K</Kbd>
<Kbd>Ctrl</Kbd><Kbd>S</Kbd>
<Kbd>\`</Kbd>`}
      >
        <div className="flex flex-wrap items-center gap-4">
          <span className="flex items-center gap-1">
            <Kbd>⌘</Kbd>
            <Kbd>K</Kbd>
          </span>
          <span className="flex items-center gap-1">
            <Kbd>Ctrl</Kbd>
            <Kbd>S</Kbd>
          </span>
          <span className="flex items-center gap-1">
            <Kbd>Shift</Kbd>
            <Kbd>⌥</Kbd>
            <Kbd>P</Kbd>
          </span>
          <Kbd>`</Kbd>
        </div>
      </ComponentRow>

      <SubHeading title="Card" />

      <ComponentRow
        name="Card"
        importPath="import Card from '@/components/ui/card'; // + CardHeader, CardTitle, etc."
        description="Content container. Composed from Card + CardHeader + CardTitle + CardDescription + CardContent + CardFooter."
        code={`import Card from "@/components/ui/card";
import CardHeader from "@/components/ui/card-header";
import CardTitle from "@/components/ui/card-title";
import CardDescription from "@/components/ui/card-description";
import CardContent from "@/components/ui/card-content";
import CardFooter from "@/components/ui/card-footer";

<Card>
  <CardHeader>
    <CardTitle>Project Settings</CardTitle>
    <CardDescription>Manage your project configuration.</CardDescription>
  </CardHeader>
  <CardContent>
    <p>Card body content goes here.</p>
  </CardContent>
  <CardFooter>
    <Button variant="primary" label="Save changes" />
  </CardFooter>
</Card>`}
      >
        <div className="max-w-sm">
          <Card>
            <CardHeader>
              <CardTitle>Project Settings</CardTitle>
              <CardDescription>Manage your project configuration.</CardDescription>
            </CardHeader>
            <CardContent>
              <p className="text-sm text-[var(--studio-text-soft)]">Card body content goes here.</p>
            </CardContent>
            <CardFooter>
              <Button variant="primary" label="Save changes" />
            </CardFooter>
          </Card>
        </div>
      </ComponentRow>

      <SubHeading title="Dialog" />

      <ComponentRow
        name="Dialog"
        importPath="import Dialog from '@/components/ui/dialog'; // + Dialog* family"
        description="Modal dialog panel. Toggle open prop via useState. Renders native <dialog> element."
        code={`const [open, setOpen] = useState(false);

<Button variant="outline" label="Open dialog" onClick={() => setOpen(true)} />

{open ? (
  <Dialog open>
    <DialogHeader>
      <DialogTitle>Confirm action</DialogTitle>
      <DialogDescription>This cannot be undone.</DialogDescription>
    </DialogHeader>
    <DialogContent>
      <p>Are you sure you want to continue?</p>
    </DialogContent>
    <DialogFooter>
      <Button variant="outline" label="Cancel" onClick={() => setOpen(false)} />
      <Button variant="destructive" label="Continue" onClick={() => setOpen(false)} />
    </DialogFooter>
  </Dialog>
) : null}`}
      >
        <div className="flex flex-col gap-4">
          <Button variant="outline" label="Open dialog" onClick={() => setDialogOpen(true)} />
          {dialogOpen ? (
            <Dialog open>
              <DialogHeader>
                <DialogTitle>Confirm action</DialogTitle>
                <DialogDescription>This cannot be undone.</DialogDescription>
              </DialogHeader>
              <DialogContent>
                <p className="text-sm">Are you sure you want to continue?</p>
              </DialogContent>
              <DialogFooter>
                <Button variant="outline" label="Cancel" onClick={() => setDialogOpen(false)} />
                <Button variant="destructive" label="Continue" onClick={() => setDialogOpen(false)} />
              </DialogFooter>
            </Dialog>
          ) : null}
        </div>
      </ComponentRow>

      <SubHeading title="Tabs" />

      <ComponentRow
        name="Tabs"
        importPath="import Tabs from '@/components/ui/tabs'; // + TabsList, TabsTrigger, TabsContent"
        description="Tabbed content switcher. Controlled via active prop on TabsTrigger and TabsContent."
        code={`const [activeTab, setActiveTab] = useState("tab1");

<Tabs>
  <TabsList>
    <TabsTrigger label="Overview" active={activeTab === "tab1"} onClick={() => setActiveTab("tab1")} />
    <TabsTrigger label="Settings" active={activeTab === "tab2"} onClick={() => setActiveTab("tab2")} />
    <TabsTrigger label="Members" active={activeTab === "tab3"} onClick={() => setActiveTab("tab3")} />
  </TabsList>
  <TabsContent active={activeTab === "tab1"}>
    <p>Overview content</p>
  </TabsContent>
  <TabsContent active={activeTab === "tab2"}>
    <p>Settings content</p>
  </TabsContent>
  <TabsContent active={activeTab === "tab3"}>
    <p>Members content</p>
  </TabsContent>
</Tabs>`}
      >
        <Tabs>
          <TabsList>
            <TabsTrigger label="Overview" active={activeTab === "tab1"} onClick={() => setActiveTab("tab1")} />
            <TabsTrigger label="Settings" active={activeTab === "tab2"} onClick={() => setActiveTab("tab2")} />
            <TabsTrigger label="Members" active={activeTab === "tab3"} onClick={() => setActiveTab("tab3")} />
          </TabsList>
          <TabsContent active={activeTab === "tab1"}>
            <p className="text-sm text-[var(--studio-text-soft)] p-2">Overview content — configure your project from here.</p>
          </TabsContent>
          <TabsContent active={activeTab === "tab2"}>
            <p className="text-sm text-[var(--studio-text-soft)] p-2">Settings content — adjust plan, billing, and preferences.</p>
          </TabsContent>
          <TabsContent active={activeTab === "tab3"}>
            <p className="text-sm text-[var(--studio-text-soft)] p-2">Members content — invite collaborators and manage roles.</p>
          </TabsContent>
        </Tabs>
      </ComponentRow>

      <SubHeading title="Dropdown Menu" />

      <ComponentRow
        name="DropdownMenu"
        importPath="import DropdownMenu from '@/components/ui/dropdown-menu'; // + DropdownMenu* family"
        description="Native <details>-based dropdown — no JavaScript needed. DropdownMenuTrigger is the <summary>."
        code={`<DropdownMenu>
  <DropdownMenuTrigger>
    <Button variant="outline" label="Options ▾" />
  </DropdownMenuTrigger>
  <DropdownMenuContent>
    <DropdownMenuItem label="Edit" />
    <DropdownMenuItem label="Duplicate" />
    <DropdownMenuSeparator />
    <DropdownMenuItem label="Delete" />
  </DropdownMenuContent>
</DropdownMenu>`}
      >
        <DropdownMenu>
          <DropdownMenuTrigger>
            <Button variant="outline" label="Options ▾" />
          </DropdownMenuTrigger>
          <DropdownMenuContent>
            <DropdownMenuItem label="Edit" />
            <DropdownMenuItem label="Duplicate" />
            <DropdownMenuSeparator />
            <DropdownMenuItem label="Delete" />
          </DropdownMenuContent>
        </DropdownMenu>
      </ComponentRow>
    </div>
  );
}

// ── Section: Patterns ─────────────────────────────────────────────────────────

function PatternsSection() {
  const twVariantsCode = `// In your TSX template, add tw-variants to safelist dynamic classes:
<div
  data-status-panel="true"
  tw-variants="bg-green-500/10 bg-red-500/10 text-green-400 text-red-400 border-green-500/20 border-red-500/20"
/>

// In your .ts behavior file, freely use those classes:
panel.className = isOk
  ? "bg-green-500/10 text-green-400 border border-green-500/20"
  : "bg-red-500/10 text-red-400 border border-red-500/20";

// For CSS-var-based dynamic values, use inline style instead:
btn.style.background = isActive ? "var(--studio-panel-2)" : "";`;

  const behaviorFileCode = `// src/platform/web/templates/components/behavior/my-feature.ts
// Behavior files wire DOM events. No render(), no npm:preact imports.
// Components live in the page or layout — behavior files are DOM-only.

export function initMyFeatureBehavior() {
  if (typeof document === "undefined") return;
  const run = () => scanRoots();
  if (typeof requestAnimationFrame === "function") {
    requestAnimationFrame(run);
  } else {
    setTimeout(run, 0);
  }
}

const initializedRoots = new WeakSet();

function scanRoots() {
  document.querySelectorAll("[data-my-feature]").forEach((root) => {
    if (initializedRoots.has(root)) return;
    initializedRoots.add(root);
    initRoot(root as HTMLElement);
  });
}

function initRoot(root: HTMLElement) {
  const btn = root.querySelector("[data-my-btn]") as HTMLElement | null;
  if (!btn) return;
  btn.addEventListener("click", () => {
    const active = root.getAttribute("data-active") === "true";
    root.setAttribute("data-active", String(!active));
  });
}`;

  const preactRefsCode = `// ✗ WRONG — never querySelector inside a .tsx component:
const el = document.querySelector("[data-thing]");
el.style.color = "red";

// ✓ CORRECT — use Preact useRef:
// (useRef is a global — no import needed, or import from "zeb" for clarity)
import { useRef, useEffect } from "zeb";

function MyComponent() {
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (ref.current) {
      ref.current.style.color = "red"; // only if you truly need DOM access
    }
  }, []);

  return <div ref={ref}>...</div>;
}`;

  const newComponentCode = `// src/platform/web/templates/components/ui/my-widget.tsx
// cx() is a runtime global — no import needed, no import from "zeb" required.
function MyWidget({ label, variant = "default", className }) {
  const cls = variant === "accent"
    ? "bg-[var(--studio-accent)] text-white"
    : "bg-[var(--studio-panel)] text-[var(--studio-text)]";

  return (
    <div className={cx("rounded-lg px-3 py-2 text-sm", cls, className)}>
      {label}
    </div>
  );
}`;

  const atAliasCode = `// @/ always resolves to the template root:
//   src/platform/web/templates/
//
// Examples:
import Button from "@/components/ui/button";
import Input from "@/components/ui/input";
import Badge from "@/components/ui/badge";
import ColorSwatch from "@/components/ui/color-swatch";
import ProjectStudioShell from "@/components/layout/project-studio-shell";
import { initMyFeatureBehavior } from "@/components/behavior/my-feature";

// Works in: page files, ui components, layout components, behavior files.
// The @/ alias is resolved at compile time by the RWE engine — not by TypeScript.
// (TypeScript editor will show "cannot find module" warnings — ignore them, @/ works at runtime.)`;

  return (
    <div>
      <SectionHeading
        title="Patterns"
        description="Conventions every platform developer and AI agent must follow. Violations will break the UI."
      />

      {/* Imports */}
      <div className="mb-12">
        <h3 className="text-base font-semibold text-[var(--studio-text)] mb-2">
          Import paths: @/ alias and rwe globals
        </h3>
        <RuleAlert>
          Use @/ for all local template imports — it resolves to the template root at compile time.
          Hooks (useState, useRef, useEffect, usePageState, useMemo) and cx() are runtime globals —
          import them from "zeb" as a declaration hint (the import is stripped; they are already available).
          Never use npm:preact, npm:preact/hooks, or call render(). Components belong in the page or layout.
        </RuleAlert>
        <CodeBlock code={atAliasCode} />
      </div>

      {/* Rule 1 */}
      <div className="mb-12">
        <h3 className="text-base font-semibold text-[var(--studio-text)] mb-2">
          Always use components/ui/
        </h3>
        <RuleAlert>
          Never write raw {"<button>"}, {"<input>"}, or {"<div className=\"...\">"}
          {" "}when a ui/ component exists. Check components/ui/ first — if it is there, use it.
          If it is not there, create a new reusable component in components/ui/ and consult first.
        </RuleAlert>
        <CodeBlock code={newComponentCode} />
      </div>

      {/* Rule 2 */}
      <div className="mb-12">
        <h3 className="text-base font-semibold text-[var(--studio-text)] mb-2">
          tw-variants for behavior-file Tailwind classes
        </h3>
        <RuleAlert>
          The Tailwind compiler only scans class= attributes in rendered TSX. Classes toggled by
          .ts behavior files are invisible to the compiler and will NOT be generated. Add a
          tw-variants attribute to the relevant template element to safelist them.
        </RuleAlert>
        <CodeBlock code={twVariantsCode} />
      </div>

      {/* Rule 3 */}
      <div className="mb-12">
        <h3 className="text-base font-semibold text-[var(--studio-text)] mb-2">
          No document.querySelector in .tsx components
        </h3>
        <RuleAlert>
          Use Preact refs and state in .tsx files. document.querySelector belongs only in .ts
          behavior files — never inside a component function.
        </RuleAlert>
        <CodeBlock code={preactRefsCode} />
      </div>

      {/* Rule 4 */}
      <div className="mb-12">
        <h3 className="text-base font-semibold text-[var(--studio-text)] mb-2">
          Behavior file anatomy
        </h3>
        <RuleAlert>
          Behavior files (.ts) wire DOM events only — no Preact mounting, no render(), no npm:preact.
          The exported init function is called from Page() so it runs after every SPA navigation.
          Use data-* attributes to scope initialization to specific DOM roots.
        </RuleAlert>
        <CodeBlock code={behaviorFileCode} />
      </div>

      {/* Rule 5 */}
      <div className="mb-12">
        <h3 className="text-base font-semibold text-[var(--studio-text)] mb-2">
          main.css is global only
        </h3>
        <RuleAlert>
          main.css is for global resets, design token declarations (:root vars), and the
          .project-studio-frame theme block. Do NOT add component-specific styles here. Use
          Tailwind classes for everything else. Consult before adding to main.css.
        </RuleAlert>
      </div>
    </div>
  );
}

// ── Main page ─────────────────────────────────────────────────────────────────

const NAV_ITEMS = [
  { id: "tokens", label: "Tokens", icon: "◈" },
  { id: "components", label: "Components", icon: "⬡" },
  { id: "patterns", label: "Patterns", icon: "⚑" },
];

export default function Page(input) {
  const [activeSection, setActiveSection] = useState("tokens");

  initDesignSystemBehavior();

  return (
    <div className="project-studio-frame h-screen overflow-hidden flex flex-col">
      {/* Top header */}
      <header className="shrink-0 flex items-center gap-4 px-6 h-12 border-b border-[var(--studio-border)] bg-[var(--studio-panel)]">
        <a
          href="/home"
          className="text-xs font-mono text-[var(--studio-text-soft)] hover:text-[var(--studio-text)] transition-colors flex items-center gap-1.5"
        >
          ← home
        </a>
        <div className="w-px h-4 bg-[var(--studio-border)]" />
        <div className="flex items-baseline gap-2">
          <span className="text-sm font-bold tracking-tight text-[var(--studio-text)]">
            Design System
          </span>
          <span className="text-[0.65rem] font-mono text-[var(--studio-text-soft)] uppercase tracking-widest">
            Zebflow Platform
          </span>
        </div>
        <div className="flex-1" />
        <a
          href="https://github.com/mecha-id/zebflow"
          className="text-xs font-mono text-[var(--studio-text-soft)] hover:text-[var(--studio-text)] transition-colors"
        >
          agent-readable
        </a>
      </header>

      {/* Body */}
      <div className="flex flex-1 min-h-0">
        {/* Left sidebar */}
        <nav className="shrink-0 w-48 border-r border-[var(--studio-border)] bg-[var(--studio-panel)] flex flex-col pt-4 pb-6 gap-0.5 px-2">
          {NAV_ITEMS.map((item) => (
            <button
              key={item.id}
              onClick={() => setActiveSection(item.id)}
              className={cx(
                "flex items-center gap-2.5 px-3 py-2 rounded-lg text-sm transition-colors w-full text-left",
                activeSection === item.id
                  ? "bg-[var(--studio-panel-2)] text-[var(--studio-text)] font-semibold"
                  : "text-[var(--studio-text-soft)] hover:text-[var(--studio-text)] hover:bg-[var(--studio-panel-3)]",
              )}
            >
              <span className="text-base leading-none opacity-70">{item.icon}</span>
              <span>{item.label}</span>
            </button>
          ))}

          <div className="mt-auto pt-6 px-1">
            <Separator />
            <p className="text-[0.65rem] font-mono text-[var(--studio-text-soft)] opacity-50 mt-4 leading-relaxed">
              Platform-level reference.{"\n"}Not project-scoped.
            </p>
          </div>
        </nav>

        {/* Content */}
        <main
          data-ds-content="true"
          className="flex-1 overflow-y-auto px-10 py-8"
          style={{ scrollbarWidth: "thin", scrollbarColor: "var(--studio-border) transparent" }}
        >
          <section hidden={activeSection !== "tokens"}>
            <TokensSection />
          </section>

          <section hidden={activeSection !== "components"}>
            <ComponentsSection />
          </section>

          <section hidden={activeSection !== "patterns"}>
            <PatternsSection />
          </section>
        </main>
      </div>
    </div>
  );
}
