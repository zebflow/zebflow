import { useState } from "zeb/react";
import Button from "@/components/ui/button";
import Toggle from "@/components/ui/toggle";
import Kbd from "@/components/ui/kbd";
import DropdownMenu from "@/components/ui/dropdown-menu";
import DropdownMenuContent from "@/components/ui/dropdown-menu-content";
import DropdownMenuItem from "@/components/ui/dropdown-menu-item";
import DropdownMenuSeparator from "@/components/ui/dropdown-menu-separator";
import ContextMenu from "@/components/ui/context-menu";
import { SectionHeading, Entry, Matrix } from "@/pages/dev/design-system/components/gallery";

const BUTTON_VARIANTS = ["primary", "secondary", "outline", "ghost", "destructive", "link", "live"];
const BUTTON_SIZES = ["lg", "md", "sm", "xs", "icon"];

export default function ActionsSection() {
  const [on, setOn] = useState(true);
  const [picked, setPicked] = useState("nothing yet");

  return (
    <div>
      <SectionHeading title="Actions" description="Things a person presses." />

      <Entry
        name="Button"
        file="button.tsx"
        description="variant × size. `live` is the running-state button; `link` renders as text."
        code={`<Button>Save</Button>
<Button variant="outline" size="sm">Cancel</Button>
<Button variant="destructive">Delete</Button>
<Button variant="ghost" size="icon" aria-label="Close">✕</Button>
<Button as="a" href="/home" variant="link">Home</Button>
<Button disabled>Disabled</Button>`}
      >
        <Matrix
          rows={BUTTON_VARIANTS}
          cols={BUTTON_SIZES}
          render={(variant, size) => (
            <Button variant={variant} size={size} aria-label={size === "icon" ? variant : undefined}>
              {size === "icon" ? "✓" : variant === "live" ? "● Live" : "Button"}
            </Button>
          )}
        />
        <div className="mt-6 flex flex-wrap items-center gap-3 border-t border-border pt-5">
          <Button disabled>Disabled</Button>
          <Button variant="outline" disabled>Disabled</Button>
          <Button as="a" href="#" variant="link">as an anchor</Button>
          <Button variant="secondary" className="min-w-40">custom width</Button>
        </div>
      </Entry>

      <Entry
        name="Toggle"
        file="toggle.tsx"
        description="A switch. Controlled with `checked` + `onChange`, or left to hold its own state."
        code={`<Toggle label="Auto-navigate" checked={on} onChange={(e) => setOn(e.target.checked)} />
<Toggle label="Uncontrolled" defaultChecked />
<Toggle label="Disabled" disabled />`}
      >
        <div className="flex flex-wrap items-center gap-6">
          <Toggle label={on ? "On" : "Off"} checked={on} onChange={(e) => setOn(e.target.checked)} />
          <Toggle label="Uncontrolled" defaultChecked />
          <Toggle label="Disabled off" disabled />
          <Toggle label="Disabled on" disabled checked />
        </div>
      </Entry>

      <Entry
        name="Kbd"
        file="kbd.tsx"
        description="A key. Inline, mono, sized to the surrounding text."
        code={`Press <Kbd>⌘</Kbd> <Kbd>K</Kbd> to open the palette`}
      >
        <p className="text-sm text-foreground">
          Press <Kbd>⌘</Kbd> <Kbd>K</Kbd> to open the palette, <Kbd>Esc</Kbd> to close it, <Kbd>`</Kbd> for the console.
        </p>
      </Entry>

      <Entry
        name="DropdownMenu"
        file="dropdown-menu.tsx"
        description="A trigger and a floating list, closed by an outside click. `align` left | right. Items go straight in as children."
        code={`<DropdownMenu trigger={<Button variant="outline" size="sm">Actions ▾</Button>}>
  <DropdownMenuItem label="Rename" onClick={…} />
  <DropdownMenuItem label="Duplicate" onClick={…} />
  <DropdownMenuSeparator />
  <DropdownMenuItem label="Delete" variant="destructive" onClick={…} />
</DropdownMenu>`}
      >
        <div className="flex flex-wrap items-center gap-6">
          {["left", "right"].map((align) => (
            <DropdownMenu key={align} align={align} trigger={<Button variant="outline" size="sm">align {align} ▾</Button>}>
              <DropdownMenuItem label="Rename" onClick={() => setPicked("rename")} />
              <DropdownMenuItem label="Duplicate" icon="⧉" onClick={() => setPicked("duplicate")} />
              <DropdownMenuSeparator />
              <DropdownMenuItem label="Delete" variant="destructive" onClick={() => setPicked("delete")} />
            </DropdownMenu>
          ))}
          <span className="font-mono text-xs text-muted-foreground">picked: {picked}</span>
        </div>
      </Entry>

      <Entry
        name="DropdownMenuContent"
        file="dropdown-menu-content.tsx"
        description="The panel on its own, for a caller that anchors it under something that is not a DropdownMenu — a <details> summary, say. `align` left | right | center."
        code={`<details className="relative">
  <summary>Session ▾</summary>
  <DropdownMenuContent align="right" className="w-64">
    <DropdownMenuItem label="Sign out" />
  </DropdownMenuContent>
</details>`}
      >
        <div className="flex flex-wrap items-start gap-6">
          <details className="relative">
            <summary className="cursor-pointer select-none rounded-md border border-input px-3 py-1.5 text-sm">Session ▾</summary>
            <DropdownMenuContent align="left" className="w-56">
              <DropdownMenuItem label="Preferences" onClick={() => setPicked("preferences")} />
              <DropdownMenuItem label="Sign out" onClick={() => setPicked("sign out")} />
            </DropdownMenuContent>
          </details>
          <div className="relative h-32 w-64 rounded-md border border-dashed border-border">
            <span className="absolute left-2 top-2 font-mono text-[0.65rem] text-muted-foreground">static, align=center</span>
            <DropdownMenuContent align="center" className="top-8 w-40">
              <DropdownMenuItem label="Item" />
              <DropdownMenuSeparator />
              <DropdownMenuItem label="Another" />
            </DropdownMenuContent>
          </div>
        </div>
      </Entry>

      <Entry
        name="ContextMenu"
        file="context-menu.tsx"
        description="Right-click the row, or press its ⋯. Items: { label, icon?, variant?, disabled?, onSelect } or { separator: true }."
        code={`<ContextMenu
  className="flex items-center justify-between rounded-md border border-border px-3 py-2"
  trigger="⋯"
  items={[
    { label: "Open", onSelect: … },
    { label: "Rename", onSelect: … },
    { separator: true },
    { label: "Delete", variant: "destructive", onSelect: … },
  ]}
>
  <span>pipelines/auth/login.zf.json</span>
</ContextMenu>`}
      >
        <ContextMenu
          className="flex items-center justify-between rounded-md border border-border px-3 py-2 text-sm"
          trigger="⋯"
          items={[
            { label: "Open", onSelect: () => setPicked("open") },
            { label: "Rename", onSelect: () => setPicked("rename") },
            { label: "Unavailable", disabled: true },
            { separator: true },
            { label: "Delete", variant: "destructive", onSelect: () => setPicked("delete") },
          ]}
        >
          <span className="font-mono">pipelines/auth/login.zf.json</span>
        </ContextMenu>
      </Entry>
    </div>
  );
}
