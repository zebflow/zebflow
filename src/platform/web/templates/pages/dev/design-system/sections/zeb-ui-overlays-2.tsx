import { useState } from "zeb/react";
import { Button, buttonVariants } from "zeb/ui/button";
import { Popover, PopoverTrigger, PopoverContent } from "zeb/ui/popover";
import { Tooltip, TooltipTrigger, TooltipContent } from "zeb/ui/tooltip";
import { HoverCard, HoverCardTrigger, HoverCardContent } from "zeb/ui/hover-card";
import {
  DropdownMenu,
  DropdownMenuTrigger,
  DropdownMenuContent,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuItem,
  DropdownMenuShortcut,
  DropdownMenuCheckboxItem,
  DropdownMenuRadioGroup,
  DropdownMenuRadioItem,
} from "zeb/ui/dropdown-menu";
import { ContextMenu, ContextMenuTrigger, ContextMenuContent, ContextMenuItem, ContextMenuSeparator } from "zeb/ui/context-menu";
import { Select, SelectTrigger, SelectValue, SelectContent, SelectGroup, SelectLabel, SelectItem } from "zeb/ui/select";
import { toast, Toaster } from "zeb/ui/sonner";
import { SectionHeading, Entry } from "@/pages/dev/design-system/components/gallery";

/** zeb/ui — overlays, part 2: anchored panels, the two menus, Select, Sonner. */

function PopoverDemo() {
  return (
    <Popover>
      <PopoverTrigger className={buttonVariants({ variant: "outline" })}>Open popover</PopoverTrigger>
      <PopoverContent>
        <p className="text-sm font-medium text-foreground">Dimensions</p>
        <p className="mt-1 text-sm text-muted-foreground">Set the width and height for the layer.</p>
      </PopoverContent>
    </Popover>
  );
}

function TooltipDemo() {
  return (
    <Tooltip delayDuration={200}>
      <TooltipTrigger>
        <button type="button" className={buttonVariants({ variant: "outline" })}>
          Hover or focus me
        </button>
      </TooltipTrigger>
      <TooltipContent>Add to library</TooltipContent>
    </Tooltip>
  );
}

function HoverCardDemo() {
  return (
    <HoverCard>
      <HoverCardTrigger>
        <a href="#" className="text-sm font-medium text-primary underline-offset-4 hover:underline">
          @zebflow
        </a>
      </HoverCardTrigger>
      <HoverCardContent>
        <p className="text-sm font-semibold text-foreground">Zebflow</p>
        <p className="mt-1 text-sm text-muted-foreground">The platform this whole gallery renders on. Joined 2019.</p>
      </HoverCardContent>
    </HoverCard>
  );
}

