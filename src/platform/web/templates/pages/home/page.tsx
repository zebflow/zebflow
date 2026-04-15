import { Link } from "zeb";
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

export const page = {
  head: {
    title: ctx?.seo?.title ?? "",
    description: ctx?.seo?.description ?? "",
  },
  html: {
    lang: "en",
  },
  body: {
    className: "min-h-screen bg-zinc-50 text-gray-900 font-sans",
  },
  navigation: "history",
};

export default function Page(input) {
  const projects = Array.isArray(input?.projects) ? input.projects : [];
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

      <main className="pb-16 pt-24">
        <section className="mx-auto max-w-6xl px-6">
          <header className="mb-10 flex flex-col gap-4 border-b border-gray-200 pb-4 sm:flex-row sm:items-end sm:justify-between">
            <div>
              <h1 className="text-3xl font-black tracking-tighter text-gray-900">
                Projects for {input.owner}
              </h1>
              <p className="mt-2 text-sm text-gray-500">
                Create and manage automation projects inside this office.
              </p>
              {input?.app_version ? (
                <p className="mt-1 text-[0.7rem] text-gray-400 tracking-wide">v{input.app_version}</p>
              ) : null}
            </div>
            <div className="flex shrink-0 flex-wrap gap-2">
              <Button type="button" variant="primary" onClick={() => setCreateOpen(true)}>
                Create project
              </Button>
              <Button type="button" variant="outline" onClick={openCloneDialog}>
                Clone project
              </Button>
              <Link href="/home/project-templates" className="inline-flex hover:no-underline">
                <Button as="span" variant="outline">
                  See templates
                </Button>
              </Link>
            </div>
          </header>

          <section className="grid gap-5 md:grid-cols-2 lg:grid-cols-3">
            {projects.map((item, index) => (
              <Link
                key={`${item?.project ?? "project"}-${index}`}
                href={item?.path ?? "#"}
                className="block hover:no-underline"
              >
                    <Card className="cursor-pointer transition-all hover:border-gray-300 hover:shadow-md">
                      <CardContent className="py-5">
                        <CardTitle className="text-lg">{item?.title}</CardTitle>
                        <CardDescription className="mt-1">{item?.project}</CardDescription>
                        <div className="mt-4 space-y-1 text-xs text-gray-500">
                          <p>
                            <span className="font-medium text-gray-700">Runtime:</span>{" "}
                            {item?.runtime_mode || "shared"} · {item?.runtime_summary || "Local office"}
                          </p>
                          <p>
                            <span className="font-medium text-gray-700">Office:</span>{" "}
                            {item?.office_label || "Local office"}
                          </p>
                          <p className="truncate">
                            <span className="font-medium text-gray-700">Address:</span>{" "}
                            {item?.office_url || "Uses the current office address"}
                          </p>
                        </div>
                      </CardContent>
                    </Card>
              </Link>
            ))}
          </section>

          <section className="mt-12">
            <header className="mb-4">
              <h2 className="text-xl font-black tracking-tight text-gray-900">Office status</h2>
              <p className="mt-1 text-sm text-gray-500">
                Current office inventory, runtime availability, and placement health.
              </p>
            </header>
            <div className="grid gap-5 md:grid-cols-2 lg:grid-cols-3">
              {offices.map((office, index) => {
                const availability = String(office?.availability || "unknown");
                const availabilityTone =
                  availability === "online"
                    ? "bg-emerald-50 text-emerald-700 border-emerald-200"
                    : availability === "dangling"
                      ? "bg-amber-50 text-amber-700 border-amber-200"
                      : "bg-gray-100 text-gray-700 border-gray-200";
                const projects = Array.isArray(office?.hosted_projects) ? office.hosted_projects : [];
                const capabilities = Array.isArray(office?.capabilities) ? office.capabilities : [];
                return (
                  <Card key={`${office?.id ?? "office"}-${index}`}>
                    <CardContent className="py-5">
                      <div className="flex items-start justify-between gap-3">
                        <div>
                          <CardTitle className="text-lg">{office?.label || office?.id}</CardTitle>
                          <CardDescription className="mt-1">{office?.role || "Office"}</CardDescription>
                        </div>
                        <span
                          className={`inline-flex rounded-full border px-2 py-1 text-[0.7rem] font-semibold uppercase tracking-wide ${availabilityTone}`}
                        >
                          {availability}
                        </span>
                      </div>
                      <div className="mt-4 space-y-1 text-xs text-gray-500">
                        <p>
                          <span className="font-medium text-gray-700">State:</span>{" "}
                          {office?.resource_state || "unknown"}
                        </p>
                        <p className="truncate">
                          <span className="font-medium text-gray-700">Address:</span>{" "}
                          {office?.address || "No advertised address"}
                        </p>
                        <p>
                          <span className="font-medium text-gray-700">Version:</span>{" "}
                          {office?.version || "unknown"}
                        </p>
                        <p>
                          <span className="font-medium text-gray-700">Last seen:</span>{" "}
                          {office?.last_seen || "unknown"}
                        </p>
                        <p>
                          <span className="font-medium text-gray-700">Hosted projects:</span>{" "}
                          {office?.hosted_project_count ?? 0}
                        </p>
                        <p className="truncate">
                          <span className="font-medium text-gray-700">Capabilities:</span>{" "}
                          {capabilities.length > 0 ? capabilities.join(", ") : "none declared"}
                        </p>
                        {projects.length > 0 ? (
                          <p className="truncate">
                            <span className="font-medium text-gray-700">Examples:</span>{" "}
                            {projects.slice(0, 3).join(", ")}
                            {projects.length > 3 ? ` +${projects.length - 3} more` : ""}
                          </p>
                        ) : null}
                      </div>
                    </CardContent>
                  </Card>
                );
              })}
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
                    className="h-10 w-full rounded-xl border border-gray-300 bg-white px-3 text-sm text-gray-900"
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
                    className="h-10 w-full rounded-xl border border-gray-300 bg-white px-3 text-sm text-gray-900"
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
                    className="h-10 w-full rounded-xl border border-gray-300 bg-white px-3 text-sm text-gray-900"
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
                    className="h-10 w-full rounded-xl border border-gray-300 bg-white px-3 text-sm text-gray-900"
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
