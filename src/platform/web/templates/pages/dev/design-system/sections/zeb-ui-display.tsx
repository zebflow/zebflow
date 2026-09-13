import { Alert, AlertTitle, AlertDescription } from "zeb/ui/alert";
import { Badge } from "zeb/ui/badge";
import { Card, CardHeader, CardTitle, CardDescription, CardAction, CardContent, CardFooter } from "zeb/ui/card";
import { Separator } from "zeb/ui/separator";
import { Skeleton } from "zeb/ui/skeleton";
import { Kbd, KbdGroup } from "zeb/ui/kbd";
import { Avatar, AvatarImage, AvatarFallback } from "zeb/ui/avatar";
import { Progress } from "zeb/ui/progress";
import { Button } from "zeb/ui/button";
import { SectionHeading, Entry, Matrix } from "@/pages/dev/design-system/components/gallery";

/**
 * zeb/ui — Display, part 1 of 2 (see zeb-ui-display-2.tsx for the rest of
 * YOUR GROUP). Static and near-static components: what a project reaches for
 * to show status, structure and identity without any interaction logic.
 */

const BADGE_VARIANTS = ["default", "secondary", "outline", "destructive", "ghost", "link"];

export default function ZebUiDisplaySection() {
  return (
    <div>
      <SectionHeading
        title="zeb/ui · Display (1/2)"
        description="Alert, Badge, Card, Separator, Skeleton, Kbd, Avatar, Progress — see zeb-ui-display-2 for Empty, Spinner, AspectRatio, Table, Breadcrumb, Pagination, Item."
      />

      <Entry
        name="Alert"
        file="zeb/ui/alert"
        description="An icon is a plain sibling svg sized col-start-1 row-span-2 — no auto has-[>svg] layout here."
        code={`import { Alert, AlertTitle, AlertDescription } from "zeb/ui/alert";

<Alert>
  <svg className="col-start-1 row-span-2 size-4" ... />
  <AlertTitle>Deployed</AlertTitle>
  <AlertDescription>Your changes are live.</AlertDescription>
</Alert>`}
      >
        <div className="flex flex-col gap-4">
          <Alert>
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className="col-start-1 row-span-2 size-4">
              <path d="M20 6 9 17l-5-5" />
            </svg>
            <AlertTitle>Deployed successfully</AlertTitle>
            <AlertDescription>Your changes are now live on production.</AlertDescription>
          </Alert>
          <Alert variant="destructive">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className="col-start-1 row-span-2 size-4">
              <circle cx="12" cy="12" r="10" />
              <line x1="12" y1="8" x2="12" y2="12" />
              <line x1="12" y1="16" x2="12.01" y2="16" />
            </svg>
            <AlertTitle>Deploy failed</AlertTitle>
            <AlertDescription>Build exited with a non-zero status.</AlertDescription>
          </Alert>
        </div>
      </Entry>

      <Entry
        name="Badge"
        file="zeb/ui/badge"
        description="No asChild — pass as='a' and an href instead."
        code={`import { Badge } from "zeb/ui/badge";

<Badge>Default</Badge>
<Badge variant="destructive">Failing</Badge>
<Badge as="a" href="#" variant="outline">Link</Badge>`}
      >
        <Matrix
          rows={BADGE_VARIANTS}
          cols={["badge"]}
          colLabel=""
          render={(variant) => <Badge variant={variant}>{variant}</Badge>}
        />
      </Entry>

      <Entry
        name="Card"
        file="zeb/ui/card"
        description="CardHeader / CardAction's has-data-[…] grid shift is manual — pass grid-cols-[1fr_auto] yourself."
        code={`import { Card, CardHeader, CardTitle, CardDescription, CardAction, CardContent, CardFooter } from "zeb/ui/card";

<Card>
  <CardHeader className="grid-cols-[1fr_auto]">
    <CardTitle>Project alpha</CardTitle>
    <CardDescription>Created 3 days ago</CardDescription>
    <CardAction><Button size="sm" variant="outline">Edit</Button></CardAction>
  </CardHeader>
  <CardContent>...</CardContent>
  <CardFooter className="border-t pt-6">...</CardFooter>
</Card>`}
      >
        <Card className="max-w-sm">
          <CardHeader className="grid-cols-[1fr_auto]">
            <CardTitle>Project alpha</CardTitle>
            <CardDescription>Created 3 days ago</CardDescription>
            <CardAction>
              <Button size="sm" variant="outline">Edit</Button>
            </CardAction>
          </CardHeader>
          <CardContent>
            <p className="text-sm text-muted-foreground">Three pipelines active, no failing runs in the last 24 hours.</p>
          </CardContent>
          <CardFooter className="border-t pt-6">
            <Button size="sm">Open project</Button>
          </CardFooter>
        </Card>
      </Entry>

      <Entry
        name="Separator"
        file="zeb/ui/separator"
        description="A plain div: sets data-orientation itself, no Radix."
        code={`import { Separator } from "zeb/ui/separator";

<Separator />
<Separator orientation="vertical" className="h-6" />`}
      >
        <div className="flex flex-col gap-4">
          <div>
            <p className="text-sm text-foreground">Above</p>
            <Separator className="my-3" />
            <p className="text-sm text-foreground">Below</p>
          </div>
          <div className="flex h-6 items-center gap-3 text-sm text-foreground">
            <span>Left</span>
            <Separator orientation="vertical" />
            <span>Right</span>
          </div>
        </div>
      </Entry>

      <Entry
        name="Skeleton"
        file="zeb/ui/skeleton"
        code={`import { Skeleton } from "zeb/ui/skeleton";

<Skeleton className="h-12 w-12 rounded-full" />
<Skeleton className="h-4 w-40" />`}
      >
        <div className="flex items-center gap-4">
          <Skeleton className="size-12 rounded-full" />
          <div className="flex flex-col gap-2">
            <Skeleton className="h-4 w-40" />
            <Skeleton className="h-4 w-28" />
          </div>
        </div>
      </Entry>

      <Entry
        name="Kbd"
        file="zeb/ui/kbd"
        code={`import { Kbd, KbdGroup } from "zeb/ui/kbd";

<Kbd>⌘</Kbd>
<KbdGroup><Kbd>Ctrl</Kbd><Kbd>K</Kbd></KbdGroup>`}
      >
        <div className="flex items-center gap-4">
          <Kbd>⌘</Kbd>
          <Kbd>Enter</Kbd>
          <KbdGroup>
            <Kbd>Ctrl</Kbd>
            <Kbd>K</Kbd>
          </KbdGroup>
        </div>
      </Entry>

      <Entry
        name="Avatar"
        file="zeb/ui/avatar"
        description="AvatarImage falls back to AvatarFallback on onError — no mount-order fallback like Radix."
        code={`import { Avatar, AvatarImage, AvatarFallback } from "zeb/ui/avatar";

<Avatar>
  <AvatarImage src="/broken.png" alt="Ada" />
  <AvatarFallback>AL</AvatarFallback>
</Avatar>
<Avatar size="lg"><AvatarFallback>ZB</AvatarFallback></Avatar>`}
      >
        <div className="flex items-center gap-4">
          <Avatar size="sm">
            <AvatarFallback>SM</AvatarFallback>
          </Avatar>
          <Avatar>
            <AvatarImage src="data:image/png;base64,broken" alt="Ada Lovelace" />
            <AvatarFallback>AL</AvatarFallback>
          </Avatar>
          <Avatar size="lg">
            <AvatarFallback>ZB</AvatarFallback>
          </Avatar>
        </div>
      </Entry>

      <Entry
        name="Progress"
        file="zeb/ui/progress"
        description="value is 0-100."
        code={`import { Progress } from "zeb/ui/progress";

<Progress value={33} />
<Progress value={78} />`}
      >
        <div className="flex flex-col gap-4">
          <Progress value={13} />
          <Progress value={58} />
          <Progress value={92} />
        </div>
      </Entry>
    </div>
  );
}
