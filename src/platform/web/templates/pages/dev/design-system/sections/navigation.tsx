import { useState } from "zeb/react";
import Tabs from "@/components/ui/tabs";
import TabsList from "@/components/ui/tabs-list";
import TabsTrigger from "@/components/ui/tabs-trigger";
import TabsContent from "@/components/ui/tabs-content";
import { StudioTabNav, StudioTabLink } from "@/components/ui/studio-tab-nav";
import Separator from "@/components/ui/separator";
import TreeView from "@/components/ui/tree-view";
import TreeItem from "@/components/ui/tree-item";
import HierarchyTree from "@/components/ui/hierarchy-tree";
import WebhookRouteTree from "@/components/ui/webhook-route-tree";
import RepoTree from "@/components/ui/repo-tree";
import RepoTreeRow from "@/components/ui/repo-tree-row";
import { SectionHeading, Entry } from "@/pages/dev/design-system/components/gallery";

const ROUTES = [
  { name: "login", title: "Login", webhook_method: "POST", webhook_path: "/auth/login", editor_href: "#login" },
  { name: "me", title: "Me", webhook_method: "GET", webhook_path: "/me", editor_href: "#me" },
  { name: "google-start", title: "Google start", webhook_method: "GET", webhook_path: "/auth/google/start", editor_href: "#gs" },
  { name: "google-callback", title: "Google callback", webhook_method: "GET", webhook_path: "/auth/google/callback", editor_href: "#gc" },
  { name: "admin", title: "Admin", webhook_method: "GET", webhook_path: "/auth/admin", editor_href: "#admin" },
];

const HIERARCHY = [
  {
    id: "pipelines", label: "pipelines", expanded: true,
    children: [
      { id: "auth", label: "auth", expanded: true, badge: "8", children: [
        { id: "login", label: "login.zf.json", active: true },
        { id: "me", label: "me.zf.json" },
      ] },
      { id: "test", label: "test", children: [{ id: "smoke", label: "smoke.zf.json" }] },
    ],
  },
  { id: "templates", label: "templates", children: [{ id: "pages", label: "pages" }] },
];

