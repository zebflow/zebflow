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
      actionClass="hidden"
      chatClass=""
    >
      <div className="text-xs text-slate-500">{input.page_subtitle}</div>

      <section className="grid lg:grid-cols-2 gap-2">
        <article jFor="item in input.cards" className="bg-white border border-slate-200 rounded-md p-3 hover:border-slate-400 transition-all">
          <h3 className="text-sm font-semibold tracking-tight text-slate-900">{item.title}</h3>
          <p className="text-sm text-slate-600 mt-2">{item.description}</p>
        </article>
      </section>
    </AdminWrapper>
</Page>
  );
}
