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

export const app = (() => {
return {
  state: {
    pageTitle: "Compile Fixture",
    counter: 0,
    lastToggleAt: 0,
    badge: "warm"
  },
  actions: {
    "card.toggle": ({ get, set }) => {
      const next = (get("state.counter") || 0) + 1;
      set("state.counter", next);
      set("state.lastToggleAt", Date.now());
    }
  },
  memo: {
    pageTitleUpper: ({ state }) => (state.pageTitle || "").toUpperCase(),
    counterLabel: ({ state }) => `count:${state.counter || 0}`
  },
  effect: {
    syncBadge: {
      deps: ["state.counter"],
      run: ({ get, set }) => {
        const count = get("state.counter") || 0;
        set("state.badge", count > 9 ? "hot" : "warm");
      }
    }
  }
};
})();

export default function Page(input) {
  return (
<Page>
    <main className="mx-auto max-w-3xl px-6 py-8">
      <header className="mb-6">
        <h1 className="text-2xl font-black tracking-tight">{input.pageTitle}</h1>
        <p className="text-sm text-gray-600 mt-1">{input.pageSubtitle}</p>
      </header>

      <section className="grid md:grid-cols-2 gap-4">
        <article className="rounded-xl border border-gray-200 p-4">
          <h3 className="text-base font-bold text-gray-900">{input.cards.news.title}</h3>
          <p className="text-sm text-gray-600 mt-2">{input.cards.news.desc}</p>
          <button onClick="card.toggle" className="mt-3 px-3 py-1 rounded bg-gray-900 text-white text-xs font-mono">
            Toggle
          </button>
        </article>
        <article className="rounded-xl border border-gray-200 p-4">
          <h3 className="text-base font-bold text-gray-900">{input.cards.blog.title}</h3>
          <p className="text-sm text-gray-600 mt-2">{input.cards.blog.desc}</p>
          <button onClick="card.toggle" className="mt-3 px-3 py-1 rounded bg-gray-900 text-white text-xs font-mono">
            Toggle
          </button>
        </article>
      </section>
    </main>
</Page>
  );
}
