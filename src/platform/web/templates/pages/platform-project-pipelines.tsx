import AdminWrapper from "@/components/layout/admin-wrapper";

export const page = {
  head: {
    title: "{{input.seo.title}}",
    description: "{{input.seo.description}}",
  },
  html: {
    lang: "en",
  },
  body: {
    className: "min-h-screen bg-zinc-50 text-gray-900 font-sans",
  },
  navigation: "history",
};

export const app = {};

export default function Page(input) {
  return (
<Page>
    <AdminWrapper
      backHref="/home"
      backLabel="{input.title}"
      title="{input.page_title}"
      meta="{input.project}"
      actionHref="{input.nav.links.studio_templates}"
      actionLabel="Open Studio"
      actionClass=""
      chatClass=""
    >
      <div className="text-xs text-slate-500">{input.page_subtitle}</div>

      <article className="bg-white border border-slate-200 rounded-lg p-2">
        <div className="grid grid-cols-2 md:grid-cols-4 gap-1.5" tw-variants="text-[#005B9A] bg-sky-50 text-slate-500 hover:text-slate-900 hover:bg-slate-100">
          <a href="{input.nav.links.pipelines_registry}" className="block px-2 py-1.5 rounded-md text-[11px] font-mono uppercase tracking-[0.14em] text-center {input.nav.classes.pipeline_registry}">Registry</a>
          <a href="{input.nav.links.pipelines_webhooks}" className="block px-2 py-1.5 rounded-md text-[11px] font-mono uppercase tracking-[0.14em] text-center {input.nav.classes.pipeline_webhooks}">Webhooks</a>
          <a href="{input.nav.links.pipelines_schedules}" className="block px-2 py-1.5 rounded-md text-[11px] font-mono uppercase tracking-[0.14em] text-center {input.nav.classes.pipeline_schedules}">Schedules</a>
          <a href="{input.nav.links.pipelines_functions}" className="block px-2 py-1.5 rounded-md text-[11px] font-mono uppercase tracking-[0.14em] text-center {input.nav.classes.pipeline_functions}">Functions</a>
        </div>
      </article>

      <section jShow="input.is_registry" className="space-y-4">
        <article className="bg-white border border-slate-200 rounded-lg p-4">
          <div className="flex flex-wrap items-center gap-2">
            <p className="text-[11px] font-mono uppercase tracking-[0.14em] text-slate-500">Path</p>
            <p className="text-sm text-slate-800">{input.registry.current_path}</p>
          </div>
          <div className="mt-3 flex flex-wrap gap-1.5">
            <a
              jFor="crumb in input.registry.breadcrumbs"
              href="{crumb.path}"
              className="px-2 py-1 rounded-md border border-slate-200 text-[11px] font-mono uppercase tracking-[0.14em] text-slate-600 hover:bg-slate-100"
            >
              {crumb.name}
            </a>
          </div>
        </article>

        <article jShow="input.registry.has_folders" className="bg-white border border-slate-200 rounded-lg p-4">
          <h3 className="text-[11px] font-mono uppercase tracking-[0.14em] text-slate-700">Folders</h3>
          <div className="mt-3 grid md:grid-cols-2 lg:grid-cols-3 gap-2">
            <a
              jFor="folder in input.registry.folders"
              href="{folder.path}"
              className="block border border-slate-200 rounded-md p-3 hover:border-slate-400 hover:bg-slate-50 transition-all"
            >
              <p className="text-sm font-semibold text-slate-900">{folder.name}</p>
            </a>
          </div>
        </article>

        <article jShow="input.registry.has_pipelines" className="bg-white border border-slate-200 rounded-lg p-4">
          <h3 className="text-[11px] font-mono uppercase tracking-[0.14em] text-slate-700">Pipelines</h3>
          <div className="mt-3 grid lg:grid-cols-2 gap-2">
            <article jFor="item in input.registry.pipelines" className="bg-white border border-slate-200 rounded-md p-3 hover:border-slate-400 transition-all">
              <div className="flex items-start justify-between gap-3">
                <h3 className="text-sm font-semibold tracking-tight text-slate-900">{item.title}</h3>
                <p className="text-[10px] font-mono uppercase tracking-[0.14em] text-slate-500">{item.trigger_kind}</p>
              </div>
              <p className="text-[11px] text-slate-500 mt-1">{item.name}</p>
              <p className="text-sm text-slate-600 mt-2">{item.description}</p>
              <p className="text-[10px] font-mono uppercase tracking-[0.14em] text-slate-500 mt-2">{item.file_rel_path}</p>
            </article>
          </div>
        </article>
      </section>

      <section jShow="input.is_non_registry" className="grid lg:grid-cols-2 gap-2">
        <article jFor="item in input.pipeline_items" className="bg-white border border-slate-200 rounded-md p-3 hover:border-slate-400 transition-all">
          <h3 className="text-sm font-semibold tracking-tight text-slate-900">{item.name}</h3>
          <p className="text-sm text-slate-600 mt-2">{item.description}</p>
        </article>
      </section>
    </AdminWrapper>
</Page>
  );
}