function DropdownMenuDemo() {
  const [bookmarked, setBookmarked] = useState(true);
  const [account, setAccount] = useState("personal");
  return (
    <DropdownMenu>
      <DropdownMenuTrigger className={buttonVariants({ variant: "outline" })}>Options</DropdownMenuTrigger>
      <DropdownMenuContent>
        <DropdownMenuLabel>My account</DropdownMenuLabel>
        <DropdownMenuSeparator />
        <DropdownMenuItem>
          Profile
          <DropdownMenuShortcut>⇧⌘P</DropdownMenuShortcut>
        </DropdownMenuItem>
        <DropdownMenuItem>Billing</DropdownMenuItem>
        <DropdownMenuCheckboxItem checked={bookmarked} onCheckedChange={setBookmarked}>
          Bookmarked
        </DropdownMenuCheckboxItem>
        <DropdownMenuSeparator />
        <DropdownMenuRadioGroup>
          <DropdownMenuRadioItem checked={account === "personal"} onClick={() => setAccount("personal")}>
            Personal
          </DropdownMenuRadioItem>
          <DropdownMenuRadioItem checked={account === "team"} onClick={() => setAccount("team")}>
            Team
          </DropdownMenuRadioItem>
        </DropdownMenuRadioGroup>
        <DropdownMenuSeparator />
        <DropdownMenuItem variant="destructive">Delete account</DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

function ContextMenuDemo() {
  return (
    <ContextMenu>
      <ContextMenuTrigger className="flex h-28 w-full items-center justify-center rounded-md border border-dashed border-border text-sm text-muted-foreground">
        Right click here
      </ContextMenuTrigger>
      <ContextMenuContent>
        <ContextMenuItem>Back</ContextMenuItem>
        <ContextMenuItem>Forward</ContextMenuItem>
        <ContextMenuItem>Reload</ContextMenuItem>
        <ContextMenuSeparator />
        <ContextMenuItem variant="destructive">Delete</ContextMenuItem>
      </ContextMenuContent>
    </ContextMenu>
  );
}

const FRUITS = { apple: "Apple", banana: "Banana", blueberry: "Blueberry", grapes: "Grapes" };

function SelectDemo() {
  const [value, setValue] = useState("blueberry");
  return (
    <div className="w-56">
      <Select value={value} onValueChange={setValue}>
        <SelectTrigger>
          <SelectValue placeholder="Select a fruit" />
        </SelectTrigger>
        <SelectContent>
          <SelectGroup>
            <SelectLabel>Fruits</SelectLabel>
            {Object.keys(FRUITS).map((key) => (
              <SelectItem key={key} value={key}>
                {FRUITS[key]}
              </SelectItem>
            ))}
          </SelectGroup>
        </SelectContent>
      </Select>
    </div>
  );
}

function SonnerDemo() {
  return (
    <div className="flex flex-wrap gap-3">
      <Button variant="outline" onClick={() => toast("Event has been created")}>
        Default
      </Button>
      <Button variant="outline" onClick={() => toast.success("Saved successfully")}>
        Success
      </Button>
      <Button variant="outline" onClick={() => toast.error("Something went wrong")}>
        Error
      </Button>
      <Button variant="outline" onClick={() => toast.warning("Check your input")}>
        Warning
      </Button>
      <Button variant="outline" onClick={() => toast.info("A new version is available")}>
        Info
      </Button>
      <Toaster />
    </div>
  );
}

export default function ZebUiOverlays2Section() {
  return (
    <div>
      <SectionHeading
        title="zeb/ui · Overlays (2/2)"
        description="Anchored panels, the two menus, Select, and Sonner. Part 1 (zeb-ui-overlays.tsx) has the hooks and the modal-panel family."
      />

      <Entry
        name="Popover"
        file="zeb/ui/popover"
        description="Opens on click; closes on outside click, focus leaving both trigger and panel, or Escape. Anchored with useAnchoredPosition (side/align/sideOffset live on PopoverContent), which flips side when it would run off-screen and portals to document.body."
        code={`<Popover>
  <PopoverTrigger>Open popover</PopoverTrigger>
  <PopoverContent>
    <p className="font-medium">Dimensions</p>
    <p className="text-muted-foreground">Set the width and height.</p>
  </PopoverContent>
</Popover>`}
      >
        <PopoverDemo />
      </Entry>

      <Entry
        name="Tooltip"
        file="zeb/ui/tooltip"
        description="Opens on hover or focus after delayDuration; Escape dismisses it. Hiding is delayed and cancelled while the pointer is over the panel, so you can move the pointer onto the tooltip without it closing first. No TooltipProvider — delayDuration is a prop on each Tooltip instead."
        code={`<Tooltip delayDuration={200}>
  <TooltipTrigger><button>Hover or focus me</button></TooltipTrigger>
  <TooltipContent>Add to library</TooltipContent>
</Tooltip>`}
      >
        <TooltipDemo />
      </Entry>

      <Entry
        name="HoverCard"
        file="zeb/ui/hover-card"
        description="Opens 700ms after the pointer enters the trigger or the card itself; closes 150ms after it leaves both."
        code={`<HoverCard>
  <HoverCardTrigger><a href="/zebflow">@zebflow</a></HoverCardTrigger>
  <HoverCardContent>
    <p className="font-semibold">Zebflow</p>
    <p className="text-muted-foreground">Joined 2019.</p>
  </HoverCardContent>
</HoverCard>`}
      >
        <HoverCardDemo />
      </Entry>

      <Entry
        name="DropdownMenu"
        file="zeb/ui/dropdown-menu"
        description="Arrow keys move real DOM focus between items; Enter/Space activates; Tab closes the menu and moves on to the next tab stop (without trapping focus inside it); Escape and outside click close it too. A checkbox item stays open and toggles on click instead of dismissing."
        code={`<DropdownMenu>
  <DropdownMenuTrigger>Options</DropdownMenuTrigger>
  <DropdownMenuContent>
    <DropdownMenuLabel>My account</DropdownMenuLabel>
    <DropdownMenuSeparator />
    <DropdownMenuCheckboxItem checked={bookmarked} onCheckedChange={setBookmarked}>Bookmarked</DropdownMenuCheckboxItem>
    <DropdownMenuItem variant="destructive">Delete account</DropdownMenuItem>
  </DropdownMenuContent>
</DropdownMenu>`}
      >
        <DropdownMenuDemo />
      </Entry>

      <Entry
        name="ContextMenu"
        file="zeb/ui/context-menu"
        description="Opens at the pointer on a right-click (contextmenu), not anchored to the trigger element. Same item types and keyboard support as DropdownMenu."
        code={`<ContextMenu>
  <ContextMenuTrigger>Right click here</ContextMenuTrigger>
  <ContextMenuContent>
    <ContextMenuItem>Back</ContextMenuItem>
    <ContextMenuItem variant="destructive">Delete</ContextMenuItem>
  </ContextMenuContent>
</ContextMenu>`}
      >
        <ContextMenuDemo />
      </Entry>

      <Entry
        name="Select"
        file="zeb/ui/select"
        description="A listbox (role=listbox/option), not the native element — upstream's is too. Select owns the selected value through context (controlled with value/onValueChange, uncontrolled with defaultValue) the same way RadioGroup injects checked, so SelectItem only needs its own value. On open, focus moves to the selected option (else the first); ArrowUp/ArrowDown on the trigger opens the listbox too."
        code={`<Select value={value} onValueChange={setValue}>
  <SelectTrigger><SelectValue placeholder="Select a fruit" /></SelectTrigger>
  <SelectContent>
    <SelectGroup>
      <SelectLabel>Fruits</SelectLabel>
      <SelectItem value="blueberry">Blueberry</SelectItem>
    </SelectGroup>
  </SelectContent>
</Select>`}
      >
        <SelectDemo />
      </Entry>

      <Entry
        name="Sonner"
        file="zeb/ui/sonner"
        description="No npm sonner underneath — toast() and Toaster talk through a module-level subscriber list instead of props. Mount exactly one Toaster per page."
        code={`import { toast, Toaster } from "zeb/ui/sonner";

<Button onClick={() => toast.success("Saved successfully")}>Save</Button>
<Toaster /> {/* once, near the root */}`}
      >
        <SonnerDemo />
      </Entry>
    </div>
  );
}
