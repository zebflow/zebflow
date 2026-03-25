import ProjectStudioShell from "@/pages/project-studio/components/shell";
import { StudioTabNav, StudioTabLink } from "@/components/ui/studio-tab-nav";
import Badge from "@/components/ui/badge";
import Card from "@/components/ui/card";
import CardContent from "@/components/ui/card-content";

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


export default function Page(input) {
  const cards = Array.isArray(input?.cards) ? input.cards : [];
  const activeTab = input?.active_tab ?? "files";
  const base = `/projects/${input.owner}/${input.project}/files`;

  return (
<Page>
    <ProjectStudioShell
      projectHref={input.project_href}
      projectLabel={input.title}
      currentMenu={input.current_menu}
      owner={input.owner}
      project={input.project}
      nav={input.nav}
    >
      <div className="flex-1 min-h-0 flex flex-col overflow-hidden">
        <StudioTabNav>
          <StudioTabLink href={base} active={activeTab === "files"}>Files</StudioTabLink>
          <StudioTabLink href={`${base}/s3`} active={activeTab === "s3"}>S3</StudioTabLink>
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

            {activeTab === "files" ? (
              <section className="project-content-section">
                <div className="project-content-body">
                  <div className="project-card-grid cols-2">
                    {cards.map((item, index) => (
                      <a key={`${item?.href ?? "card"}-${index}`} href={item?.href ?? "#"} className="project-card block">
                        <div className="flex items-start justify-between gap-3">
                          <div>
                            <h3 className="project-card-title">{item?.title}</h3>
                            <p className="project-card-copy">{item?.description}</p>
                          </div>
                          {item?.tag ? <span className="project-inline-chip">{item.tag}</span> : null}
                        </div>
                      </a>
                    ))}
                  </div>
                </div>
              </section>
            ) : null}

            {activeTab === "s3" ? (
              <section className="project-content-section">
                <div className="project-content-body">
                  <S3Panel />
                </div>
              </section>
            ) : null}
          </div>
        </section>
      </div>
    </ProjectStudioShell>
</Page>
  );
}

function S3Panel() {
  return (
    <div className="project-settings-panel">
      <div className="project-settings-panel-head">
        <p className="project-card-label">S3 / Object Storage</p>
        <Badge variant="outline">Coming soon</Badge>
      </div>
      <div className="project-settings-panel-body flex flex-col gap-6 pt-2">
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
          Object storage integration is on the roadmap. When available you'll bind a credential profile,
          choose a bucket, and optionally migrate existing assets from{" "}
          <code className="font-mono text-[0.75rem]">repo/pipelines/assets/</code> automatically.
        </p>
      </div>
    </div>
  );
}
