import { useState, cx, Link } from "zeb";
import { StudioTabNav, StudioTabLink } from "@/components/ui/studio-tab-nav";
import ProjectStudioShell from "@/pages/project-studio/components/shell";
import Checkbox from "@/components/ui/checkbox";
import Input from "@/components/ui/input";
import Textarea from "@/components/ui/textarea";
import Badge from "@/components/ui/badge";
import Separator from "@/components/ui/separator";
import Button from "@/components/ui/button";
import Card from "@/components/ui/card";
import CardContent from "@/components/ui/card-content";
import CardDescription from "@/components/ui/card-description";
import CommitDialog from "@/components/ui/commit-dialog";
import Field from "@/components/ui/field";
import { Select, SelectOption } from "@/components/ui/select";

export const page = {
  head: {
    title: ctx?.seo?.title ?? "",
    description: ctx?.seo?.description ?? "",
  },
  html: {
    lang: "en",
  },
  body: {
    className: "font-sans",
  },
  navigation: "history",
};

// ─── Shared helpers ────────────────────────────────────────────────────────

async function requestJson(url, options = {}) {
  return fetch(url, {
    headers: {
      Accept: "application/json",
      ...(options.body ? { "Content-Type": "application/json" } : {}),
      ...(options.headers || {}),
    },
    ...options,
  }).then(async (response) => {
    if (response.status === 401) { window.location.href = "/login"; return null; }
    const payload = await response.json().catch(() => null);
    if (!response.ok) {
      const message =
        payload?.error?.message ||
        payload?.message ||
        payload?.error ||
        `${response.status} ${response.statusText}`;
      throw new Error(message);
    }
    return payload;
  });
}

function renderCardGrid(items) {
  const rows = Array.isArray(items) ? items : [];
  return rows.map((item, index) => (
    <a key={`${item?.href ?? "item"}-${index}`} href={item?.href ?? "#"} className="project-card block">
      <div className="flex items-start justify-between gap-3">
        <div>
          <h3 className="project-card-title">{item?.title}</h3>
          <p className="project-card-copy">{item?.description}</p>
        </div>
        {item?.tag ? <span className="project-inline-chip">{item.tag}</span> : null}
      </div>
    </a>
  ));
}

// ─── RWE Settings Panel ─────────────────────────────────────────────────────

