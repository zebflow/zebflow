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
      backHref="{input.nav.links.tables_connections}"
      backLabel="{input.title}"
      title="Tables"
      meta="{input.connection}"
      actionClass="hidden"
      chatClass=""
    >
      <div className="text-xs text-slate-500">Read-only preview for available tables in this connection.</div>

      <section className="bg-white border border-slate-200 rounded-md overflow-hidden">
        <table className="w-full text-left">
          <thead className="bg-slate-100 border-b border-slate-200">
            <tr>
              <th className="px-3 py-2.5 text-[11px] font-mono uppercase tracking-[0.14em] text-slate-600">Table</th>
              <th className="px-3 py-2.5 text-[11px] font-mono uppercase tracking-[0.14em] text-slate-600">Rows</th>
              <th className="px-3 py-2.5 text-[11px] font-mono uppercase tracking-[0.14em] text-slate-600">Updated</th>
            </tr>
          </thead>
          <tbody>
            <tr jFor="item in input.tables" className="border-b border-slate-100">
              <td className="px-3 py-2.5 text-sm font-semibold text-slate-900">{item.name}</td>
              <td className="px-3 py-2.5 text-sm text-slate-700">{item.rows}</td>
              <td className="px-3 py-2.5 text-sm text-slate-500">{item.updated}</td>
            </tr>
          </tbody>
        </table>
      </section>
    </AdminWrapper>
</Page>
  );
}
