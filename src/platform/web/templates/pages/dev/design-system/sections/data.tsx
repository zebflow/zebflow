import Card from "@/components/ui/card";
import CardHeader from "@/components/ui/card-header";
import CardTitle from "@/components/ui/card-title";
import CardDescription from "@/components/ui/card-description";
import CardContent from "@/components/ui/card-content";
import CardFooter from "@/components/ui/card-footer";
import Badge from "@/components/ui/badge";
import Alert from "@/components/ui/alert";
import Button from "@/components/ui/button";
import { StudioTable, StudioThead, StudioTh, StudioTd } from "@/components/ui/studio-data-table";
import { Markdown } from "zeb/markdown";
import FileKindIcon from "@/components/ui/file-kind-icon";
import ColorSwatch from "@/components/ui/color-swatch";
import { SectionHeading, Entry } from "@/pages/dev/design-system/components/gallery";

const FILES = ["login.zf.json", "me.tsx", "notes.md", "schema.sql", "logo.png", "data.csv", "settings.yaml", "index.ts", "styles.css", "readme", "photo.jpg", "bundle.js"];

const ROWS = [
  ["auth/login", "POST /auth/login", "active", "4m ago"],
  ["auth/me", "GET /me", "active", "4m ago"],
  ["auth/google-callback", "GET /auth/google/callback", "active", "1h ago"],
  ["test/smoke", "manual", "inactive", "—"],
];

const MD = `## Release notes

- **Sandbox**: scripts are validated at save time.
- \`n.auth.token.verify\` is new — see \`help("pipeline/nodes")\`.

| node | pins |
|---|---|
| logic.if | true, false |
| kv.get | out |

> Help describes the implementation. It does not define it.`;

