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
      actionHref="{input.primary_action.href}"
      actionLabel="{input.primary_action.label}"
      actionClass=""
      chatClass=""
    >
      <div className="text-xs text-slate-500">{input.page_subtitle}</div>

      <article className="bg-white border border-slate-200 rounded-lg p-2">
        <div className="grid grid-cols-3 gap-1.5" tw-variants="text-[#005B9A] bg-sky-50 text-slate-500 hover:text-slate-900 hover:bg-slate-100">
          <a href="{input.nav.links.studio_templates}" className="block px-2 py-1.5 rounded-md text-[11px] font-mono uppercase tracking-[0.14em] text-center {input.nav.classes.studio_templates}">Templates</a>
          <a href="{input.nav.links.studio_assets}" className="block px-2 py-1.5 rounded-md text-[11px] font-mono uppercase tracking-[0.14em] text-center {input.nav.classes.studio_assets}">Assets</a>
          <a href="{input.nav.links.studio_schema}" className="block px-2 py-1.5 rounded-md text-[11px] font-mono uppercase tracking-[0.14em] text-center {input.nav.classes.studio_schema}">Schema</a>
        </div>
      </article>

      <section className="grid lg:grid-cols-2 gap-2">
        <article jFor="item in input.items" className="bg-white border border-slate-200 rounded-md p-3 hover:border-slate-400 transition-all">
          <h3 className="text-sm font-semibold tracking-tight text-slate-900">{item.title}</h3>
          <p className="text-sm text-slate-600 mt-2">{item.description}</p>
        </article>
      </section>
    </AdminWrapper>
</Page>
  );
}
