import { Link, useState } from "zeb/react";
import PendingInvitations from "@/pages/home/components/pending-invitations";
import ChromeHeader from "@/pages/home/components/chrome-header";
import Button from "@/components/ui/button";
import Input from "@/components/ui/input";
import Card from "@/components/ui/card";
import CardContent from "@/components/ui/card-content";
import CardTitle from "@/components/ui/card-title";
import CardDescription from "@/components/ui/card-description";
import Field from "@/components/ui/field";
import { Dialog } from "@/components/ui/dialog";
import DialogContent from "@/components/ui/dialog-content";
import DialogHeader from "@/components/ui/dialog-header";
import DialogTitle from "@/components/ui/dialog-title";
import DialogDescription from "@/components/ui/dialog-description";
import DialogFooter from "@/components/ui/dialog-footer";

const GITLAB_TOKEN_HELP =
  "In GitLab: User Settings → Access Tokens. Create a token with read_repository + write_repository scopes.";
const GITHUB_TOKEN_HELP =
  "In GitHub: Settings → Developer settings → Personal access tokens → Tokens (classic). Select the repo scope.";
const SELECT_CLASS =
  "h-10 w-full rounded-md border border-ui-border bg-ui-bg px-3 text-sm text-ui-text shadow-sm transition-all focus:border-brand-blue/40 focus:outline-none focus:ring-1 focus:ring-brand-blue/40";

export const page = {
  html: {
    lang: "en",
  },
  body: {
    className: "min-h-screen bg-ui-bg-muted text-ui-text font-sans",
  },
  navigation: "history",
};

export function getPage(input) {
  return {
    head: {
      title: input?.seo?.title ?? "",
      description: input?.seo?.description ?? "",
    },
  };
}

function SectionHeading({ title, description }) {
  return (
    <header className="mb-5">
      <h2 className="text-[26px] font-semibold leading-tight tracking-tight text-ui-text">{title}</h2>
      <p className="mt-1 max-w-2xl text-[15px] leading-6 text-ui-text-soft">{description}</p>
    </header>
  );
}

function DetailRow({ label, children, mono = false }) {
  return (
    <p className={mono ? "truncate font-mono text-[11.5px] text-ui-text-muted" : "text-[13.5px] leading-7 text-ui-text-soft"}>
      <span className={mono ? "font-sans font-semibold text-ui-text" : "font-semibold text-ui-text"}>{label}:</span>{" "}
      {children}
    </p>
  );
}

function ProjectCard({ item, index }) {
  const accent = item?.is_app ? "var(--color-brand-orange)" : index % 3 === 2 ? "var(--color-brand-blue)" : "var(--color-ui-text-muted)";
  const primaryAction = item?.open_app_path ? "Play" : "Edit";
  const primaryHref = item?.open_app_path || item?.edit_path || item?.path || "#";

  return (
    <Card className="relative overflow-hidden rounded-[14px] transition-all hover:border-ui-border-strong hover:shadow-md">
      {item?.is_app ? (
        <div aria-hidden="true" className="pointer-events-none absolute right-0 top-0 h-16 w-32 opacity-35">
          <svg width="128" height="64" viewBox="0 0 128 64" fill="none">
            <path d="M0 42 C 32 42, 42 16, 74 16 S 116 38, 128 24" stroke="var(--color-brand-orange)" strokeWidth="1.5" strokeDasharray="2 7" />
          </svg>
        </div>
      ) : null}
      <CardContent className="relative p-6">
        <div className="flex items-start justify-between gap-4">
          <div className="min-w-0">
            <CardTitle className="truncate text-[21px] font-semibold leading-tight">{item?.title}</CardTitle>
            <CardDescription className="mt-1 truncate font-mono text-xs">{item?.project}</CardDescription>
          </div>
          <span className="mt-1 h-[9px] w-[9px] shrink-0 rounded-[3px]" style={{ backgroundColor: accent }} />
        </div>

        <div className="mt-[18px]">
          <DetailRow label="Runtime">
            {item?.runtime_mode || "shared"} · {item?.runtime_summary || "Local office"}
          </DetailRow>
          <DetailRow label="Office">{item?.office_label || "Local office"}</DetailRow>
          <DetailRow label="Address" mono>
            {item?.office_url || "Uses the current office address"}
          </DetailRow>
        </div>

        <div className="mt-5 flex flex-wrap gap-2">
          <Link href={primaryHref} className="inline-flex hover:no-underline">
            <Button as="span" variant="primary" size="sm">
              {primaryAction}
            </Button>
          </Link>
          {item?.open_app_path ? (
            <Link href={item?.edit_path ?? item?.path ?? "#"} className="inline-flex hover:no-underline">
              <Button as="span" variant="outline" size="sm">
                Edit
              </Button>
            </Link>
          ) : null}
        </div>
      </CardContent>
    </Card>
  );
}

