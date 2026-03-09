import { usePageState } from 'rwe';

export const page = {
  head: {
    title: "List Hydration",
  },
  body: {
    className: "min-h-screen bg-zinc-950 text-zinc-100 antialiased",
  },
  navigation: "history",
};

export const app = (() => {
return {
    state: {
      client: {
        counter: 0
      }
    },
    actions: {
      "counter.inc": (ctx) => {
        const current = Number(ctx.get("client.counter") || 0);
        ctx.set("client.counter", current + 1);
        return "client.counter";
      }
    }
  };
})();

export default function Page(input) {
  const state = usePageState();
  return (
<Page>
    <main className="mx-auto max-w-3xl px-6 py-10 space-y-6">
      <section className="rounded-2xl border border-zinc-800 bg-zinc-900 p-6 shadow-2xl">
        <h1 className="text-3xl font-bold tracking-tight">Keyed List + Hydration Islands</h1>
        <p className="mt-2 text-sm text-zinc-400">
          SSR gives immediate content, client hydration wakes only where needed.
        </p>
      </section>

      <section className="rounded-2xl border border-zinc-800 bg-zinc-900 p-6 shadow-2xl">
        <h2 className="text-sm uppercase tracking-wide text-zinc-400">SSR Keyed List</h2>
        <ul className="mt-4 space-y-2">
          {(input.items || []).map((item) => (
            <li key={item.id} className="rounded-lg border border-zinc-800 bg-zinc-950 px-4 py-3 text-sm">
              {item.title} (#{item.id})
            </li>
          ))}
        </ul>
      </section>

      <section className="grid grid-cols-1 md:grid-cols-2 gap-4">
        <section hydrate="off" className="rounded-2xl border border-zinc-800 bg-zinc-900 p-6 space-y-3">
          <h2 className="text-sm uppercase tracking-wide text-zinc-400">Hydrate Off</h2>
          <p className="text-sm text-zinc-400">No client event wiring in this island.</p>
          <button className="inline-flex items-center rounded-lg bg-cyan-500 px-4 py-2 text-sm font-semibold text-zinc-950 hover:bg-cyan-400 transition-colors" onClick="counter.inc">
            This island is off by default
          </button>
        </section>

        <section hydrate="interaction" className="rounded-2xl border border-zinc-800 bg-zinc-900 p-6 space-y-3">
          <h2 className="text-sm uppercase tracking-wide text-zinc-400">Hydrate On Interaction</h2>
          <p className="text-sm text-zinc-400">Wakes on pointer/focus/key interaction.</p>
          <button className="inline-flex items-center rounded-lg bg-cyan-500 px-4 py-2 text-sm font-semibold text-zinc-950 hover:bg-cyan-400 transition-colors" onClick="counter.inc">
            Wake and Increment
          </button>
          <p className="text-sm">Counter: <span className="font-semibold text-cyan-400">{state.client?.counter ?? 0}</span></p>
        </section>
      </section>
    </main>
</Page>
  );
}