function RwePanel({ api, initialConfig, owner, project }) {
  const [allowList, setAllowList] = useState(
    (initialConfig?.allow_list ?? []).join("\n")
  );
  const [minify, setMinify] = useState(Boolean(initialConfig?.minify_html));
  const [strict, setStrict] = useState(initialConfig?.strict_mode !== false);
  const [deploymentAssetBase, setDeploymentAssetBase] = useState(
    String(initialConfig?.deployment_asset_base ?? "")
  );
  const [statusMsg, setStatusMsg] = useState("Ready.");
  const [statusTone, setStatusTone] = useState("info");
  const [saving, setSaving] = useState(false);
  const [clearing, setClearing] = useState(false);
  const [commitOpen, setCommitOpen] = useState(false);
  const [pendingData, setPendingData] = useState(null);

  function handleSubmit(e) {
    e.preventDefault();
    setPendingData({
      allow_list: allowList.split(/[\n,]/).map((s) => s.trim()).filter(Boolean),
      minify_html: minify,
      strict_mode: strict,
      deployment_asset_base: deploymentAssetBase.trim() || null,
    });
    setCommitOpen(true);
  }

  async function handleCommit(commitMessage) {
    setCommitOpen(false);
    setSaving(true);
    setStatusMsg("Saving...");
    setStatusTone("info");
    try {
      const resp = await requestJson(api, {
        method: "PUT",
        body: JSON.stringify({ commit_message: commitMessage, data: pendingData }),
      });
      if (resp?.committed) {
        setStatusMsg("Saved & committed.");
        setStatusTone("ok");
      } else if (resp?.git_error) {
        setStatusMsg(`Saved (git: ${resp.git_error})`);
        setStatusTone("info");
      } else {
        setStatusMsg("Saved.");
        setStatusTone("ok");
      }
    } catch (err) {
      setStatusMsg(`Failed: ${err?.message || String(err)}`);
      setStatusTone("error");
    } finally {
      setSaving(false);
      setPendingData(null);
    }
  }

  async function handleClearCache() {
    setClearing(true);
    try {
      const res = await fetch(`/api/projects/${owner}/${project}/rwe/cache/clear`, { method: "POST" });
      if (res.ok) {
        setStatusMsg("Template cache cleared.");
        setStatusTone("ok");
      } else {
        setStatusMsg("Failed to clear cache.");
        setStatusTone("error");
      }
    } finally {
      setClearing(false);
    }
  }

  return (
    <article className="border border-border rounded-xl bg-surface p-[0.85rem] mb-[0.9rem]">
      <CommitDialog
        open={commitOpen}
        section="rwe"
        defaultMessage="settings(rwe): update RWE config"
        onConfirm={handleCommit}
        onCancel={() => { setCommitOpen(false); setPendingData(null); }}
      />
      <header className="flex items-start justify-between gap-3 mb-[0.65rem]">
        <div>
          <h3 className="project-card-title">Reactive Web Engine</h3>
          <p className="project-card-copy">
            Project-level compile and render controls for all <code>n.web.render</code> nodes.
          </p>
        </div>
        <span className="project-inline-chip">RWE</span>
      </header>

      <form className="grid grid-cols-2 gap-[0.65rem]" onSubmit={handleSubmit}>
        <label className="pipeline-editor-field">
          <span>Script Allow List</span>
          <Textarea
            name="allow_list"
            rows={4}
            placeholder={"https://cdnjs.cloudflare.com/*\nhttps://cdn.jsdelivr.net/*"}
            value={allowList}
            onInput={(e) => setAllowList(e.currentTarget.value)}
          />
          <small className="pipeline-editor-field-help">
            One URL pattern per line (or comma-separated). Controls which external scripts and
            stylesheets <code>--load-scripts</code> may reference. Blessed <code>zeb/*</code>{" "}
            libraries are always allowed and do not appear here.
          </small>
        </label>

        <div className="flex flex-col gap-2 pt-1">
          <Checkbox
            name="minify_html"
            label="Minify HTML output"
            checked={minify}
            onChange={(e) => setMinify(e.target.checked)}
          />
          <Checkbox
            name="strict_mode"
            label="Strict compile-time checks"
            checked={strict}
            onChange={(e) => setStrict(e.target.checked)}
          />
        </div>

        <label className="pipeline-editor-field col-span-full">
          <span>Asset Base Path</span>
          <Input
            name="deployment_asset_base"
            placeholder={`/assets/${owner ?? "owner"}/${project ?? "project"}`}
            value={deploymentAssetBase}
            onInput={(e) => setDeploymentAssetBase(e.currentTarget.value)}
          />
          <small className="pipeline-editor-field-help">
            Replaces the default <code>/assets/{"{owner}/{project}"}</code> prefix in all rendered HTML — scripts, images, uploads, library chunks. E.g. set to <code>/my/custom/path</code> and <code>/assets/{"{owner}/{project}"}/rwe/…</code> becomes <code>/my/custom/path/rwe/…</code>. Leave empty to keep the default.
          </small>
        </label>

        <div className="col-span-full flex items-center gap-[0.7rem]">
          <Button
            type="submit"
            variant="primary"
            size="sm"
            disabled={saving}
            label={saving ? "Saving..." : "Save RWE Config"}
          />
          <Button
            type="button"
            variant="outline"
            size="sm"
            disabled={clearing}
            onClick={handleClearCache}
            label={clearing ? "Clearing…" : "Clear Template Cache"}
          />
          <span className={cx("text-[0.72rem]", statusTone === "ok" ? "text-[color-mix(in_srgb,var(--color-accent)_80%,#e6f9ef)]" : statusTone === "error" ? "text-red-300" : "text-body-soft")}>{statusMsg}</span>
        </div>
      </form>
    </article>
  );
}

// ─── Assistant Settings Panel ───────────────────────────────────────────────

function AssistantPanel({ api, credentials, initialConfig }) {
  const creds = Array.isArray(credentials) ? credentials : [];
  const [highModel, setHighModel] = useState(
    String(initialConfig?.llm_high_credential_id || "")
  );
  const [generalModel, setGeneralModel] = useState(
    String(initialConfig?.llm_general_credential_id || "")
  );
  const [maxSteps, setMaxSteps] = useState(Number(initialConfig?.max_steps ?? 50));
  const [maxReplans, setMaxReplans] = useState(Number(initialConfig?.max_replans ?? 2));
  const [historyPairs, setHistoryPairs] = useState(
    Number(initialConfig?.chat_history_pairs ?? 10)
  );
  const [enabled, setEnabled] = useState(Boolean(initialConfig?.enabled));
  const [statusMsg, setStatusMsg] = useState("Ready.");
  const [statusTone, setStatusTone] = useState("info");
  const [saving, setSaving] = useState(false);

  async function handleSubmit(e) {
    e.preventDefault();
    setSaving(true);
    setStatusMsg("Saving...");
    setStatusTone("info");
    try {
      await requestJson(api, {
        method: "PUT",
        body: JSON.stringify({
          llm_high_credential_id: highModel.trim() || null,
          llm_general_credential_id: generalModel.trim() || null,
          max_steps: maxSteps,
          max_replans: maxReplans,
          chat_history_pairs: historyPairs,
          enabled,
        }),
      });
      setStatusMsg("Saved.");
      setStatusTone("ok");
    } catch (err) {
      setStatusMsg(`Failed: ${err?.message || String(err)}`);
      setStatusTone("error");
    } finally {
      setSaving(false);
    }
  }

  return (
    <article className="border border-border rounded-xl bg-surface p-[0.85rem] mb-[0.9rem]">
      <header className="flex items-start justify-between gap-3 mb-[0.65rem]">
        <div>
          <h3 className="project-card-title">Project Assistant</h3>
          <p className="project-card-copy">Bind credential profiles for assistant reasoning tiers.</p>
        </div>
        <span className="project-inline-chip">Automaton</span>
      </header>

      <form className="grid grid-cols-2 gap-[0.65rem]" onSubmit={handleSubmit}>
        <label className="pipeline-editor-field">
          <span>High Model</span>
          <Select name="llm_high_credential_id" value={highModel} onChange={(e) => setHighModel(e.target.value)}>
            <SelectOption value="" label="None" />
            {creds.map((item, index) => (
              <SelectOption
                key={`${item?.credential_id ?? "credential"}-${index}`}
                value={item?.credential_id ?? ""}
                label={`${item?.title} · ${item?.credential_id}`}
              />
            ))}
          </Select>
          <small className="pipeline-editor-field-help">Planning and decomposition model.</small>
        </label>

        <label className="pipeline-editor-field">
          <span>General Model</span>
          <Select name="llm_general_credential_id" value={generalModel} onChange={(e) => setGeneralModel(e.target.value)}>
            <SelectOption value="" label="None" />
            {creds.map((item, index) => (
              <SelectOption
                key={`${item?.credential_id ?? "credential-general"}-${index}`}
                value={item?.credential_id ?? ""}
                label={`${item?.title} · ${item?.credential_id}`}
              />
            ))}
          </Select>
          <small className="pipeline-editor-field-help">Default model for regular project chat requests.</small>
        </label>

        <label className="pipeline-editor-field">
          <span>Max Steps</span>
          <Input
            type="number"
            name="max_steps"
            min={1}
            max={1000}
            value={maxSteps}
            onChange={(e) => setMaxSteps(Number(e.target.value))}
          />
          <small className="pipeline-editor-field-help">Upper bound for future multi-step agent execution.</small>
        </label>

        <label className="pipeline-editor-field">
          <span>Max Replans</span>
          <Input
            type="number"
            name="max_replans"
            min={0}
            max={64}
            value={maxReplans}
            onChange={(e) => setMaxReplans(Number(e.target.value))}
          />
          <small className="pipeline-editor-field-help">Maximum replanning attempts before stopping.</small>
        </label>

        <label className="pipeline-editor-field">
          <span>Chat History Pairs</span>
          <Input
            type="number"
            name="chat_history_pairs"
            min={0}
            max={50}
            value={historyPairs}
            onChange={(e) => setHistoryPairs(Number(e.target.value))}
          />
          <small className="pipeline-editor-field-help">
            Number of previous user/assistant exchanges kept as context (0 = no history).
          </small>
        </label>

        <div className="flex flex-col gap-2 pt-1">
          <Checkbox
            name="enabled"
            label="Enable assistant for this project"
            checked={enabled}
            onChange={(e) => setEnabled(e.target.checked)}
          />
        </div>

        <div className="col-span-full flex items-center gap-[0.7rem]">
          <Button
            type="submit"
            variant="primary"
            size="sm"
            disabled={saving}
            label={saving ? "Saving..." : "Save Assistant Config"}
          />
          <span className={cx("text-[0.72rem]", statusTone === "ok" ? "text-[color-mix(in_srgb,var(--color-accent)_80%,#e6f9ef)]" : statusTone === "error" ? "text-red-300" : "text-body-soft")}>{statusMsg}</span>
        </div>
      </form>
    </article>
  );
}

// ─── Libraries Panel ─────────────────────────────────────────────────────────

function LibrariesPanel({ items, api }) {
  const [libs, setLibs] = useState(Array.isArray(items) ? items : []);
  const [loading, setLoading] = useState(null);
  const [errorMsg, setErrorMsg] = useState(null);

  async function toggle(lib) {
    setLoading(lib.name);
    setErrorMsg(null);
    try {
      if (lib.enabled) {
        await requestJson(`${api}/disable?name=${encodeURIComponent(lib.name)}`, { method: "DELETE" });
      } else {
        await requestJson(`${api}/enable`, {
          method: "POST",
          body: JSON.stringify({
            name: lib.name,
            version: lib.packed_version,
            source: "offline",
          }),
        });
      }
      const updated = await requestJson(api);
      setLibs(Array.isArray(updated) ? updated : libs);
    } catch (err) {
      setErrorMsg(String(err?.message || err));
    } finally {
      setLoading(null);
    }
  }

  return (
    <div className="flex flex-col gap-4">
      {errorMsg ? (
        <p className="text-[0.72rem] text-red-300">{errorMsg}</p>
      ) : null}
      {libs.map((lib) => (
        <article key={lib.name} className="border border-border rounded-xl bg-surface p-[0.85rem] mb-[0.9rem]">
          <header className="flex items-start justify-between gap-3 mb-[0.65rem]">
            <div className="flex flex-col gap-1">
              <div className="flex items-center gap-2">
                <h3 className="project-card-title">{lib.name}</h3>
                <Badge label={lib.packed_version} variant="secondary" />
                <Badge label={lib.packed_kind} variant={lib.packed_kind === "full" ? "default" : "secondary"} />
              </div>
              <p className="project-card-copy">{lib.description}</p>
              {lib.enabled ? (
                <p className="project-card-copy" data-tone="ok">
                  locked: {lib.installed_version} · {lib.source}
                </p>
              ) : null}
            </div>
            <Button
              variant={lib.enabled ? "outline" : "primary"}
              size="sm"
              disabled={loading === lib.name}
              label={loading === lib.name ? "..." : lib.enabled ? "Disable" : "Enable"}
              onClick={() => toggle(lib)}
            />
          </header>
        </article>
      ))}
    </div>
  );
}

// ─── Re-index Panel ─────────────────────────────────────────────────────────

function ReIndexPanel({ api }) {
  const [busy, setBusy] = useState(false);
  const [result, setResult] = useState(null);
  const [error, setError] = useState(null);

  async function handleReindex() {
    setBusy(true);
    setResult(null);
    setError(null);
    try {
      const data = await requestJson(api, { method: "POST" });
      setResult(data);
    } catch (err) {
      setError((err as any)?.message || String(err));
    } finally {
      setBusy(false);
    }
  }

  return (
    <article className="border border-border rounded-xl bg-surface p-[0.85rem] mb-[0.9rem]">
      <header className="flex items-start justify-between gap-3 mb-[0.65rem]">
        <div>
          <h3 className="project-card-title">Re-index from Files</h3>
          <p className="project-card-copy">
            Scan <code>repo/pipelines/</code> on disk and register all found files into the catalog.
            Use this after a DB wipe, crash recovery, or restoring from a git backup.
          </p>
        </div>
        <span className="project-inline-chip">Recovery</span>
      </header>
      <div className="flex flex-col gap-[0.55rem]">
        <div className="flex items-center gap-[0.7rem]">
          <Button
            variant="outline"
            size="sm"
            disabled={busy}
            label={busy ? "Indexing..." : "Run Re-index"}
            onClick={handleReindex}
          />
          {error ? (
            <span className="text-[0.72rem] text-red-300">{error}</span>
          ) : null}
        </div>
        {result ? (
          <div className="flex items-center gap-[0.55rem] flex-wrap">
            <span className="project-inline-chip">{result.pipelines} pipeline{result.pipelines !== 1 ? "s" : ""}</span>
            <span className="project-inline-chip">{result.templates} template{result.templates !== 1 ? "s" : ""}</span>
            <span className="project-inline-chip">{result.assets} asset{result.assets !== 1 ? "s" : ""}</span>
            {Array.isArray(result.errors) && result.errors.length > 0 ? (
              <span className="text-[0.72rem] text-red-300">{result.errors.length} error{result.errors.length !== 1 ? "s" : ""}</span>
            ) : (
              <span className="text-[0.72rem] text-[color-mix(in_srgb,var(--color-accent)_80%,#e6f9ef)]">Done.</span>
            )}
          </div>
        ) : null}
      </div>
    </article>
  );
}

// ─── Git Identity Panel ──────────────────────────────────────────────────────

function GitPanel({ api, initialConfig }) {
  const [authorName, setAuthorName] = useState(String(initialConfig?.author_name ?? ""));
  const [authorEmail, setAuthorEmail] = useState(String(initialConfig?.author_email ?? ""));
  const [statusMsg, setStatusMsg] = useState("Ready.");
  const [statusTone, setStatusTone] = useState("info");
  const [saving, setSaving] = useState(false);
  const [commitOpen, setCommitOpen] = useState(false);
  const [pendingData, setPendingData] = useState(null);

  function handleSubmit(e) {
    e.preventDefault();
    setPendingData({ author_name: authorName.trim(), author_email: authorEmail.trim() });
    setCommitOpen(true);
  }

  async function handleCommit(commitMessage) {
    setCommitOpen(false);
    setSaving(true);
    setStatusMsg("Saving...");
    setStatusTone("info");
    try {
      const resp = await requestJson(api, {
        method: "PUT",
        body: JSON.stringify({ commit_message: commitMessage, data: pendingData }),
      });
      if (resp?.committed) {
        setStatusMsg("Saved & committed.");
        setStatusTone("ok");
      } else if (resp?.git_error) {
        setStatusMsg(`Saved (git: ${resp.git_error})`);
        setStatusTone("info");
      } else {
        setStatusMsg("Saved.");
        setStatusTone("ok");
      }
    } catch (err) {
      setStatusMsg(`Failed: ${err?.message || String(err)}`);
      setStatusTone("error");
    } finally {
      setSaving(false);
      setPendingData(null);
    }
  }

  return (
    <article className="border border-border rounded-xl bg-surface p-[0.85rem] mb-[0.9rem]">
      <CommitDialog
        open={commitOpen}
        section="git"
        defaultMessage="settings(git): set git identity"
        onConfirm={handleCommit}
        onCancel={() => { setCommitOpen(false); setPendingData(null); }}
      />
      <header className="flex items-start justify-between gap-3 mb-[0.65rem]">
        <div>
          <h3 className="project-card-title">Git Identity</h3>
          <p className="project-card-copy">
            Author name and email used for all git commits in this project.
            Required for commits to succeed.
          </p>
        </div>
        <span className="project-inline-chip">Git</span>
      </header>
      <form className="grid grid-cols-2 gap-[0.65rem]" onSubmit={handleSubmit}>
        <Field label="Author Name">
          <Input
            name="author_name"
            placeholder="Your Name"
            value={authorName}
            onInput={(e) => setAuthorName(e.currentTarget.value)}
          />
        </Field>
        <Field label="Author Email">
          <Input
            name="author_email"
            placeholder="you@example.com"
            value={authorEmail}
            onInput={(e) => setAuthorEmail(e.currentTarget.value)}
          />
        </Field>
        <div className="col-span-full flex items-center gap-[0.7rem]">
          <Button
            type="submit"
            variant="primary"
            size="sm"
            disabled={saving}
            label={saving ? "Saving..." : "Save Git Identity"}
          />
          <span className={cx("text-[0.72rem]", statusTone === "ok" ? "text-[color-mix(in_srgb,var(--color-accent)_80%,#e6f9ef)]" : statusTone === "error" ? "text-red-300" : "text-body-soft")}>{statusMsg}</span>
        </div>
      </form>
    </article>
  );
}

// ─── Node Registry Panel ─────────────────────────────────────────────────────

function NodeRegistryPanel({ groups, count }) {
  const [searchQuery, setSearchQuery] = useState("");
  const [activeTab, setActiveTab] = useState("installed");

  const nodeGroups = Array.isArray(groups) ? groups : [];
  const query = searchQuery.toLowerCase().trim();

  const filteredGroups = query
    ? nodeGroups
        .map((group) => ({
          ...group,
          nodes: (Array.isArray(group?.nodes) ? group.nodes : []).filter((node) =>
            `${node?.title ?? ""} ${node?.kind ?? ""} ${node?.description ?? ""}`
              .toLowerCase()
              .includes(query)
          ),
        }))
        .filter((group) => group.nodes.length > 0)
    : nodeGroups;

  const visibleCount = filteredGroups.reduce(
    (sum, g) => sum + (g.nodes?.length ?? 0),
    0
  );

  return (
    <div className="border border-border rounded-xl bg-surface overflow-hidden">
      {/* Toolbar */}
      <div className="flex flex-row items-center gap-[0.55rem] px-3 pt-[0.7rem] pb-[0.6rem]">
        <Input
          placeholder="Search nodes by name or kind..."
          value={searchQuery}
          onInput={(e) => setSearchQuery(e.currentTarget.value)}
        />
        <Button variant="outline" size="sm" label="+ Install" disabled />
      </div>

      {/* Tabs */}
      <div className="flex gap-1 px-3 pb-2">
        {(["installed", "discover", "updates"] as const).map((tab) => (
          <Button
            key={tab}
            variant="ghost"
            size="sm"
            className={cx(tab === activeTab && "bg-[color-mix(in_srgb,var(--color-accent)_14%,transparent)] text-accent border-[color-mix(in_srgb,var(--color-accent)_40%,transparent)]")}
            label={tab === "installed" ? `Installed · ${count}` : tab === "discover" ? "Discover" : "Updates"}
            onClick={() => setActiveTab(tab)}
          />
        ))}
      </div>

      <Separator />

      {/* Installed panel */}
      {activeTab === "installed" ? (
        <div>
          <p className="px-3 py-[0.4rem] text-[0.72rem] text-body-soft border-b border-border-soft">
            {visibleCount === count
              ? `${count} nodes · ${count} built-in`
              : `${visibleCount} of ${count} nodes · ${count} built-in`}
          </p>
          <div className="flex flex-col gap-[0.35rem] px-3 py-[0.6rem]">
            {filteredGroups.length === 0 ? (
              <p className="p-8 text-center text-[0.8rem] text-body-soft">No nodes found.</p>
            ) : (
              filteredGroups.map((group, gi) => (
                <div key={`grp-${gi}`}>
                  <div className="flex items-center gap-2 mb-[0.35rem] mt-2 first:mt-0">
                    {group?.prefix ? (
                      <span className="text-[0.65rem] font-mono text-body-soft tracking-[0.05em] whitespace-nowrap shrink-0">{group.prefix}</span>
                    ) : null}
                    <div className="flex-1 h-px bg-border-soft" />
                  </div>
                  {(Array.isArray(group?.nodes) ? group.nodes : []).map((node, ni) => (
                    <div
                      key={`${node?.kind ?? "node"}-${ni}`}
                      className="flex items-stretch border border-border-soft rounded-lg bg-surface-2 overflow-hidden transition-colors duration-[120ms]"
                    >
                      <div className="w-[3px] shrink-0 bg-border" />
                      <div className="flex-1 min-w-0 px-3 py-[0.6rem] flex items-start justify-between gap-3">
                        <div className="flex-1 min-w-0">
                          <div className="text-[0.83rem] font-bold text-body leading-tight">{node?.title}</div>
                          <div className="text-[0.66rem] font-mono text-body-soft mt-[0.15rem] tracking-[0.03em]">{node?.kind}</div>
                          <div className="text-[0.75rem] leading-[1.4] text-body-soft mt-[0.3rem]">{node?.description}</div>
                        </div>
                        <div className="flex items-center flex-wrap gap-[0.3rem] shrink-0 pt-[0.1rem]">
                          {node?.script_available ? (
                            <Badge label="n.script access" variant="outline" className="text-[0.65rem] text-gray-200 border-white/25 bg-transparent" />
                          ) : null}
                          {node?.ai_registered ? (
                            <Badge label="agent tool" variant="outline" className="text-[0.65rem] text-gray-200 border-white/25 bg-transparent" />
                          ) : null}
                          <span className="inline-flex items-center px-2 py-[0.25rem] rounded-full border border-[rgba(74,222,128,0.3)] text-[#4ade80] text-[0.65rem] font-mono uppercase tracking-widest">● installed</span>
                        </div>
                      </div>
                    </div>
                  ))}
                </div>
              ))
            )}
          </div>
        </div>
      ) : null}

      {/* Discover panel */}
      {activeTab === "discover" ? (
        <div className="p-4 px-3">
          <Card>
            <CardContent>
              <CardDescription label="Community registry — coming soon." />
              <CardDescription label="Install custom nodes from a Git URL using + Install above." />
            </CardContent>
          </Card>
        </div>
      ) : null}

      {/* Updates panel */}
      {activeTab === "updates" ? (
        <div className="p-4 px-3">
          <Card>
            <CardContent>
              <CardDescription label={`All ${count} built-in nodes are current.`} />
            </CardContent>
          </Card>
        </div>
      ) : null}
    </div>
  );
}