function StatusBadge({ status }) {
  const value = String(status || "unknown");
  const tone =
    value === "online"
      ? "border-emerald-200 bg-emerald-50 text-emerald-700"
      : value === "dangling"
        ? "border-amber-200 bg-amber-50 text-amber-700"
        : "border-ui-border bg-ui-bg-muted text-ui-text-soft";
  return (
    <span className={`inline-flex rounded-full border px-2.5 py-1 font-mono text-[10px] font-semibold uppercase tracking-[0.08em] ${tone}`}>
      {value}
    </span>
  );
}

function OfficeCard({ office, index }) {
  const projects = Array.isArray(office?.hosted_projects) ? office.hosted_projects : [];
  const capabilities = Array.isArray(office?.capabilities) ? office.capabilities : [];

  return (
    <Card key={`${office?.id ?? "office"}-${index}`} className="rounded-[14px]">
      <CardContent className="p-[22px]">
        <div className="flex items-start justify-between gap-4">
          <div className="min-w-0">
            <CardTitle className="truncate text-[19px] font-semibold leading-tight">{office?.label || office?.id}</CardTitle>
            <CardDescription className="mt-1 text-[13px]">{office?.role || "Office"}</CardDescription>
          </div>
          <StatusBadge status={office?.availability} />
        </div>
        <div className="my-4 h-px bg-ui-border" />
        <div>
          <DetailRow label="State">{office?.resource_state || "unknown"}</DetailRow>
          <DetailRow label="Address" mono>
            {office?.address || "No advertised address"}
          </DetailRow>
          <DetailRow label="Version">{office?.version || "unknown"}</DetailRow>
          <DetailRow label="Last seen">{office?.last_seen || "unknown"}</DetailRow>
          <DetailRow label="Hosted projects">{office?.hosted_project_count ?? 0}</DetailRow>
          <DetailRow label="Capabilities">{capabilities.length > 0 ? capabilities.join(", ") : "none declared"}</DetailRow>
          {projects.length > 0 ? (
            <DetailRow label="Examples">
              {projects.slice(0, 3).join(", ")}
              {projects.length > 3 ? ` +${projects.length - 3} more` : ""}
            </DetailRow>
          ) : null}
        </div>
        {office?.open_url ? (
          <div className="mt-4">
            <Button as="a" href={office.open_url} variant="outline" size="sm">
              Open office
            </Button>
          </div>
        ) : null}
      </CardContent>
    </Card>
  );
}

