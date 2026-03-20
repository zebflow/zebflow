import { usePageState, useMemo } from "zeb";

const FEATURES = [
  {
    id: "pipelines",
    title: "Pipeline Graph",
    detail: "Visual workflows with trigger, script, DB, and render nodes.",
  },
  {
    id: "templates",
    title: "Reactive Templates",
    detail: "TSX authoring with SSR + hydration over one compile/render contract.",
  },
  {
    id: "ops",
    title: "Ops & Observability",
    detail: "Runtime trace, diagnostics, and deterministic output artifacts.",
  },
  {
    id: "assistants",
    title: "Project Assistant",
    detail: "Two-model strategy for planning + tool execution under policy.",
  },
];

export default function RweFrontpage(input) {
  const pricing = input?.pricing || { monthly: 39, yearly: 390 };
  const state = usePageState({ yearly: false });
  const yearly = state?.yearly ?? false;
  const setPageState = state.setPageState;

  const finalPrice = useMemo(() => {
    return yearly ? pricing.yearly : pricing.monthly;
  }, [yearly]);

  return (
    <div className="min-h-screen bg-gray-950 px-6 py-10 font-mono text-gray-100">
      <div className="mx-auto max-w-7xl space-y-6">
        <section className="rounded-2xl border border-gray-800 bg-gradient-to-br from-gray-900 via-gray-900 to-indigo-950 p-8 shadow-2xl">
          <p className="text-xs uppercase tracking-[0.22em] text-indigo-300">rwe</p>
          <h1 className="mt-3 max-w-3xl text-4xl font-black leading-tight text-indigo-200">
            Build responsive product surfaces with one TSX pipeline.
          </h1>
          <p className="mt-3 max-w-2xl text-sm text-gray-300">
            Complex landing-page example using sections, stateful pricing switch, and dense visual structure.
          </p>
          <div className="mt-6 flex flex-wrap gap-3">
            <button className="rounded-md border border-indigo-500 bg-indigo-600/30 px-4 py-2 text-sm font-semibold text-indigo-100 hover:bg-indigo-600/50">
              Start Project
            </button>
            <button className="rounded-md border border-gray-700 bg-gray-900 px-4 py-2 text-sm font-semibold text-gray-300 hover:text-white">
              View Docs
            </button>
          </div>
        </section>

        <section className="grid gap-4 md:grid-cols-2 xl:grid-cols-4">
          {FEATURES.map((feature) => (
            <article key={feature.id} className="rounded-xl border border-gray-800 bg-black/30 p-4">
              <h2 className="text-sm font-semibold text-indigo-300">{feature.title}</h2>
              <p className="mt-2 text-xs leading-relaxed text-gray-400">{feature.detail}</p>
            </article>
          ))}
        </section>

        <section className="grid gap-4 lg:grid-cols-[1.5fr_1fr]">
          <article className="rounded-xl border border-gray-800 bg-black/30 p-5">
            <h2 className="text-sm font-semibold text-indigo-300">Release Notes</h2>
            <div className="mt-3 space-y-3 text-xs">
              <p className="rounded border border-gray-800 bg-gray-900/80 px-3 py-2 text-gray-300">[2026-03-04] unified rwe engine + deno worker protocol</p>
              <p className="rounded border border-gray-800 bg-gray-900/80 px-3 py-2 text-gray-300">[2026-03-03] project-wide script artifact hashing for deterministic payloads</p>
              <p className="rounded border border-gray-800 bg-gray-900/80 px-3 py-2 text-gray-300">[2026-03-01] registry API scope separation path/project</p>
            </div>
          </article>

          <article className="rounded-xl border border-gray-800 bg-black/30 p-5">
            <h2 className="text-sm font-semibold text-indigo-300">Pricing</h2>
            <div className="mt-3 flex items-center gap-2">
              <button
                type="button"
                onClick={() => setPageState({ yearly: false })}
                className={
                  !yearly
                    ? "rounded-md border border-indigo-500 bg-indigo-600/30 px-3 py-1.5 text-xs font-semibold text-indigo-100"
                    : "rounded-md border border-gray-700 bg-gray-900 px-3 py-1.5 text-xs font-semibold text-gray-400"
                }
              >
                monthly
              </button>
              <button
                type="button"
                onClick={() => setPageState({ yearly: true })}
                className={
                  yearly
                    ? "rounded-md border border-indigo-500 bg-indigo-600/30 px-3 py-1.5 text-xs font-semibold text-indigo-100"
                    : "rounded-md border border-gray-700 bg-gray-900 px-3 py-1.5 text-xs font-semibold text-gray-400"
                }
              >
                yearly
              </button>
            </div>
            <p className="mt-4 text-4xl font-black text-emerald-300">
              ${finalPrice}
              <span className="ml-1 text-sm text-gray-400">{yearly ? "/year" : "/month"}</span>
            </p>
            <p className="mt-2 text-xs text-gray-500">Includes pipeline editor, template builder, and assistant settings.</p>
          </article>
        </section>
      </div>
    </div>
  );
}
