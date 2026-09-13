import { useState } from "zeb/react";
import { Button, buttonVariants } from "zeb/ui/button";
import { Dialog, DialogTrigger, DialogContent, DialogHeader, DialogFooter, DialogTitle, DialogDescription, DialogClose } from "zeb/ui/dialog";
import {
  AlertDialog,
  AlertDialogTrigger,
  AlertDialogContent,
  AlertDialogHeader,
  AlertDialogFooter,
  AlertDialogTitle,
  AlertDialogDescription,
  AlertDialogAction,
  AlertDialogCancel,
} from "zeb/ui/alert-dialog";
import { Sheet, SheetTrigger, SheetContent, SheetHeader, SheetFooter, SheetTitle, SheetDescription, SheetClose } from "zeb/ui/sheet";
import { Drawer, DrawerTrigger, DrawerContent, DrawerHeader, DrawerFooter, DrawerTitle, DrawerDescription, DrawerClose } from "zeb/ui/drawer";
import { SectionHeading, Entry } from "@/pages/dev/design-system/components/gallery";

/**
 * zeb/ui — overlays, part 1: the hooks the family is built from, and the
 * four modal-panel components (Dialog, AlertDialog, Sheet, Drawer). Part 2
 * (zeb-ui-overlays-2.tsx) has the anchored panels, the two menus, Select,
 * and the toast stack. Every trigger below is real — click through them.
 * No Radix: every family composes through `createContext` instead, so a
 * shadcn snippet's nesting works unchanged.
 */

