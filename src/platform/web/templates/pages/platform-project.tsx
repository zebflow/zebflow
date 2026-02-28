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
      backLabel="Back to Home"
      title="{input.title}"
      meta="owner: {input.owner} | project: {input.project}"
      actionClass="hidden"
      chatClass=""
    >
      <section className="bg-white border border-slate-200 rounded-2xl p-8 shadow-sm">
        <h1 className="text-4xl font-black tracking-tight text-slate-900">{input.title}</h1>
        <p className="mt-3 text-sm text-slate-500">owner: {input.owner} | project: {input.project}</p>

        <div className="mt-8 grid lg:grid-cols-2 gap-5">
          <article className="border border-slate-200 rounded-xl p-5 bg-slate-50">
            <h2 className="text-lg font-bold uppercase tracking-tight">Project Shell Active</h2>
            <p className="text-sm text-slate-600 mt-2">
              This page is rendered through RWE and uses the same visual direction as Zebflow showcase.
            </p>
          </article>
          <article className="border border-slate-200 rounded-xl p-5 bg-slate-50">
            <h2 className="text-lg font-bold uppercase tracking-tight">Studio Ready</h2>
            <p className="text-sm text-slate-600 mt-2">
              Project authoring now groups templates, assets, and schema under one studio workspace.
            </p>
          </article>
        </div>
      </section>
    </AdminWrapper>
</Page>
  );
}