export default function DataSection() {
  return (
    <div>
      <SectionHeading title="Data" description="Containers and displays for content." />

      <Entry
        name="Card"
        file="card.tsx"
        description="A raised panel on bg-card. Header, Title, Description, Content, Footer are the sections."
        code={`<Card>
  <CardHeader>
    <CardTitle>Members</CardTitle>
    <CardDescription>Accepted accounts. Sign-in requires a row here.</CardDescription>
  </CardHeader>
  <CardContent>…</CardContent>
  <CardFooter><Button size="sm">Add member</Button></CardFooter>
</Card>`}
        demoClassName="bg-background"
      >
        <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
          <Card>
            <CardHeader>
              <CardTitle>Members</CardTitle>
              <CardDescription>Accepted accounts. Sign-in requires a row here.</CardDescription>
            </CardHeader>
            <CardContent>
              <p className="text-3xl font-bold tracking-tight">42</p>
              <p className="text-xs text-muted-foreground">+3 this week</p>
            </CardContent>
            <CardFooter>
              <Button size="sm">Add member</Button>
            </CardFooter>
          </Card>
          <Card>
            <CardHeader>
              <CardTitle>Registrations</CardTitle>
              <CardDescription>Waiting for a human to approve.</CardDescription>
            </CardHeader>
            <CardContent>
              <p className="text-3xl font-bold tracking-tight">7</p>
              <p className="text-xs text-muted-foreground">oldest 2 days</p>
            </CardContent>
          </Card>
        </div>
      </Entry>

      <Entry
        name="Badge"
        file="badge.tsx"
        description="A small label. Variants: default | secondary | destructive | outline."
        code={`<Badge>default</Badge>
<Badge variant="secondary">secondary</Badge>
<Badge variant="destructive">destructive</Badge>
<Badge variant="outline">outline</Badge>`}
      >
        <div className="flex flex-wrap items-center gap-3">
          <Badge>default</Badge>
          <Badge variant="secondary">secondary</Badge>
          <Badge variant="destructive">destructive</Badge>
          <Badge variant="outline">outline</Badge>
          <Badge variant="outline" className="text-success border-success/40">● live</Badge>
        </div>
      </Entry>

      <Entry
        name="Alert"
        file="alert.tsx"
        description="An inline notice. Variants: info | success | warning | error — each a tinted status token."
        code={`<Alert variant="warning">The signing key was regenerated; existing sessions are invalid.</Alert>`}
      >
        <div className="grid gap-3">
          <Alert variant="info">Templates are compiled into the binary; restart to see edits.</Alert>
          <Alert variant="success">Pipeline registered — 12 nodes.</Alert>
          <Alert variant="warning">The signing key was regenerated; existing sessions are invalid.</Alert>
          <Alert variant="error">FW_NODE_SQLITE_MUTATE: NOT NULL constraint failed: members.name</Alert>
        </div>
      </Entry>

      <Entry
        name="StudioTable"
        file="studio-data-table.tsx"
        description="The studio's data table: mono uppercase headers, hairline rows. `variant='dbGrid'` for the database grid. `studioTableTdClass` is exported for behaviours that build cells in JS."
        code={`<StudioTable>
  <StudioThead><tr><StudioTh>Pipeline</StudioTh><StudioTh>Trigger</StudioTh></tr></StudioThead>
  <tbody>{rows.map((r) => <tr><StudioTd>{r.id}</StudioTd><StudioTd>{r.trigger}</StudioTd></tr>)}</tbody>
</StudioTable>`}
        demoClassName="p-0 overflow-hidden"
      >
        <StudioTable>
          <StudioThead>
            <tr>
              <StudioTh>Pipeline</StudioTh>
              <StudioTh>Trigger</StudioTh>
              <StudioTh>Status</StudioTh>
              <StudioTh>Last run</StudioTh>
            </tr>
          </StudioThead>
          <tbody>
            {ROWS.map((r) => (
              <tr key={r[0]}>
                <StudioTd className="font-mono">{r[0]}</StudioTd>
                <StudioTd className="font-mono text-muted-foreground">{r[1]}</StudioTd>
                <StudioTd>
                  <Badge variant={r[2] === "active" ? "outline" : "secondary"} className={r[2] === "active" ? "text-success border-success/40" : ""}>{r[2]}</Badge>
                </StudioTd>
                <StudioTd className="text-muted-foreground">{r[3]}</StudioTd>
              </tr>
            ))}
          </tbody>
        </StudioTable>
      </Entry>

      <Entry
        name="Markdown"
        file="zeb/markdown"
        description="Not a ui/ file — a Zeb library. The server renders it to HTML before the page leaves; styled by the .prose rules. `renderMarkdown(text)` is the same thing as a string."
        code={`import { Markdown } from "zeb/markdown";
<Markdown content={text} />`}
      >
        <Markdown content={MD} />
      </Entry>

      <Entry
        name="FileKindIcon"
        file="file-kind-icon.tsx"
        description="An icon for a file name, by extension. `.zf.json` is a pipeline, not just JSON."
        code={`<FileKindIcon name="login.zf.json" />`}
      >
        <div className="grid grid-cols-2 gap-3 sm:grid-cols-4">
          {FILES.map((f) => (
            <div key={f} className="flex items-center gap-2 font-mono text-xs text-foreground">
              <FileKindIcon name={f} /> {f}
            </div>
          ))}
        </div>
      </Entry>

      <Entry
        name="ColorSwatch"
        file="color-swatch.tsx"
        description="A named colour with its value — for palettes and theme editors, not for the theme itself (see Theme)."
        code={`<ColorSwatch name="ink" value="#1b1f24" />`}
      >
        <div className="grid grid-cols-2 gap-4 sm:grid-cols-4">
          <ColorSwatch name="zeb-ink" value="#1b1f24" />
          <ColorSwatch name="zeb-bg" value="#e9edf3" />
          <ColorSwatch name="brand-orange" value="#ea5a0c" />
          <ColorSwatch name="brand-blue" value="#1e66d6" />
        </div>
      </Entry>
    </div>
  );
}