export default function NavigationSection({ owner, project }) {
  const [tab, setTab] = useState("runs");
  const [selected, setSelected] = useState("");

  return (
    <div>
      <SectionHeading title="Navigation" description="Ways to move around inside a page." />

      <Entry
        name="Tabs"
        file="tabs.tsx"
        description="Segmented tabs for switching content in place. State lives with the caller: `active` on each Trigger and Content."
        code={`<Tabs>
  <TabsList>
    <TabsTrigger label="Runs" active={tab === "runs"} onClick={() => setTab("runs")} />
    <TabsTrigger label="Settings" active={tab === "settings"} onClick={() => setTab("settings")} />
  </TabsList>
  <TabsContent active={tab === "runs"}>…</TabsContent>
  <TabsContent active={tab === "settings"}>…</TabsContent>
</Tabs>`}
      >
        <Tabs>
          <TabsList>
            <TabsTrigger label="Runs" active={tab === "runs"} onClick={() => setTab("runs")} />
            <TabsTrigger label="Settings" active={tab === "settings"} onClick={() => setTab("settings")} />
            <TabsTrigger label="Disabled" disabled />
          </TabsList>
          <TabsContent active={tab === "runs"}><p className="text-sm text-muted-foreground">12 runs, last one 4 minutes ago.</p></TabsContent>
          <TabsContent active={tab === "settings"}><p className="text-sm text-muted-foreground">Capture level: on-error.</p></TabsContent>
        </Tabs>
      </Entry>

      <Entry
        name="StudioTabNav"
        file="studio-tab-nav.tsx"
        description="The page-level tab bar under a studio header. Links, not buttons — each tab is a route."
        code={`<StudioTabNav>
  <StudioTabLink href="/…/pipelines/registry" active>Registry</StudioTabLink>
  <StudioTabLink href="/…/pipelines/webhooks">Webhooks</StudioTabLink>
</StudioTabNav>`}
        demoClassName="p-0 overflow-hidden"
      >
        <StudioTabNav>
          <StudioTabLink href="#" active>Registry</StudioTabLink>
          <StudioTabLink href="#">Webhooks</StudioTabLink>
          <StudioTabLink href="#">Schedules</StudioTabLink>
          <StudioTabLink href="#">Manual</StudioTabLink>
        </StudioTabNav>
      </Entry>

      <Entry
        name="Separator"
        file="separator.tsx"
        description="A hairline. `orientation` horizontal (default) | vertical."
        code={`<Separator />
<div className="flex h-6 items-center gap-3">a <Separator orientation="vertical" /> b</div>`}
      >
        <div className="grid gap-4 text-sm">
          <Separator />
          <div className="flex h-6 items-center gap-3">
            <span>registry</span>
            <Separator orientation="vertical" />
            <span>webhooks</span>
            <Separator orientation="vertical" />
            <span>schedules</span>
          </div>
        </div>
      </Entry>

      <Entry
        name="TreeView · TreeItem"
        file="tree-view.tsx"
        description="A <details>-based tree: works before hydration, expands natively. TreeItem: isFolder, expanded, href, active."
        code={`<TreeView>
  <TreeItem id="pipelines" label="pipelines" isFolder expanded>
    <TreeItem id="login" label="login.zf.json" href="#" active />
  </TreeItem>
</TreeView>`}
      >
        <TreeView>
          <TreeItem id="pipelines" label="pipelines" isFolder expanded>
            <TreeItem id="auth" label="auth" isFolder expanded>
              <TreeItem id="login" label="login.zf.json" href="#" active />
              <TreeItem id="me" label="me.zf.json" href="#" />
            </TreeItem>
            <TreeItem id="test" label="test" isFolder expanded={false}>
              <TreeItem id="smoke" label="smoke.zf.json" href="#" />
            </TreeItem>
          </TreeItem>
          <TreeItem id="templates" label="templates" isFolder expanded={false}>
            <TreeItem id="pages" label="pages" isFolder />
          </TreeItem>
        </TreeView>
      </Entry>

      <Entry
        name="HierarchyTree"
        file="hierarchy-tree.tsx"
        description="A data-driven tree: items of { id, label, icon?, badge?, href?, active?, expanded?, children? }. `defaultExpandedDepth` opens the first levels."
        code={`<HierarchyTree items={items} defaultExpandedDepth={2} emptyLabel="No pipelines." />`}
      >
        <HierarchyTree items={HIERARCHY} defaultExpandedDepth={2} emptyLabel="No pipelines." />
      </Entry>

      <Entry
        name="WebhookRouteTree"
        file="webhook-route-tree.tsx"
        description="Pipelines grouped by webhook path, one branch per segment, the method beside each leaf. Reads webhook_path, webhook_method, editor_href, title."
        code={`<WebhookRouteTree items={pipelineItems} />`}
      >
        <WebhookRouteTree items={ROUTES} />
      </Entry>

      <Entry
        name="RepoTreeRow"
        file="repo-tree-row.tsx"
        description="One row of the repository tree: kind icon, name, git status, a ⋯ context menu. RepoTree stacks these."
        code={`<RepoTreeRow item={{ kind: "folder", name: "pipelines", rel_path: "pipelines" }} depth={0} open />
<RepoTreeRow item={{ kind: "file", name: "login.zf.json", rel_path: "pipelines/auth/login.zf.json" }} depth={2} active status="M" />`}
      >
        <div className="max-w-sm">
          <RepoTreeRow item={{ kind: "root", name: "repo", rel_path: "" }} depth={0} open onActivate={() => {}} menu={[]} />
          <RepoTreeRow item={{ kind: "folder", name: "pipelines", rel_path: "pipelines" }} depth={1} open onActivate={() => {}} menu={[{ label: "New file" }]} />
          <RepoTreeRow item={{ kind: "folder", name: "auth", rel_path: "pipelines/auth" }} depth={2} open onActivate={() => {}} menu={[{ label: "New file" }]} />
          <RepoTreeRow item={{ kind: "file", name: "login.zf.json", rel_path: "pipelines/auth/login.zf.json" }} depth={3} active status="M" onActivate={() => {}} menu={[{ label: "Rename" }, { label: "Delete", variant: "destructive" }]} />
          <RepoTreeRow item={{ kind: "file", name: "me.zf.json", rel_path: "pipelines/auth/me.zf.json" }} depth={3} status="?" onActivate={() => {}} menu={[{ label: "Rename" }]} />
          <RepoTreeRow item={{ kind: "file", name: "me.tsx", rel_path: "templates/pages/me.tsx" }} depth={2} onActivate={() => {}} menu={[]} />
        </div>
      </Entry>

      <Entry
        name="RepoTree"
        file="repo-tree.tsx"
        description={project ? `Live against ${owner}/${project}: folders fetch on open, git status paints the rows.` : "Needs a project; the viewer has none yet."}
        code={`<RepoTree owner={owner} project={project} rootLabel={project} selected={selected} onSelect={(item) => setSelected(item.rel_path)} onAction={(action, item) => …} />`}
      >
        {project ? (
          <div className="grid gap-3">
            <div className="max-w-sm">
              <RepoTree owner={owner} project={project} rootLabel={project} selected={selected} onSelect={(item) => setSelected(String(item?.rel_path ?? ""))} onAction={() => {}} />
            </div>
            <span className="font-mono text-xs text-muted-foreground">selected: {selected || "(none)"}</span>
          </div>
        ) : (
          <p className="text-sm text-muted-foreground">No project to browse.</p>
        )}
      </Entry>
    </div>
  );
}