// ─── Logging Settings Panel ─────────────────────────────────────────────────

function LoggingPanel({ api, initialConfig }) {
  const [maxInv, setMaxInv] = useState(String(initialConfig?.max_invocations ?? 20));
  const [statusMsg, setStatusMsg] = useState("Ready.");
  const [statusTone, setStatusTone] = useState("info");
  const [saving, setSaving] = useState(false);
  const [commitOpen, setCommitOpen] = useState(false);
  const [pendingData, setPendingData] = useState(null);

  function handleSubmit(e) {
    e.preventDefault();
    setPendingData({ max_invocations: parseInt(maxInv, 10) || 20 });
    setCommitOpen(true);
  }

  async function handleCommit(commitMessage) {
    setCommitOpen(false);
    setSaving(true);
    setStatusMsg("Saving...");
    setStatusTone("info");
    try {
      const resp = await requestJson(api, {
        method: "PUT",
        body: JSON.stringify({ commit_message: commitMessage, data: pendingData }),
      });
      if (resp?.committed) {
        setStatusMsg("Saved & committed.");
        setStatusTone("ok");
      } else if (resp?.git_error) {
        setStatusMsg(`Saved (git: ${resp.git_error})`);
        setStatusTone("info");
      } else {
        setStatusMsg("Saved.");
        setStatusTone("ok");
      }
    } catch (err) {
      setStatusMsg(`Failed: ${err?.message || String(err)}`);
      setStatusTone("error");
    } finally {
      setSaving(false);
      setPendingData(null);
    }
  }

  return (
    <article className="border border-border rounded-xl bg-surface p-[0.85rem] mb-[0.9rem]">
      <CommitDialog
        open={commitOpen}
        section="logging"
        defaultMessage="settings(logging): update retention config"
        onConfirm={handleCommit}
        onCancel={() => { setCommitOpen(false); setPendingData(null); }}
      />
      <header className="flex items-start justify-between gap-3 mb-[0.65rem]">
        <div>
          <h3 className="project-card-title">Pipeline Logging</h3>
          <p className="project-card-copy">Retention settings for pipeline invocation logs.</p>
        </div>
        <span className="project-inline-chip">Logging</span>
      </header>
      <form className="grid grid-cols-2 gap-[0.65rem]" onSubmit={handleSubmit}>
        <Field label="Max Invocations Per Pipeline">
          <Input
            name="max_invocations"
            type="number"
            min="1"
            max="1000"
            value={maxInv}
            onInput={(e) => setMaxInv(e.currentTarget.value)}
          />
          <small className="pipeline-editor-field-help">
            How many invocation log entries to retain per pipeline. Oldest are dropped. Default: 20.
          </small>
        </Field>
        <div className="col-span-full flex items-center gap-[0.7rem]">
          <Button
            type="submit"
            variant="primary"
            size="sm"
            disabled={saving}
            label={saving ? "Saving..." : "Save Logging Config"}
          />
          <span className={cx("text-[0.72rem]", statusTone === "ok" ? "text-[color-mix(in_srgb,var(--color-accent)_80%,#e6f9ef)]" : statusTone === "error" ? "text-red-300" : "text-body-soft")}>{statusMsg}</span>
        </div>
      </form>
    </article>
  );
}

