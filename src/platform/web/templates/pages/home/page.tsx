import { Link, useRef } from "zeb";
import ChromeHeader from "@/pages/home/components/chrome-header";
import Button from "@/components/ui/button";
import Input from "@/components/ui/input";
import Card from "@/components/ui/card";
import CardContent from "@/components/ui/card-content";
import CardTitle from "@/components/ui/card-title";
import CardDescription from "@/components/ui/card-description";
import Field from "@/components/ui/field";
import DialogHeader from "@/components/ui/dialog-header";
import DialogTitle from "@/components/ui/dialog-title";
import DialogDescription from "@/components/ui/dialog-description";
import DialogFooter from "@/components/ui/dialog-footer";

const CREATE_DIALOG_CLASS =
  "backdrop:bg-slate-950/80 backdrop:backdrop-blur-sm p-0 rounded-lg border border-ui-border bg-ui-bg text-ui-text shadow-lg overflow-hidden w-full max-w-lg";

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
  const createDialogRef = useRef(null);
  const cloneDialogRef = useRef(null);

  const [provider, setProvider] = useState("gitlab");
  const [projectSlug, setProjectSlug] = useState("");

  const closeCreateDialog = () => {
    const el = createDialogRef.current;
    if (el && typeof el.close === "function") el.close();
  };

  const openCreateDialog = () => {
    const el = createDialogRef.current;
    if (el && typeof el.showModal === "function") el.showModal();
  };

  const closeCloneDialog = () => {
    const el = cloneDialogRef.current;
    if (el && typeof el.close === "function") el.close();
  };

  const openCloneDialog = () => {
    setProvider("gitlab");
    setProjectSlug("");
    const el = cloneDialogRef.current;
    if (el && typeof el.showModal === "function") el.showModal();
  };

  // Auto-derive project slug from the last path segment of the repo URL
  const handleRepoUrlInput = (e) => {
    const url = e.target.value.trim();
    if (!url) { setProjectSlug(""); return; }
    try {
      // strip trailing .git, take last path segment
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
          <header className="mb-10 flex flex-col gap-4 border-b border-slate-200 pb-4 sm:flex-row sm:items-end sm:justify-between">
            <div>
              <h1 className="text-3xl font-black tracking-tighter text-slate-900">
                Projects for {input.owner}
              </h1>
              <p className="mt-2 text-sm text-slate-500">Create and manage your automation projects.</p>
            </div>
            <div className="flex shrink-0 flex-wrap gap-2">
              <Button type="button" variant="primary" onClick={openCreateDialog}>
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
                <Card className="cursor-pointer transition-all hover:border-slate-300 hover:shadow-md">
                  <CardContent className="py-5">
                    <CardTitle className="text-lg">{item?.title}</CardTitle>
                    <CardDescription className="mt-1">{item?.project}</CardDescription>
                  </CardContent>
                </Card>
              </Link>
            ))}
          </section>
        </section>
      </main>

      {/* Create project dialog */}
      <dialog
        ref={createDialogRef}
        className={CREATE_DIALOG_CLASS}
        onClick={(e) => {
          if (e.target === e.currentTarget) closeCreateDialog();
        }}
      >
        <form method="post" action="/home/projects/create" className="flex flex-col">
          <div className="space-y-4 px-6 pt-6">
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
            </div>
          </div>
          <DialogFooter className="px-6 pb-6">
            <Button type="button" variant="outline" onClick={closeCreateDialog}>
              Cancel
            </Button>
            <Button type="submit" variant="primary">
              Create
            </Button>
          </DialogFooter>
        </form>
      </dialog>

      {/* Clone project dialog */}
      <dialog
        ref={cloneDialogRef}
        className={CREATE_DIALOG_CLASS}
        onClick={(e) => {
          if (e.target === e.currentTarget) closeCloneDialog();
        }}
      >
        <form method="post" action="/home/projects/clone" className="flex flex-col">
          <input type="hidden" name="provider" value={provider} />

          <div className="space-y-4 px-6 pt-6">
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
              {/* GitLab instance URL (only for gitlab) */}
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

              {/* Repo URL */}
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

              {/* Project slug (auto-derived, editable) */}
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

              {/* Username */}
              <Field label="Username" id="home-clone-username">
                <Input
                  type="text"
                  name="username"
                  id="home-clone-username"
                  placeholder={provider === "github" ? "GitHub username" : "GitLab username"}
                  required
                />
              </Field>

              {/* Access token with help tooltip */}
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

              {/* Committer name */}
              <Field label="Committer name" id="home-clone-git-name">
                <Input
                  type="text"
                  name="git_name"
                  id="home-clone-git-name"
                  placeholder="Your Name"
                />
              </Field>

              {/* Committer email */}
              <Field label="Committer email" id="home-clone-git-email">
                <Input
                  type="email"
                  name="git_email"
                  id="home-clone-git-email"
                  placeholder="you@example.com"
                />
              </Field>
            </div>
          </div>

          <DialogFooter className="px-6 pb-6">
            <Button type="button" variant="outline" onClick={closeCloneDialog}>
              Cancel
            </Button>
            <Button type="submit" variant="primary">
              Clone
            </Button>
          </DialogFooter>
        </form>
      </dialog>
    </>
  );
}
