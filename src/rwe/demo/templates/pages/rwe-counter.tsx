import { usePageState } from "zeb";

export default function Page(input) {
  const initialCount = Number(input?.initialCount || 0);
  const page = usePageState({});
  const count = Number(page?.count ?? initialCount);

  const setCount = (next) => {
    page.setPageState({ count: Number(next) });
  };

  const status =
    count < 0 ? "negative" : count > 9 ? "high" : count === 0 ? "idle" : "normal";

  return (
    <main className="min-h-screen bg-gray-950 text-gray-100 px-6 py-10 font-mono">
      <section className="mx-auto w-full max-w-3xl rounded-2xl border border-gray-800 bg-gradient-to-br from-gray-900 to-gray-950 shadow-2xl">
        <header className="border-b border-gray-800 p-6">
          <p className="text-xs uppercase tracking-[0.22em] text-indigo-300">rwe</p>
          <h1 className="mt-2 text-3xl font-bold text-indigo-400">Counter Control Panel</h1>
          <p className="mt-1 text-sm text-gray-400">
            Styled with the comprehensive RWE visual language.
          </p>
        </header>

        <div className="grid gap-6 p-6 md:grid-cols-[1.2fr_1fr]">
          <div className="rounded-xl border border-gray-800 bg-black/30 p-5">
            <p className="text-xs uppercase tracking-widest text-gray-500">Current Count</p>
            <p
              className={
                status === "negative"
                  ? "mt-3 text-6xl font-black text-red-400"
                  : status === "high"
                    ? "mt-3 text-6xl font-black text-green-400"
                    : status === "idle"
                      ? "mt-3 text-6xl font-black text-gray-300"
                      : "mt-3 text-6xl font-black text-indigo-300"
              }
            >
              {count}
            </p>
            <p className="mt-2 text-xs text-gray-500">
              Status:{" "}
              <span
                className={
                  status === "negative"
                    ? "text-red-300"
                    : status === "high"
                      ? "text-green-300"
                      : status === "idle"
                        ? "text-gray-300"
                        : "text-indigo-300"
                }
              >
                {status}
              </span>
            </p>
          </div>

          <div className="rounded-xl border border-gray-800 bg-black/30 p-5">
            <p className="text-xs uppercase tracking-widest text-gray-500">Actions</p>
            <div className="mt-4 grid grid-cols-3 gap-3">
              <button
                type="button"
                onClick={() => setCount(count - 1)}
                className="rounded-lg border border-gray-700 bg-gray-900 py-2 text-sm font-semibold hover:bg-gray-800"
              >
                -1
              </button>
              <button
                type="button"
                onClick={() => setCount(0)}
                className="rounded-lg border border-gray-700 bg-gray-900 py-2 text-sm font-semibold hover:bg-gray-800"
              >
                Reset
              </button>
              <button
                type="button"
                onClick={() => setCount(count + 1)}
                className="rounded-lg border border-indigo-600 bg-indigo-700/30 py-2 text-sm font-semibold text-indigo-100 hover:bg-indigo-700/50"
              >
                +1
              </button>
            </div>

            <div className="mt-4 rounded-md bg-gray-900/60 p-3 text-xs text-gray-400">
              <p>Tip:</p>
              <p className="mt-1">
                values {">"} 9 become <span className="text-green-300">high</span>, below 0 become{" "}
                <span className="text-red-300">negative</span>.
              </p>
            </div>
          </div>
        </div>

        <footer className="border-t border-gray-800 px-6 py-4 text-xs text-gray-500">
          Demo route powered by <span className="text-indigo-300">rwe</span> + Preact SSR.
        </footer>
      </section>
    </main>
  );
}