// ─── Runtime Defaults Panel ──────────────────────────────────────────────────

function RuntimeDefaultsPanel({ api, initialConfig }) {
  const [maxMb, setMaxMb] = useState(Number(initialConfig?.max_asset_size_mb ?? 10));
  const [statusMsg, setStatusMsg] = useState("Ready.");
  const [statusTone, setStatusTone] = useState("info");
  const [saving, setSaving] = useState(false);
  const [commitOpen, setCommitOpen] = useState(false);
  const [pendingData, setPendingData] = useState(null);

  function handleSubmit(e) {
    e.preventDefault();
    setPendingData({ max_asset_size_mb: maxMb });
    setCommitOpen(true);
  }

  async function handleCommit(commitMessage) {
    setCommitOpen(false);
    setSaving(true);
    setStatusMsg("Saving...");
    setStatusTone("info");
    try {
      const resp = await requestJson(api, {
        method: "PUT",
        body: JSON.stringify({ commit_message: commitMessage, data: pendingData }),
      });
      if (resp?.committed) {
        setStatusMsg("Saved & committed.");
        setStatusTone("ok");
      } else if (resp?.git_error) {
        setStatusMsg(`Saved (git: ${resp.git_error})`);
        setStatusTone("info");
      } else {
        setStatusMsg("Saved.");
        setStatusTone("ok");
      }
    } catch (err) {
      setStatusMsg(`Failed: ${err?.message || String(err)}`);
      setStatusTone("error");
    } finally {
      setSaving(false);
      setPendingData(null);
    }
  }

  return (
    <article className="border border-border rounded-xl bg-surface p-[0.85rem] mb-[0.9rem]">
      <CommitDialog
        open={commitOpen}
        section="assets"
        defaultMessage="settings(assets): update runtime defaults"
        onConfirm={handleCommit}
        onCancel={() => { setCommitOpen(false); setPendingData(null); }}
      />
      <header className="flex items-start justify-between gap-3 mb-[0.65rem]">
        <div>
          <h3 className="project-card-title">Runtime Defaults</h3>
          <p className="project-card-copy">Upload size and asset serving limits for this project.</p>
        </div>
        <span className="project-inline-chip">Assets</span>
      </header>
      <form className="flex flex-col gap-[0.65rem]" onSubmit={handleSubmit}>
        <label className="pipeline-editor-field">
          <span>Max asset upload size: <strong>{maxMb} MB</strong></span>
          <input
            type="range"
            name="max_asset_size_mb"
            min={5}
            max={50}
            step={1}
            value={maxMb}
            onInput={(e) => setMaxMb(Number((e.target as HTMLInputElement).value))}
            className="w-full accent-[var(--color-accent)] cursor-pointer"
          />
          <small className="pipeline-editor-field-help">
            Maximum file size per uploaded asset (5–50 MB). Aligns with git hosting soft-warning threshold.
          </small>
        </label>
        <div className="flex items-center gap-[0.7rem]">
          <Button
            type="submit"
            variant="primary"
            size="sm"
            disabled={saving}
            label={saving ? "Saving..." : "Save Defaults"}
          />
          <span className={cx("text-[0.72rem]", statusTone === "ok" ? "text-[color-mix(in_srgb,var(--color-accent)_80%,#e6f9ef)]" : statusTone === "error" ? "text-red-300" : "text-body-soft")}>{statusMsg}</span>
        </div>
      </form>
    </article>
  );
}

