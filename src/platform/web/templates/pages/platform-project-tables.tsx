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
      title="Connections"
      meta="{input.project}"
      actionClass="hidden"
      chatClass=""
    >
      <div className="text-xs text-slate-500">Select a connection, then inspect tables like a lightweight DBeaver browser.</div>

      <section className="grid md:grid-cols-2 gap-2">
        <a
          jFor="item in input.connections"
          href="{item.path}"
          className="block bg-white border border-slate-200 rounded-md p-3 hover:border-slate-400 transition-all"
        >
          <div className="flex items-start justify-between gap-3">
            <h3 className="text-sm font-semibold tracking-tight text-slate-900">{item.name}</h3>
            <p className="text-[10px] font-mono uppercase tracking-[0.14em] text-slate-500">{item.driver}</p>
          </div>
          <p className="text-sm text-slate-600 mt-2">Open table browser for this connection.</p>
        </a>
      </section>
    </AdminWrapper>
</Page>
  );
}
