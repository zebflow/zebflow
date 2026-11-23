import Field from "@/components/ui/field";
import { Select, SelectOption } from "@/components/ui/select";
import Input from "@/components/ui/input";

/**
 * Where a package is published, and the credential that lets it go there.
 *
 * - A Hub repository is a Hub-format git repository, the official
 *   `github.com/zebflow/hub` by default: the package is committed and pushed
 *   with a token the git host accepts for that repository, used once and never
 *   stored.
 * - A Hub service is a Zebflow running the Hub: it takes a publisher token
 *   issued from Home > Hub.
 *
 * `publish` is `usePublishForm()`.
 */
export default function PublishDestination({ publish }) {
  const form = publish.form;
  const patch = publish.patch;
  const repository = form.destination === "hub_repository";

  return (
    <div className="space-y-3">
      <Field label="Publish to">
        <Select value={form.destination} onChange={(e) => patch({ destination: e.target.value })}>
          <SelectOption value="hub_repository" label="Hub repository (git, e.g. Zebflow Hub on GitHub)" />
          <SelectOption value="hub_service" label="Hub service (a Zebflow running the Hub)" />
        </Select>
      </Field>
      {repository ? (
        <>
          <div className="grid gap-3 md:grid-cols-[1fr_10rem]">
            <Field label="Repository">
              <Input value={form.repo_url} onInput={(e) => patch({ repo_url: e.target.value })} placeholder={publish.officialHubGitUrl} />
            </Field>
            <Field label="Branch">
              <Input value={form.branch} onInput={(e) => patch({ branch: e.target.value })} placeholder="main" />
            </Field>
          </div>
          <Field label="Repository token">
            <Input type="password" value={form.repo_token} onInput={(e) => patch({ repo_token: e.target.value })} placeholder="github_pat_..." />
          </Field>
          <p className="text-xs text-muted-foreground">
            A token the git host accepts for writing to this repository, e.g. a GitHub fine-grained token with Contents: read and write on that repository only. It is used for this push and not stored.
          </p>
          <Field label="Package page (Markdown file from the repository)">
            <Input value={form.readme_file_path} onInput={(e) => patch({ readme_file_path: e.target.value })} placeholder="README.md" />
          </Field>
        </>
      ) : (
        <Field label="Publisher Token">
          <Input type="password" value={form.publisher_token} onInput={(e) => patch({ publisher_token: e.target.value })} placeholder="zfmt_..." />
        </Field>
      )}
    </div>
  );
}