// ─── Files ────────────────────────────────────────────────────────────────────

function FilesPanel() {
  return (
    <div className="project-settings-panel">
      <div className="project-settings-panel-head">
        <p className="project-card-label">Object Storage</p>
        <Badge variant="outline">Coming soon</Badge>
      </div>
      <div className="project-settings-panel-body flex flex-col gap-6 pt-2">

        {/* S3 */}
        <Card className="opacity-60">
          <CardContent className="flex items-start gap-4 pt-5">
            <div className="mt-0.5 rounded bg-[color-mix(in_srgb,var(--color-accent)_12%,transparent)] p-2 text-accent">
              <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
                <path d="M21 8V16a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8"/>
                <path d="M3 8l9-5 9 5"/>
                <path d="M12 3v18"/>
              </svg>
            </div>
            <div className="flex-1">
              <p className="text-[0.88rem] font-semibold text-body">Amazon S3 / S3-Compatible</p>
              <p className="mt-0.5 text-[0.78rem] text-body-soft">
                Connect an S3 bucket (or any compatible store — Cloudflare R2, MinIO, Backblaze B2) as the
                primary file backend for this project. Uploaded assets will be stored in the bucket and served
                via pre-signed or public URLs.
              </p>
              <div className="mt-3 flex flex-wrap gap-2">
                {["AWS S3", "Cloudflare R2", "MinIO", "Backblaze B2", "Tigris"].map((label) => (
                  <Badge key={label} variant="secondary" className="text-[0.72rem]">{label}</Badge>
                ))}
              </div>
            </div>
          </CardContent>
        </Card>

        <p className="text-[0.78rem] text-body-soft">
          File storage integration is on the roadmap. When available, you'll be able to bind a credential
          profile here and choose a bucket. Existing assets in <code className="font-mono text-[0.75rem]">repo/pipelines/assets/</code> will be
          migrated automatically.
        </p>
      </div>
    </div>
  );
}