export default function Page(input) {
  const initialProjects = Array.isArray(input?.projects) ? input.projects : [];
  const offices = Array.isArray(input?.offices) ? input.offices : [];
  const runtimeTargets = Array.isArray(input?.runtime_targets)
    ? input.runtime_targets
    : [{ value: "local", label: "Local office", description: "" }];

  const [projects, setProjects] = useState(initialProjects);
  const [createOpen, setCreateOpen] = useState(false);
  const [cloneOpen, setCloneOpen] = useState(false);
  const [provider, setProvider] = useState("gitlab");
  const [projectSlug, setProjectSlug] = useState("");
  const [createBranch, setCreateBranch] = useState("main");
  const [createRuntimeMode, setCreateRuntimeMode] = useState("shared");
  const [createPlacementWorker, setCreatePlacementWorker] = useState("local");
  const [remoteBranch, setRemoteBranch] = useState("main");
  const [localBranch, setLocalBranch] = useState("main");
  const [cloneRuntimeMode, setCloneRuntimeMode] = useState("shared");
  const [clonePlacementWorker, setClonePlacementWorker] = useState("local");

  const openCloneDialog = () => {
    setProvider("gitlab");
    setProjectSlug("");
    setRemoteBranch("main");
    setLocalBranch("main");
    setCloneRuntimeMode("shared");
    setClonePlacementWorker("local");
    setCloneOpen(true);
  };

  const handleRemoteBranchInput = (e) => {
    const val = e.target.value;
    if (localBranch === remoteBranch) setLocalBranch(val);
    setRemoteBranch(val);
  };

  // Auto-derive project slug from the last path segment of the repo URL
  const handleRepoUrlInput = (e) => {
    const url = e.target.value.trim();
    if (!url) { setProjectSlug(""); return; }
    try {
      const clean = url.replace(/\.git$/, "").replace(/\/$/, "");
      const parts = clean.split("/");
      const last = parts[parts.length - 1] || "";
      setProjectSlug(last.toLowerCase().replace(/[^a-z0-9-_]/g, "-").replace(/-+/g, "-").replace(/^-|-$/g, ""));
    } catch (_) {}
  };

  const tokenHelp = provider === "github" ? GITHUB_TOKEN_HELP : GITLAB_TOKEN_HELP;

  return (
    <>
      <ChromeHeader />

      <main className="pb-20 pt-28">
        <section className="mx-auto w-full max-w-[1960px] px-6 sm:px-10">
          <header className="flex flex-col gap-6 sm:flex-row sm:items-end sm:justify-between">
            <div>
              <h1 className="text-[40px] font-semibold leading-none tracking-tight text-ui-text">
                Projects for <span className="text-brand-orange">{input.owner}</span>
              </h1>
              <p className="mt-3 max-w-2xl text-base leading-6 text-ui-text-soft">
                Create and manage automation projects inside this office.
              </p>
              {input?.app_version ? (
                <p className="mt-1.5 font-mono text-[11px] tracking-wide text-ui-text-muted">v{input.app_version}</p>
              ) : null}
            </div>
            <div className="flex shrink-0 flex-wrap gap-3">
              <Button type="button" variant="primary" onClick={() => setCreateOpen(true)}>
                Create project
              </Button>
              <Button type="button" variant="outline" onClick={openCloneDialog}>
                Clone project
              </Button>
              <Button as="a" href="/hub" variant="outline">
                Hub
              </Button>
            </div>
          </header>

          <div className="my-8 h-px bg-ui-border" />

          {/* Above the project list, because an invitation is about a project
              that is not in that list yet. */}
          <PendingInvitations />

          <section className="grid gap-5 md:grid-cols-2 lg:grid-cols-3">
            {projects.map((item, index) => (
              <ProjectCard key={`${item?.project ?? "project"}-${index}`} item={item} index={index} />
            ))}
          </section>

          <section className="mt-14">
            <SectionHeading
              title="Office status"
              description="Current office inventory, runtime availability, and placement health."
            />
            <div className="grid gap-5 md:grid-cols-2 lg:grid-cols-3">
              {offices.map((office, index) => (
                <OfficeCard key={`${office?.id ?? "office"}-${index}`} office={office} index={index} />
              ))}
            </div>
          </section>
        </section>
      </main>

      {/* Create project dialog */}
      <Dialog open={createOpen} onOpenChange={setCreateOpen}>
        <DialogContent>
          <form method="post" action="/home/projects/create" className="flex flex-col">
            <div className="space-y-4 px-6 pt-6 pb-2">
              <DialogHeader>
                <DialogTitle>Create project</DialogTitle>
                <DialogDescription>Choose a URL slug and an optional display title.</DialogDescription>
              </DialogHeader>
              <div className="space-y-4">
                <Field label="Project slug" id="home-create-slug">
                  <Input
                    type="text"
                    name="project"
                    id="home-create-slug"
                    placeholder="e.g. my-app"
                    required
                  />
                </Field>
                <Field label="Title" id="home-create-title">
                  <Input type="text" name="title" id="home-create-title" placeholder="Display name" />
                </Field>
                <Field label="Default local branch" id="home-create-branch">
                  <Input
                    type="text"
                    name="local_branch"
                    id="home-create-branch"
                    placeholder="main"
                    value={createBranch}
                    onInput={(e) => setCreateBranch(e.target.value)}
                  />
                </Field>
                <Field label="Runtime mode" id="home-create-runtime-mode">
                  <select
                    id="home-create-runtime-mode"
                    name="runtime_mode"
                    value={createRuntimeMode}
                    onChange={(e) => setCreateRuntimeMode(e.target.value)}
                    className={SELECT_CLASS}
                  >
                    <option value="shared">Shared</option>
                    <option value="pinned">Pinned</option>
                    <option value="dedicated">Dedicated</option>
                  </select>
                </Field>
                <Field
                  label="Office target"
                  id="home-create-placement-worker"
                  description="Local keeps the project inside this office. Pick another office to place the runtime remotely."
                >
                  <select
                    id="home-create-placement-worker"
                    name="placement_worker_id"
                    value={createPlacementWorker}
                    onChange={(e) => setCreatePlacementWorker(e.target.value)}
                    className={SELECT_CLASS}
                  >
                    {runtimeTargets.map((item) => (
                      <option key={item.value} value={item.value}>
                        {item.label}
                      </option>
                    ))}
                  </select>
                </Field>
              </div>
            </div>
            <DialogFooter className="px-6 pb-6">
              <Button type="button" variant="outline" onClick={() => setCreateOpen(false)}>
                Cancel
              </Button>
              <Button type="submit" variant="primary">
                Create
              </Button>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>

      {/* Clone project dialog */}
      <Dialog open={cloneOpen} onOpenChange={setCloneOpen}>
        <DialogContent>
          <form method="post" action="/home/projects/clone" className="flex flex-col">
            <input type="hidden" name="provider" value={provider} />

            <div className="space-y-4 px-6 pt-6 pb-2">
              <DialogHeader>
                <DialogTitle>Clone project</DialogTitle>
                <DialogDescription>Clone a remote Git repository into a new project.</DialogDescription>
              </DialogHeader>

              {/* Provider tabs */}
              <div className="flex gap-2">
                <Button
                  type="button"
                  variant={provider === "gitlab" ? "primary" : "outline"}
                  size="sm"
                  onClick={() => setProvider("gitlab")}
                >
                  GitLab
                </Button>
                <Button
                  type="button"
                  variant={provider === "github" ? "primary" : "outline"}
                  size="sm"
                  onClick={() => setProvider("github")}
                >
                  GitHub
                </Button>
              </div>

              <div className="space-y-4">
                {provider === "gitlab" && (
                  <Field label="GitLab instance URL" id="home-clone-instance-url">
                    <Input
                      type="url"
                      name="instance_url"
                      id="home-clone-instance-url"
                      placeholder="https://gitlab.com"
                      defaultValue="https://gitlab.com"
                    />
                  </Field>
                )}

                <Field label="Repository URL" id="home-clone-repo-url">
                  <Input
                    type="url"
                    name="repo_url"
                    id="home-clone-repo-url"
                    placeholder={provider === "github" ? "https://github.com/user/repo.git" : "https://gitlab.com/user/repo.git"}
                    required
                    onInput={handleRepoUrlInput}
                  />
                </Field>

                <Field label="Project slug" id="home-clone-slug">
                  <Input
                    type="text"
                    name="project"
                    id="home-clone-slug"
                    placeholder="auto-derived from URL"
                    value={projectSlug}
                    onInput={(e) => setProjectSlug(e.target.value)}
                    required
                  />
                </Field>

                <Field
                  label="Remote branch"
                  id="home-clone-remote-branch"
                  description="Branch on the remote repository to clone from."
                >
                  <Input
                    type="text"
                    name="remote_branch"
                    id="home-clone-remote-branch"
                    placeholder="main"
                    value={remoteBranch}
                    onInput={handleRemoteBranchInput}
                  />
                </Field>

                <Field
                  label="Local branch name"
                  id="home-clone-local-branch"
                  description="Name for the local branch (leave same as remote, or rename e.g. dev)."
                >
                  <Input
                    type="text"
                    name="local_branch"
                    id="home-clone-local-branch"
                    placeholder="main"
                    value={localBranch}
                    onInput={(e) => setLocalBranch(e.target.value)}
                  />
                </Field>

                <Field label="Runtime mode" id="home-clone-runtime-mode">
                  <select
                    id="home-clone-runtime-mode"
                    name="runtime_mode"
                    value={cloneRuntimeMode}
                    onChange={(e) => setCloneRuntimeMode(e.target.value)}
                    className={SELECT_CLASS}
                  >
                    <option value="shared">Shared</option>
                    <option value="pinned">Pinned</option>
                    <option value="dedicated">Dedicated</option>
                  </select>
                </Field>

                <Field
                  label="Office target"
                  id="home-clone-placement-worker"
                  description="Choose which office should host the cloned project's resident runtime."
                >
                  <select
                    id="home-clone-placement-worker"
                    name="placement_worker_id"
                    value={clonePlacementWorker}
                    onChange={(e) => setClonePlacementWorker(e.target.value)}
                    className={SELECT_CLASS}
                  >
                    {runtimeTargets.map((item) => (
                      <option key={item.value} value={item.value}>
                        {item.label}
                      </option>
                    ))}
                  </select>
                </Field>

                <Field label="Username" id="home-clone-username">
                  <Input
                    type="text"
                    name="username"
                    id="home-clone-username"
                    placeholder={provider === "github" ? "GitHub username" : "GitLab username"}
                    required
                  />
                </Field>

                <Field
                  label="Access token"
                  id="home-clone-token"
                  description={tokenHelp}
                >
                  <Input
                    type="password"
                    name="token"
                    id="home-clone-token"
                    placeholder="Paste your access token"
                    required
                  />
                </Field>

                <Field label="Committer name" id="home-clone-git-name">
                  <Input
                    type="text"
                    name="git_name"
                    id="home-clone-git-name"
                    placeholder="Your Name"
                    required
                  />
                </Field>

                <Field label="Committer email" id="home-clone-git-email">
                  <Input
                    type="email"
                    name="git_email"
                    id="home-clone-git-email"
                    placeholder="you@example.com"
                    required
                  />
                </Field>
              </div>
            </div>

            <DialogFooter className="px-6 pb-6">
              <Button type="button" variant="outline" onClick={() => setCloneOpen(false)}>
                Cancel
              </Button>
              <Button type="submit" variant="primary">
                Clone
              </Button>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>
    </>
  );
}
