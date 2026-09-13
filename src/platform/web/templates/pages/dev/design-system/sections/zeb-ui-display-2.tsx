import { Empty, EmptyHeader, EmptyMedia, EmptyTitle, EmptyDescription, EmptyContent } from "zeb/ui/empty";
import { Spinner } from "zeb/ui/spinner";
import { AspectRatio } from "zeb/ui/aspect-ratio";
import { Table, TableHeader, TableBody, TableRow, TableHead, TableCell, TableCaption } from "zeb/ui/table";
import { Breadcrumb, BreadcrumbList, BreadcrumbItem, BreadcrumbLink, BreadcrumbPage, BreadcrumbSeparator } from "zeb/ui/breadcrumb";
import { Pagination, PaginationContent, PaginationItem, PaginationLink, PaginationPrevious, PaginationNext } from "zeb/ui/pagination";
import { Item, ItemMedia, ItemContent, ItemActions, ItemGroup, ItemSeparator, ItemTitle, ItemDescription } from "zeb/ui/item";
import { Button } from "zeb/ui/button";
import { CodeBlock } from "zeb/ui/code-block";
import { SectionHeading, Entry } from "@/pages/dev/design-system/components/gallery";

/**
 * zeb/ui — Display, part 2 of 2 (see zeb-ui-display.tsx for Alert, Badge,
 * Card, Separator, Skeleton, Kbd, Avatar, Progress). Structural and
 * layout-shaped display components.
 */