// ─── Git Branch Panel ─────────────────────────────────────────────────────────

function GitBranchPanel({ owner, project }) {
  const branchesUrl = `/api/projects/${owner}/${project}/git/branches`;
  const [current, setCurrent] = useState("");
  const [branches, setBranches] = useState([]);
  const [selected, setSelected] = useState("");
  const [newBranch, setNewBranch] = useState("");
  const [busy, setBusy] = useState(false);
  const [msg, setMsg] = useState("");
  const [msgTone, setMsgTone] = useState("info");

  async function fetchBranches() {
    try {
      const data = await requestJson(branchesUrl);
      if (!data) return;
      setCurrent(data.current ?? "");
      setBranches(Array.isArray(data.branches) ? data.branches : []);
      setSelected(data.current ?? "");
    } catch (_) {}
  }

  useEffect(() => { fetchBranches(); }, []);

  async function handleSwitch() {
    if (!selected || selected === current) return;
    setBusy(true); setMsg("");
    try {
      await requestJson(branchesUrl, { method: "POST", body: JSON.stringify({ branch: selected, create: false }) });
      setMsg(`Switched to ${selected}.`); setMsgTone("ok");
      await fetchBranches();
    } catch (e) { setMsg((e as any)?.message || "Failed"); setMsgTone("error"); }
    setBusy(false);
  }

  async function handleCreate() {
    const name = newBranch.trim();
    if (!name) return;
    setBusy(true); setMsg("");
    try {
      await requestJson(branchesUrl, { method: "POST", body: JSON.stringify({ branch: name, create: true }) });
      setMsg(`Branch "${name}" created and checked out.`); setMsgTone("ok");
      setNewBranch("");
      await fetchBranches();
    } catch (e) { setMsg((e as any)?.message || "Failed"); setMsgTone("error"); }
    setBusy(false);
  }

  return (
    <article id="git" className="border border-border rounded-xl bg-surface p-[0.85rem] mb-[0.9rem]">
      <header className="flex items-start justify-between gap-3 mb-[0.65rem]">
        <div>
          <h3 className="project-card-title">Git Branches</h3>
          <p className="project-card-copy">Switch or create local branches for this project.</p>
        </div>
        {current && (
          <span className="font-mono text-[0.65rem] bg-surface-3 border border-border px-1.5 py-0.5 rounded text-body-soft">
            {current}
          </span>
        )}
      </header>

      {branches.length > 1 && (
        <div className="flex items-end gap-[0.65rem] mb-[0.65rem]">
          <Field label="Switch to branch" className="flex-1">
            <Select value={selected} onChange={(e) => setSelected(e.target.value)}>
              {branches.map((b) => <SelectOption key={b} value={b}>{b}</SelectOption>)}
            </Select>
          </Field>
          <Button size="sm" variant="outline" onClick={handleSwitch} disabled={busy || selected === current}>
            Switch
          </Button>
        </div>
      )}

      <div className="flex items-end gap-[0.65rem]">
        <Field label="Create new branch" className="flex-1">
          <Input
            placeholder="branch-name"
            value={newBranch}
            onInput={(e) => setNewBranch(e.currentTarget.value)}
          />
        </Field>
        <Button size="sm" variant="primary" onClick={handleCreate} disabled={busy || !newBranch.trim()}>
          Create
        </Button>
      </div>

      {msg && (
        <p className={cx("text-[0.72rem] mt-[0.5rem]", msgTone === "ok" ? "text-accent" : msgTone === "error" ? "text-red-400" : "text-body-soft")}>
          {msg}
        </p>
      )}
    </article>
  );
}