function DialogDemo() {
  const [open, setOpen] = useState(false);
  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogTrigger className={buttonVariants({ variant: "outline" })}>Edit profile</DialogTrigger>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Edit profile</DialogTitle>
          <DialogDescription>Make changes to your profile here. Click save when you're done.</DialogDescription>
        </DialogHeader>
        <DialogFooter>
          <DialogClose className={buttonVariants({ variant: "outline" })}>Cancel</DialogClose>
          <Button onClick={() => setOpen(false)}>Save changes</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

function AlertDialogDemo() {
  return (
    <AlertDialog>
      <AlertDialogTrigger className={buttonVariants({ variant: "destructive" })}>Delete account</AlertDialogTrigger>
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>Are you absolutely sure?</AlertDialogTitle>
          <AlertDialogDescription>
            This action cannot be undone. This will permanently delete the account and remove its data.
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel>Cancel</AlertDialogCancel>
          <AlertDialogAction>Continue</AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}

const SIDES = ["top", "right", "bottom", "left"];

function SheetDemo() {
  return (
    <div className="flex flex-wrap gap-3">
      {SIDES.map((side) => (
        <Sheet key={side}>
          <SheetTrigger className={buttonVariants({ variant: "outline", size: "sm" })}>{side}</SheetTrigger>
          <SheetContent side={side}>
            <SheetHeader>
              <SheetTitle>Edit profile</SheetTitle>
              <SheetDescription>{`Slides in from the ${side}. Closes on Escape, overlay click, or the button below.`}</SheetDescription>
            </SheetHeader>
            <SheetFooter>
              <SheetClose className={buttonVariants({ variant: "outline" })}>Close</SheetClose>
            </SheetFooter>
          </SheetContent>
        </Sheet>
      ))}
    </div>
  );
}

function DrawerDemo() {
  return (
    <Drawer>
      <DrawerTrigger className={buttonVariants({ variant: "outline" })}>Open drawer</DrawerTrigger>
      <DrawerContent>
        <DrawerHeader>
          <DrawerTitle>Move goal</DrawerTitle>
          <DrawerDescription>Set your daily activity goal.</DrawerDescription>
        </DrawerHeader>
        <DrawerFooter>
          <DrawerClose className={buttonVariants({ variant: "outline" })}>Cancel</DrawerClose>
        </DrawerFooter>
      </DrawerContent>
    </Drawer>
  );
}

export default function ZebUiOverlaysSection() {
  return (
    <div>
      <SectionHeading
        title="zeb/ui · Overlays (1/2)"
        description="No Radix: every family composes through createContext, and every panel traps focus, locks scroll, and closes on Escape itself. See zeb/ui/hooks for the behaviours underneath."
      />

      <Entry
        name="hooks"
        file="zeb/ui/hooks"
        description="useClickAway, useEscape, useFocusTrap, useControllable, useAnchoredPosition — every open/close/position/focus behaviour on this page and the next one is built from these five. useEscape and useFocusTrap arbitrate stacked overlays through a module-level stack, so a popover opened from inside a dialog only closes the popover on Escape. composeEventHandlers and isVNode are the two small helpers every trigger uses to compose with a caller's own onClick and to detect an already-interactive child (usually a Button) without cloneElement."
        code={`import { useControllable, useEscape, useFocusTrap, useClickAway, useAnchoredPosition } from "zeb/ui/hooks";

const [open, setOpen] = useControllable(props.open, false, props.onOpenChange);
useEscape(open, () => setOpen(false));
useFocusTrap(panelRef, open);`}
      >
        <p className="text-sm text-muted-foreground">
          Not a component — a sibling file the rest of zeb/ui imports from, the same way it imports a component.
        </p>
      </Entry>

      <Entry
        name="Dialog"
        file="zeb/ui/dialog"
        description="Controlled or uncontrolled. Dialog/DialogTrigger/DialogContent compose through context — no asChild, so DialogTrigger wraps an element child (usually Button) in a plain span instead of nesting a button inside one. Portals to document.body; traps focus and names itself via aria-labelledby/aria-describedby."
        code={`<Dialog open={open} onOpenChange={setOpen}>
  <DialogTrigger>Edit profile</DialogTrigger>
  <DialogContent>
    <DialogHeader>
      <DialogTitle>Edit profile</DialogTitle>
      <DialogDescription>Make changes here.</DialogDescription>
    </DialogHeader>
    <DialogFooter>
      <DialogClose>Cancel</DialogClose>
      <Button onClick={() => setOpen(false)}>Save</Button>
    </DialogFooter>
  </DialogContent>
</Dialog>`}
      >
        <DialogDemo />
      </Entry>

      <Entry
        name="AlertDialog"
        file="zeb/ui/alert-dialog"
        description="Escape closes it; a click on the overlay does not — an alert should not be dismissed by accident. Same context wiring as Dialog: role=&quot;alertdialog&quot;, aria-labelledby/aria-describedby, and a portal to document.body."
        code={`<AlertDialog>
  <AlertDialogTrigger>Delete account</AlertDialogTrigger>
  <AlertDialogContent>
    <AlertDialogHeader>
      <AlertDialogTitle>Are you absolutely sure?</AlertDialogTitle>
      <AlertDialogDescription>This cannot be undone.</AlertDialogDescription>
    </AlertDialogHeader>
    <AlertDialogFooter>
      <AlertDialogCancel>Cancel</AlertDialogCancel>
      <AlertDialogAction>Continue</AlertDialogAction>
    </AlertDialogFooter>
  </AlertDialogContent>
</AlertDialog>`}
      >
        <AlertDialogDemo />
      </Entry>

      <Entry
        name="Sheet"
        file="zeb/ui/sheet"
        description="side lives on SheetContent — top, right (default), bottom, or left. Same context wiring, portal, and aria-labelledby/aria-describedby as Dialog."
        code={`<Sheet>
  <SheetTrigger>right</SheetTrigger>
  <SheetContent side="right">
    <SheetHeader>
      <SheetTitle>Edit profile</SheetTitle>
    </SheetHeader>
    <SheetFooter><SheetClose>Close</SheetClose></SheetFooter>
  </SheetContent>
</Sheet>`}
      >
        <SheetDemo />
      </Entry>

      <Entry
        name="Drawer"
        file="zeb/ui/drawer"
        description="A bottom sheet with a grab handle. Dropped versus upstream (vaul): the swipe-to-dismiss gesture and the top/left/right directions — this is click-driven, always from the bottom."
        code={`<Drawer>
  <DrawerTrigger>Open drawer</DrawerTrigger>
  <DrawerContent>
    <DrawerHeader>
      <DrawerTitle>Move goal</DrawerTitle>
      <DrawerDescription>Set your daily activity goal.</DrawerDescription>
    </DrawerHeader>
    <DrawerFooter><DrawerClose>Cancel</DrawerClose></DrawerFooter>
  </DrawerContent>
</Drawer>`}
      >
        <DrawerDemo />
      </Entry>
    </div>
  );
}
