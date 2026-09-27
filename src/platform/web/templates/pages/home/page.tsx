import { useState } from "zeb/react";
import PendingInvitations from "@/pages/home/components/pending-invitations";
import ChromeHeader from "@/pages/home/components/chrome-header";
import OfficeDirectory from "@/pages/home/components/office-directory";
import Button from "@/components/ui/button";
import Input from "@/components/ui/input";
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
  "h-10 w-full rounded-md border border-border bg-popover px-3 text-sm text-foreground shadow-sm transition-all focus:border-ring/40 focus:outline-none focus:ring-1 focus:ring-ring/40";

export const page = {
  html: {
    lang: "en",
  },
  body: {
    className: "min-h-screen bg-background text-foreground font-sans",
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

export default function Page(input) {
  const offices = Array.isArray(input?.offices) ? input.offices : [];
  const runtimeTargets = Array.isArray(input?.runtime_targets)
    ? input.runtime_targets
    : [{ value: "local", label: "Local office", description: "" }];

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
              <h1 className="text-[40px] font-semibold leading-none tracking-tight text-foreground">
                Projects for <span className="text-primary">{input.owner}</span>
              </h1>
              <p className="mt-3 max-w-2xl text-base leading-6 text-muted-foreground">
                Every project, grouped by the office that holds it.
              </p>
              {input?.app_version ? (
                <p className="mt-1.5 font-mono text-[11px] tracking-wide text-muted-foreground">v{input.app_version}</p>
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

          <div className="my-8 h-px bg-border" />

          {/* Above the project list, because an invitation is about a project
              that is not in that list yet. */}
          <PendingInvitations />

          <OfficeDirectory offices={offices} canOpenRemote={Boolean(input?.can_open_remote)} />
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
