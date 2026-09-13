import { Tabs, TabsList, TabsTrigger, TabsContent } from "zeb/ui/tabs";
import { Accordion, AccordionItem, AccordionTrigger, AccordionContent } from "zeb/ui/accordion";
import { Collapsible, CollapsibleTrigger, CollapsibleContent } from "zeb/ui/collapsible";
import { ScrollArea } from "zeb/ui/scroll-area";
import { ResizablePanelGroup, ResizablePanel, ResizableHandle } from "zeb/ui/resizable";
import { SectionHeading, Entry } from "@/pages/dev/design-system/components/gallery";

/**
 * zeb/ui — Disclosure: tabs, accordion, collapsible, scroll-area, resizable.
 * Tabs and Accordion compose through context exactly the way their Radix
 * originals do, so a shadcn snippet's nesting works unchanged here — no
 * render-props, no required `active`/`value` props on the sub-parts. See
 * each component's own header comment for the real differences that remain.
 */

export default function ZebUiDisclosureSection() {
  return (
    <div>
      <SectionHeading
        title="zeb/ui · Disclosure"
        description="Tabs, accordion, collapsible, scroll area, and resizable panels — the same shadcn API, composed through context."
      />

      <Entry
        name="Tabs"
        file="zeb/ui/tabs"
        description="Tabs holds the active value (controlled or defaultValue) in context; TabsList/TabsTrigger/TabsContent read it at any depth. Arrow keys move focus across triggers."
        code={`import { Tabs, TabsList, TabsTrigger, TabsContent } from "zeb/ui/tabs";

<Tabs defaultValue="account">
  <TabsList>
    <TabsTrigger value="account">Account</TabsTrigger>
    <TabsTrigger value="password">Password</TabsTrigger>
  </TabsList>
  <TabsContent value="account">Account settings.</TabsContent>
  <TabsContent value="password">Change your password.</TabsContent>
</Tabs>`}
      >
        <Tabs defaultValue="account" className="w-full max-w-sm">
          <TabsList className="w-full">
            <TabsTrigger value="account">Account</TabsTrigger>
            <TabsTrigger value="password">Password</TabsTrigger>
            <TabsTrigger value="team" disabled>
              Team
            </TabsTrigger>
          </TabsList>
          <TabsContent value="account" className="rounded-md border border-border p-4 text-sm text-muted-foreground">
            Update your name and email address.
          </TabsContent>
          <TabsContent value="password" className="rounded-md border border-border p-4 text-sm text-muted-foreground">
            Change your password. You will be signed out everywhere else.
          </TabsContent>
        </Tabs>
      </Entry>

      <Entry
        name="Accordion"
        file="zeb/ui/accordion"
        description="Native <details>/<summary> — works before hydration. type='single' closes siblings through context; add collapsible to allow closing the last open item. value/defaultValue/onValueChange are real."
        code={`import { Accordion, AccordionItem, AccordionTrigger, AccordionContent } from "zeb/ui/accordion";

<Accordion type="single" collapsible defaultValue="item-1">
  <AccordionItem value="item-1">
    <AccordionTrigger>Is it accessible?</AccordionTrigger>
    <AccordionContent>Yes, it uses a native details element.</AccordionContent>
  </AccordionItem>
</Accordion>`}
      >
        <Accordion type="single" collapsible defaultValue="item-1" className="w-full max-w-sm">
          <AccordionItem value="item-1">
            <AccordionTrigger>Is it accessible?</AccordionTrigger>
            <AccordionContent>Yes. It renders a native &lt;details&gt;/&lt;summary&gt; pair, so it is a disclosure widget before any JavaScript runs.</AccordionContent>
          </AccordionItem>
          <AccordionItem value="item-2">
            <AccordionTrigger>Is it styled?</AccordionTrigger>
            <AccordionContent>Yes. It comes with default styles that match the rest of the theme's tokens.</AccordionContent>
          </AccordionItem>
          <AccordionItem value="item-3">
            <AccordionTrigger>Can it be animated?</AccordionTrigger>
            <AccordionContent>The chevron rotates with a transition; the open/close itself is the browser's native details behaviour.</AccordionContent>
          </AccordionItem>
        </Accordion>
      </Entry>

      <Entry
        name="Collapsible"
        file="zeb/ui/collapsible"
        description="A single native <details>. defaultOpen for uncontrolled use; open/onOpenChange to control it."
        code={`import { Collapsible, CollapsibleTrigger, CollapsibleContent } from "zeb/ui/collapsible";

<Collapsible defaultOpen={false}>
  <CollapsibleTrigger>Toggle</CollapsibleTrigger>
  <CollapsibleContent>Hidden content.</CollapsibleContent>
</Collapsible>`}
      >
        <Collapsible defaultOpen={false} className="w-full max-w-sm rounded-md border border-border px-4 py-3">
          <CollapsibleTrigger className="flex w-full items-center justify-between text-sm font-medium text-foreground">
            <span>@zebflow/starred-repos</span>
          </CollapsibleTrigger>
          <CollapsibleContent className="mt-2 flex flex-col gap-1.5 text-sm text-muted-foreground">
            <div className="rounded-md border border-border px-3 py-2 font-mono text-xs">zeb/react</div>
            <div className="rounded-md border border-border px-3 py-2 font-mono text-xs">zeb/ui</div>
          </CollapsibleContent>
        </Collapsible>
      </Entry>

      <Entry
        name="Scroll Area"
        file="zeb/ui/scroll-area"
        description="A plain overflow-auto div with a thinned native scrollbar (scrollbarWidth: thin). ScrollBar is a no-op export kept only for API compatibility."
        code={`import { ScrollArea } from "zeb/ui/scroll-area";

<ScrollArea className="h-40 w-full rounded-md border border-border p-4">
  {items.map((i) => <p key={i}>{i}</p>)}
</ScrollArea>`}
      >
        <ScrollArea className="h-40 w-full max-w-sm rounded-md border border-border p-4">
          {Array.from({ length: 12 }).map((_, i) => (
            <p key={i} className="border-b border-border py-1.5 text-sm text-foreground last:border-b-0">
              Tag {i + 1}
            </p>
          ))}
        </ScrollArea>
      </Entry>

      <Entry
        name="Resizable"
        file="zeb/ui/resizable"
        description="ResizableHandle drags flex-basis directly on its two DOM siblings, and reads direction from ResizablePanelGroup's context — no need to repeat it on every handle."
        code={`import { ResizablePanelGroup, ResizablePanel, ResizableHandle } from "zeb/ui/resizable";

<ResizablePanelGroup direction="horizontal" className="h-48 rounded-md border border-border">
  <ResizablePanel defaultSize={30}>Sidebar</ResizablePanel>
  <ResizableHandle withHandle />
  <ResizablePanel defaultSize={70}>Content</ResizablePanel>
</ResizablePanelGroup>`}
      >
        <ResizablePanelGroup direction="horizontal" className="h-48 w-full max-w-lg overflow-hidden rounded-md border border-border">
          <ResizablePanel defaultSize={30} className="flex items-center justify-center bg-muted text-sm text-muted-foreground">
            Sidebar
          </ResizablePanel>
          <ResizableHandle withHandle />
          <ResizablePanel defaultSize={70} className="flex items-center justify-center text-sm text-foreground">
            Drag the handle
          </ResizablePanel>
        </ResizablePanelGroup>
      </Entry>
    </div>
  );
}