export default function ZebUiDisplay2Section() {
  return (
    <div>
      <SectionHeading
        title="zeb/ui · Display (2/2)"
        description="Empty, Spinner, AspectRatio, Table, Breadcrumb, Pagination, Item."
      />


      <Entry
        name="CodeBlock"
        file="zeb/ui/code-block"
        description="Source with syntax colour from a tiny built-in tokenizer — no library, rendered on the server. Language label, copy button, optional line numbers. tsx · ts · js · json · python · shell · rust · sql · sekejap."
        code={`import { CodeBlock } from "zeb/ui/code-block";

<CodeBlock language="tsx" title="pages/home.tsx" lineNumbers code={source} />
<CodeBlock language="sql" code={'SELECT * FROM members WHERE email = $1'} />`}
      >
        <div className="grid gap-4">
          <CodeBlock language="tsx" title="pages/home.tsx" lineNumbers code={`import { Button } from "zeb/ui/button";
import "@/globals.css";

// A page is a function of its pipeline's payload.
export default function Home(input) {
  const [count, setCount] = useState(0);
  return <Button onClick={() => setCount(count + 1)}>Clicked {count} times</Button>;
}`} />
          <div className="grid gap-4 md:grid-cols-2">
            <CodeBlock language="rust" title="src/main.rs" code={`#[derive(Debug)]
struct Run { id: u64, ok: bool }

fn main() -> Result<(), String> {
    let run = Run { id: 42, ok: true };
    println!("{:?}", run); // prints the run
    Ok(())
}`} />
            <CodeBlock language="python" title="score.py" code={`import json

@cache
def score(rows: list[dict]) -> float:
    """Mean of the 'value' column."""
    total = sum(r["value"] for r in rows)
    return total / max(len(rows), 1)`} />
            <CodeBlock language="shell" title="deploy.sh" code={`#!/usr/bin/env bash
set -euo pipefail
export ZEBFLOW_PORT=10610
curl -s --cookie-jar /tmp/zf.txt -X POST "$BASE/login" -d "identifier=$USER"
./dev.sh --release   # rebuilds in ~30s`} />
            <CodeBlock language="sekejap" title="members.ql" code={`-- accepted members, newest first
SELECT email, name, roles
FROM members
WHERE accepted_at > $1 AND roles LIKE '%admin%'
ORDER BY accepted_at DESC
LIMIT 20`} />
          </div>
        </div>
      </Entry>

      <Entry
        name="Empty"
        file="zeb/ui/empty"
        code={`import { Empty, EmptyHeader, EmptyMedia, EmptyTitle, EmptyDescription, EmptyContent } from "zeb/ui/empty";

<Empty>
  <EmptyHeader>
    <EmptyMedia variant="icon"><svg className="size-6" ... /></EmptyMedia>
    <EmptyTitle>No pipelines yet</EmptyTitle>
    <EmptyDescription>Register one from the CLI or the registry page.</EmptyDescription>
  </EmptyHeader>
  <EmptyContent><Button size="sm">New pipeline</Button></EmptyContent>
</Empty>`}
      >
        <Empty className="border">
          <EmptyHeader>
            <EmptyMedia variant="icon">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className="size-6">
                <path d="M12 2v20M2 12h20" />
              </svg>
            </EmptyMedia>
            <EmptyTitle>No pipelines yet</EmptyTitle>
            <EmptyDescription>Register one from the CLI or the registry page.</EmptyDescription>
          </EmptyHeader>
          <EmptyContent>
            <Button size="sm">New pipeline</Button>
          </EmptyContent>
        </Empty>
      </Entry>

      <Entry
        name="Spinner"
        file="zeb/ui/spinner"
        description="An inline svg (no lucide here) — same glyph as upstream's Loader2Icon."
        code={`import { Spinner } from "zeb/ui/spinner";

<Spinner />
<Spinner className="size-6 text-primary" />`}
      >
        <div className="flex items-center gap-4">
          <Spinner />
          <Spinner className="size-6 text-primary" />
          <Spinner className="size-8 text-muted-foreground" />
        </div>
      </Entry>

      <Entry
        name="AspectRatio"
        file="zeb/ui/aspect-ratio"
        description="A plain div with an inline aspect-ratio style — no Radix here."
        code={`import { AspectRatio } from "zeb/ui/aspect-ratio";

<AspectRatio ratio={16 / 9} className="rounded-lg bg-muted">
  <img src="..." className="h-full w-full object-cover" />
</AspectRatio>`}
      >
        <AspectRatio ratio={16 / 9} className="max-w-sm overflow-hidden rounded-lg bg-muted">
          <div className="flex h-full w-full items-center justify-center text-sm text-muted-foreground">16:9</div>
        </AspectRatio>
      </Entry>

      <Entry
        name="Table"
        file="zeb/ui/table"
        code={`import { Table, TableHeader, TableBody, TableRow, TableHead, TableCell, TableCaption } from "zeb/ui/table";

<Table>
  <TableCaption>Recent pipeline runs.</TableCaption>
  <TableHeader><TableRow><TableHead>Run</TableHead><TableHead>Status</TableHead></TableRow></TableHeader>
  <TableBody><TableRow><TableCell>#42</TableCell><TableCell>Success</TableCell></TableRow></TableBody>
</Table>`}
      >
        <Table>
          <TableCaption>Recent pipeline runs.</TableCaption>
          <TableHeader>
            <TableRow>
              <TableHead>Run</TableHead>
              <TableHead>Trigger</TableHead>
              <TableHead>Status</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            <TableRow>
              <TableCell>#42</TableCell>
              <TableCell>webhook</TableCell>
              <TableCell>Success</TableCell>
            </TableRow>
            <TableRow>
              <TableCell>#41</TableCell>
              <TableCell>schedule</TableCell>
              <TableCell>Success</TableCell>
            </TableRow>
            <TableRow data-state="selected">
              <TableCell>#40</TableCell>
              <TableCell>manual</TableCell>
              <TableCell>Failed</TableCell>
            </TableRow>
          </TableBody>
        </Table>
      </Entry>

      <Entry
        name="Breadcrumb"
        file="zeb/ui/breadcrumb"
        description="BreadcrumbLink always renders an <a> — no asChild."
        code={`import { Breadcrumb, BreadcrumbList, BreadcrumbItem, BreadcrumbLink, BreadcrumbPage, BreadcrumbSeparator } from "zeb/ui/breadcrumb";

<Breadcrumb>
  <BreadcrumbList>
    <BreadcrumbItem><BreadcrumbLink href="/home">Home</BreadcrumbLink></BreadcrumbItem>
    <BreadcrumbSeparator />
    <BreadcrumbItem><BreadcrumbPage>default</BreadcrumbPage></BreadcrumbItem>
  </BreadcrumbList>
</Breadcrumb>`}
      >
        <Breadcrumb>
          <BreadcrumbList>
            <BreadcrumbItem>
              <BreadcrumbLink href="/home">Home</BreadcrumbLink>
            </BreadcrumbItem>
            <BreadcrumbSeparator />
            <BreadcrumbItem>
              <BreadcrumbLink href="/projects/superadmin/default">default</BreadcrumbLink>
            </BreadcrumbItem>
            <BreadcrumbSeparator />
            <BreadcrumbItem>
              <BreadcrumbPage>pipelines</BreadcrumbPage>
            </BreadcrumbItem>
          </BreadcrumbList>
        </Breadcrumb>
      </Entry>

      <Entry
        name="Pagination"
        file="zeb/ui/pagination"
        description="PaginationLink's classes come from zeb/ui/button's buttonVariants."
        code={`import { Pagination, PaginationContent, PaginationItem, PaginationLink, PaginationPrevious, PaginationNext } from "zeb/ui/pagination";

<Pagination>
  <PaginationContent>
    <PaginationItem><PaginationPrevious href="#" /></PaginationItem>
    <PaginationItem><PaginationLink href="#" isActive>1</PaginationLink></PaginationItem>
    <PaginationItem><PaginationLink href="#">2</PaginationLink></PaginationItem>
    <PaginationItem><PaginationNext href="#" /></PaginationItem>
  </PaginationContent>
</Pagination>`}
      >
        <Pagination>
          <PaginationContent>
            <PaginationItem>
              <PaginationPrevious href="#" />
            </PaginationItem>
            <PaginationItem>
              <PaginationLink href="#" isActive>1</PaginationLink>
            </PaginationItem>
            <PaginationItem>
              <PaginationLink href="#">2</PaginationLink>
            </PaginationItem>
            <PaginationItem>
              <PaginationLink href="#">3</PaginationLink>
            </PaginationItem>
            <PaginationItem>
              <PaginationNext href="#" />
            </PaginationItem>
          </PaginationContent>
        </Pagination>
      </Entry>

      <Entry
        name="Item"
        file="zeb/ui/item"
        description="Sub-parts import Separator from zeb/ui/separator, not radix."
        code={`import { Item, ItemMedia, ItemContent, ItemActions, ItemGroup, ItemSeparator, ItemTitle, ItemDescription } from "zeb/ui/item";

<ItemGroup>
  <Item variant="outline">
    <ItemMedia variant="icon"><svg className="size-4" ... /></ItemMedia>
    <ItemContent><ItemTitle>fs-thumb-check</ItemTitle><ItemDescription>webhook · active</ItemDescription></ItemContent>
    <ItemActions><Button size="sm" variant="ghost">Disable</Button></ItemActions>
  </Item>
  <ItemSeparator />
  <Item variant="outline">...</Item>
</ItemGroup>`}
      >
        <ItemGroup className="gap-2">
          <Item variant="outline">
            <ItemMedia variant="icon">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className="size-4">
                <path d="M13 2 3 14h9l-1 8 10-12h-9l1-8Z" />
              </svg>
            </ItemMedia>
            <ItemContent>
              <ItemTitle>fs-thumb-check</ItemTitle>
              <ItemDescription>webhook · active</ItemDescription>
            </ItemContent>
            <ItemActions>
              <Button size="sm" variant="ghost">Disable</Button>
            </ItemActions>
          </Item>
          <ItemSeparator />
          <Item variant="muted">
            <ItemMedia variant="icon">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className="size-4">
                <circle cx="12" cy="12" r="10" />
                <polyline points="12 6 12 12 16 14" />
              </svg>
            </ItemMedia>
            <ItemContent>
              <ItemTitle>nightly-export</ItemTitle>
              <ItemDescription>schedule · paused</ItemDescription>
            </ItemContent>
            <ItemActions>
              <Button size="sm" variant="ghost">Enable</Button>
            </ItemActions>
          </Item>
        </ItemGroup>
      </Entry>
    </div>
  );
}