// ─── Danger Zone ─────────────────────────────────────────────────────────────

function DangerZone({ owner, project }) {
  const [confirmName, setConfirmName] = useState("");
  const [password, setPassword] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");

  const canDelete = confirmName.trim() === project && password.length > 0;

  async function handleDelete(e) {
    e.preventDefault();
    if (!canDelete || busy) return;
    setBusy(true);
    setError("");
    try {
      const res = await requestJson(`/api/users/${owner}/projects/${project}`, {
        method: "DELETE",
        body: JSON.stringify({ project_name: confirmName.trim(), password }),
      });
      if (res?.ok) {
        window.location.href = "/home";
      }
    } catch (e) {
      setError((e as any)?.message || "Deletion failed");
      setBusy(false);
    }
  }

  return (
    <article className="border border-red-500/40 rounded-xl bg-surface p-[0.85rem] mt-[0.9rem]">
      <header className="flex items-start justify-between gap-3 mb-[0.65rem]">
        <div>
          <h3 className="project-card-title text-red-400">Danger Zone</h3>
          <p className="project-card-copy">
            Permanently delete this project and all its data. This cannot be undone.
          </p>
        </div>
      </header>
      <form onSubmit={handleDelete} className="flex flex-col gap-3 max-w-sm">
        <Field label={`Type "${project}" to confirm`} id="danger-confirm-name">
          <Input
            id="danger-confirm-name"
            type="text"
            value={confirmName}
            onInput={(e) => setConfirmName((e.target as HTMLInputElement).value)}
            placeholder={project}
            autoComplete="off"
          />
        </Field>
        <Field label="Your password" id="danger-password">
          <Input
            id="danger-password"
            type="password"
            value={password}
            onInput={(e) => setPassword((e.target as HTMLInputElement).value)}
            placeholder="Enter your password"
            autoComplete="current-password"
          />
        </Field>
        {error && <p className="text-[0.72rem] text-red-400">{error}</p>}
        <Button
          type="submit"
          variant="destructive"
          disabled={!canDelete || busy}
          className="self-start"
        >
          {busy ? "Deleting…" : "Delete project"}
        </Button>
      </form>
    </article>
  );
}

