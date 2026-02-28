export const page = {
  head: {
    title: "{{input.seo.title}}",
    description: "{{input.seo.description}}",
  },
  html: {
    lang: "en",
  },
  body: {
    className: "min-h-screen bg-zinc-950 text-zinc-100 antialiased",
  },
  navigation: "history",
};

export const app = (() => {
return {
    state: {
      ui: {
        pledges: 0
      }
    },
    actions: {
      "pledge.add": (ctx) => {
        const current = Number(ctx.get("ui.pledges") || 0);
        ctx.set("ui.pledges", current + 1);
        return "ui.pledges";
      }
    }
  };
})();

export default function Page(input) {
  return (
<Page>
    <main className="mx-auto max-w-5xl px-6 py-10 space-y-8">
      <section className="rounded-3xl border border-emerald-500/40 bg-gradient-to-br from-emerald-500/20 via-green-500/10 to-zinc-900 p-8 shadow-2xl">
        <p className="inline-flex items-center gap-2 rounded-full border border-emerald-400/40 bg-zinc-900 px-3 py-1 text-xs uppercase tracking-wide text-emerald-300">
          <span className="animate-pulse">●</span>
          Nature-first city plan
        </p>
        <h1 className="mt-4 text-4xl font-bold tracking-tight">{input.hero.title}</h1>
        <p className="mt-3 max-w-3xl text-zinc-300 leading-relaxed">{input.hero.subtitle}</p>
      </section>

      <section className="grid grid-cols-1 md:grid-cols-3 gap-4">
        <article className="rounded-2xl border border-zinc-800 bg-zinc-900 p-5 shadow-lg">
          <p className="text-xs uppercase tracking-wide text-zinc-400">Plastic diverted</p>
          <p className="mt-2 text-3xl font-bold text-emerald-300">{input.metrics.plasticKg}</p>
          <p className="text-sm text-zinc-400">kg this month</p>
        </article>
        <article className="rounded-2xl border border-zinc-800 bg-zinc-900 p-5 shadow-lg">
          <p className="text-xs uppercase tracking-wide text-zinc-400">Compost restored</p>
          <p className="mt-2 text-3xl font-bold text-green-300">{input.metrics.compostKg}</p>
          <p className="text-sm text-zinc-400">kg this month</p>
        </article>
        <article className="rounded-2xl border border-zinc-800 bg-zinc-900 p-5 shadow-lg">
          <p className="text-xs uppercase tracking-wide text-zinc-400">Community actions</p>
          <p className="mt-2 text-3xl font-bold text-cyan-300">{input.metrics.actions}</p>
          <p className="text-sm text-zinc-400">recorded this week</p>
        </article>
      </section>

      <section className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <article className="rounded-2xl border border-zinc-800 bg-zinc-900 p-6 shadow-2xl">
          <h2 className="text-xl font-semibold tracking-tight">Recycling Playbook</h2>
          <p className="mt-2 text-sm text-zinc-400">Simple habits with measurable impact.</p>
          <ul className="mt-4 space-y-3">
            <li className="rounded-lg border border-zinc-800 bg-zinc-950 px-4 py-3" zFor="tip in input.recycleTips" zKey="tip.id">
              <p className="font-semibold text-zinc-100">{tip.title}</p>
              <p className="mt-1 text-sm text-zinc-400">{tip.detail}</p>
            </li>
          </ul>
        </article>

        <article hydrate="interaction" className="rounded-2xl border border-zinc-800 bg-zinc-900 p-6 shadow-2xl space-y-3">
          <h2 className="text-xl font-semibold tracking-tight">Take the Weekly Pledge</h2>
          <p className="text-sm text-zinc-400">Hydrates on interaction only.</p>
          <button className="inline-flex items-center rounded-lg bg-emerald-500 px-4 py-2 text-sm font-semibold text-zinc-950 hover:bg-emerald-400 transition-colors" onClick="pledge.add">
            Count my recycling action
          </button>
          <p className="text-sm">
            Live pledges:
            <span className="font-semibold text-emerald-300" zText="ui.pledges">0</span>
          </p>
          <p className="text-sm text-zinc-400">
            Thank you for helping keep waterways and neighborhoods cleaner.
          </p>
        </article>
      </section>
    </main>
</Page>
  );
}
