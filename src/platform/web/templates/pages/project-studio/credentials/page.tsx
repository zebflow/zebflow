import { useState, useEffect } from "zeb/react";
import { requestJson } from "@/components/lib/http";
import Button from "@/components/ui/button";
import ProjectStudioShell from "@/pages/project-studio/components/shell";
import { StudioTabNav, StudioTabLink } from "@/components/ui/studio-tab-nav";
import CredentialDialog from "@/pages/project-studio/credentials/components/credential-dialog";
import CredentialTable from "@/pages/project-studio/credentials/components/credential-table";

export const page = {
  html: { lang: "en" },
  body: { className: "font-sans" },
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
  const apiList = input?.credentials?.api?.list ?? "";
  const apiItemBase = input?.credentials?.api?.item_base ?? "";

  const [items, setItems] = useState<any[]>([]);
  const [credentialTypes, setCredentialTypes] = useState<any[]>([]);
  const [dialogOpen, setDialogOpen] = useState(false);
  const [dialogMode, setDialogMode] = useState<"create" | "edit">("create");
  const [editItem, setEditItem] = useState<any>(null);

  async function loadList() {
    try {
      const payload = await requestJson(apiList);
      setItems(Array.isArray(payload?.items) ? payload.items : []);
    } catch {}
  }

  useEffect(() => {
    loadList();
    // Fetch credential type definitions from API
    requestJson(`/api/projects/${input.owner}/${input.project}/credential-types`)
      .then((data) => { if (data?.types) setCredentialTypes(data.types); })
      .catch(() => {});
    // Handle OAuth callback redirect params
    const params = new URLSearchParams(window.location.search);
    const oauthResult = params.get("oauth");
    if (oauthResult) {
      // Clean URL
      const url = new URL(window.location.href);
      url.searchParams.delete("oauth");
      window.history.replaceState({}, "", url.pathname + url.search);
    }
  }, []);

  function openCreate() {
    setDialogMode("create"); setEditItem(null); setDialogOpen(true);
  }
  function openEdit(item: any) {
    setDialogMode("edit"); setEditItem(item); setDialogOpen(true);
  }

  return (
    <ProjectStudioShell
      projectHref={input.project_href}
      projectLabel={input.title}
      currentMenu="Credentials"
      owner={input.owner}
      project={input.project}
      nav={input.nav}
    >
      <div className="flex-1 min-h-0 flex flex-col overflow-hidden">
        <StudioTabNav>
          <StudioTabLink active>Credentials</StudioTabLink>
        </StudioTabNav>
        <section className="flex-1 min-h-0 overflow-auto flex flex-col bg-background">
          <div className="project-content-wrap">
            <section className="project-content-section">
              <div className="project-content-head">
                <div>
                  <p className="project-content-title">Credentials</p>
                  <p className="project-content-copy">Create and manage project credential records used by database and service nodes.</p>
                </div>
                <Button size="sm" onClick={openCreate}>+ New Credential</Button>
              </div>
            </section>

            <section className="project-content-section">
              <div className="project-content-body">
                <CredentialTable items={items} onEdit={openEdit} />
              </div>
            </section>
          </div>
        </section>
      </div>

      <CredentialDialog
        open={dialogOpen}
        onClose={() => setDialogOpen(false)}
        mode={dialogMode}
        editItem={editItem}
        apiList={apiList}
        apiItemBase={apiItemBase}
        onSaved={loadList}
        credentialTypes={credentialTypes}
      />
    </ProjectStudioShell>
  );
}