// ─── Page ────────────────────────────────────────────────────────────────────

export default function Page(input) {
  const tabFlags = input?.tab_flags ?? {};
  const settingsTabs = Array.isArray(input?.settings_tabs) ? input.settings_tabs : [];
  const assistant = input?.assistant ?? {};
  const mcpCapabilities = Array.isArray(input?.mcp?.capabilities) ? input.mcp.capabilities : [];

  return (
    <>
      <ProjectStudioShell
        projectHref={input.project_href}
        projectLabel={input.title}
        currentMenu="Settings"
        owner={input.owner}
        project={input.project}
        nav={input.nav}
      >
        <div className="flex-1 min-h-0 flex flex-col overflow-hidden">
          <StudioTabNav>
            {settingsTabs.map((item, index) => (
              <StudioTabLink
                key={`${item?.href ?? "tab"}-${index}`}
                href={item?.href ?? "#"}
                active={item?.classes === "is-active"}
              >
                {item?.label}
              </StudioTabLink>
            ))}
          </StudioTabNav>

          <section className="flex-1 min-h-0 overflow-auto flex flex-col bg-bg">
            <div className="project-content-wrap">
              <section className="project-content-section">
                <div className="project-content-head">
                  <div>
                    <p className="project-content-title">{input.page_title}</p>
                    <p className="project-content-copy">{input.page_subtitle}</p>
                  </div>
                </div>
              </section>

              {tabFlags?.general ? (
                <section className="project-content-section">
                  <div className="project-content-body">
                    <div className="project-card-grid cols-2">{renderCardGrid(input?.cards_general)}</div>
                    <RuntimeDefaultsPanel
                      api={input?.assets?.settings_api ?? ""}
                      initialConfig={input?.assets?.config ?? {}}
                    />
                    <ReIndexPanel api={input?.reindex_api ?? ""} />
                    <GitPanel
                      api={input?.git?.api ?? ""}
                      initialConfig={input?.git?.config ?? {}}
                    />
                    <GitBranchPanel owner={input.owner} project={input.project} />
                    <DangerZone owner={input.owner} project={input.project} />
                  </div>
                </section>
              ) : null}

              {tabFlags?.policy ? (
                <section className="project-content-section">
                  <div className="project-content-body">
                    <RwePanel
                      api={input?.rwe?.api ?? ""}
                      initialConfig={input?.rwe?.config ?? {}}
                      owner={input?.owner}
                      project={input?.project}
                    />
                    <Separator className="my-6" />
                    <LoggingPanel
                      api={input?.logging?.api ?? ""}
                      initialConfig={input?.logging?.config ?? {}}
                    />
                    <div className="project-card-grid cols-2">
                      {renderCardGrid(input?.cards_policy)}
                    </div>
                  </div>
                </section>
              ) : null}

              {tabFlags?.automatons ? (
                <section className="project-content-section">
                  <div className="project-content-body">
                    <AssistantPanel
                      api={assistant?.api?.config ?? ""}
                      credentials={Array.isArray(assistant?.credentials) ? assistant.credentials : []}
                      initialConfig={assistant?.config ?? {}}
                    />

                    <article className="border border-border rounded-xl bg-surface p-[0.85rem] mb-[0.9rem]">
                      <header className="flex items-start justify-between gap-3 mb-[0.65rem]">
                        <div>
                          <h3 className="project-card-title">MCP Session</h3>
                          <p className="project-card-copy">Remote control channel for external agents.</p>
                        </div>
                        <span className="project-inline-chip">{input?.mcp?.status_label}</span>
                      </header>
                      <div className="flex flex-wrap items-center gap-[0.45rem]">
                        <p className="project-card-copy">Allowed capabilities:</p>
                        {mcpCapabilities.map((item, index) => (
                          <span key={`${item}-${index}`} className="project-inline-chip">{item}</span>
                        ))}
                      </div>
                    </article>
                  </div>
                </section>
              ) : null}

              {tabFlags?.libraries ? (
                <section className="project-content-section">
                  <div className="project-content-body">
                    <LibrariesPanel
                      items={Array.isArray(input?.libraries_available) ? input.libraries_available : []}
                      api={input?.libraries_api ?? ""}
                    />
                  </div>
                </section>
              ) : null}

              {tabFlags?.nodes ? (
                <section className="project-content-section">
                  <div className="project-content-body">
                    <NodeRegistryPanel
                      groups={input?.node_groups ?? []}
                      count={input?.node_count ?? 0}
                    />
                  </div>
                </section>
              ) : null}

              {tabFlags?.files ? (
                <section className="project-content-section">
                  <div className="project-content-body">
                    <FilesPanel />
                  </div>
                </section>
              ) : null}
            </div>
          </section>
        </div>
      </ProjectStudioShell>
    </>
  );
}
