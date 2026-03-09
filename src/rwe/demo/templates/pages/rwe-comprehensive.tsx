import { usePageState, useEffect, useMemo, useRef, useState } from "rwe";

const STATUS = {
  ok: "success",
  warn: "warning",
  err: "danger",
};

const BADGES = [
  { id: "alpha", label: "Alpha", score: 99, status: STATUS.ok },
  { id: "beta", label: "Beta", score: 72, status: STATUS.warn },
  { id: "gamma", label: "Gamma", score: 41, status: STATUS.err },
];

const TREE = [
  {
    id: "node-a",
    label: "Node A",
    children: [
      {
        id: "node-a1",
        label: "Child A1",
        leaves: ["a1-x", "a1-y"],
      },
      {
        id: "node-a2",
        label: "Child A2",
        leaves: ["a2-x"],
      },
    ],
  },
  {
    id: "node-b",
    label: "Node B",
    children: [
      {
        id: "node-b1",
        label: "Child B1",
        leaves: [],
      },
    ],
  },
];

export default function RweComprehensive(input) {
  const state = usePageState({
    tab: "badges",
    count: Number(input?.seedCount || 3),
    filter: "",
  });
  const tab = state?.tab ?? "badges";
  const count = Number(state?.count ?? Number(input?.seedCount || 3));
  const filter = state?.filter ?? "";
  const setPageState = state.setPageState;
  const [localVal, setLocalVal] = useState("hello");
  const [logs, setLogs] = useState([]);
  const inputRef = useRef(null);

  useEffect(() => {
    if (inputRef?.current) inputRef.current.focus();
  }, []);

  useEffect(() => {
    const line = `count changed -> ${count}`;
    setLogs((prev) => [line, ...prev].slice(0, 6));
  }, [count]);

  const filteredBadges = useMemo(() => {
    const keyword = String(filter || "").toLowerCase().trim();
    if (!keyword) return BADGES;
    return BADGES.filter((b) => b.label.toLowerCase().includes(keyword));
  }, [filter]);

  return (
    <div className="min-h-screen bg-gray-950 px-6 py-10 font-mono text-gray-100">
      <div className="mx-auto max-w-6xl rounded-2xl border border-gray-800 bg-gradient-to-br from-gray-900 to-gray-950 shadow-2xl">
        <header className="border-b border-gray-800 px-6 py-5">
          <p className="text-xs uppercase tracking-[0.22em] text-indigo-300">rwe</p>
          <h1 className="mt-2 text-3xl font-bold text-indigo-400">Comprehensive Playground</h1>
          <p className="mt-1 text-sm text-gray-400">Feature matrix inspired by the RWE comprehensive page.</p>
        </header>

        <main className="grid gap-6 p-6 lg:grid-cols-[280px_1fr]">
          <aside className="space-y-4">
            <section className="rounded-xl border border-gray-800 bg-black/30 p-4">
              <h2 className="text-xs uppercase tracking-widest text-gray-500">Local Input</h2>
              <input
                ref={inputRef}
                type="text"
                value={localVal}
                onInput={(event) => setLocalVal(event?.target?.value || "")}
                className="mt-3 w-full rounded-md border border-gray-700 bg-gray-900 px-3 py-2 text-sm text-gray-100 focus:border-indigo-500 focus:outline-none"
              />
              <p className="mt-2 text-xs text-gray-400">
                value:{" "}
                <span className={localVal.length > 8 ? "text-pink-300" : "text-emerald-300"}>
                  {localVal}
                </span>
              </p>
            </section>

            <section className="rounded-xl border border-gray-800 bg-black/30 p-4">
              <h2 className="text-xs uppercase tracking-widest text-gray-500">Counter</h2>
              <p className="mt-2 text-4xl font-black text-indigo-300">{count}</p>
              <div className="mt-3 grid grid-cols-3 gap-2">
                <button
                  type="button"
                  onClick={() => setPageState({ count: count - 1 })}
                  className="rounded-md border border-gray-700 bg-gray-900 py-1.5 text-xs font-semibold hover:bg-gray-800"
                >
                  -1
                </button>
                <button
                  type="button"
                  onClick={() => setPageState({ count: 0 })}
                  className="rounded-md border border-gray-700 bg-gray-900 py-1.5 text-xs font-semibold hover:bg-gray-800"
                >
                  reset
                </button>
                <button
                  type="button"
                  onClick={() => setPageState({ count: count + 1 })}
                  className="rounded-md border border-indigo-500 bg-indigo-600/30 py-1.5 text-xs font-semibold text-indigo-100 hover:bg-indigo-600/50"
                >
                  +1
                </button>
              </div>
            </section>

            <section className="rounded-xl border border-gray-800 bg-black/30 p-4">
              <h2 className="text-xs uppercase tracking-widest text-gray-500">Change Log</h2>
              <div className="mt-3 space-y-1">
                {logs.length === 0 ? (
                  <p className="text-xs text-gray-600">No changes yet.</p>
                ) : (
                  logs.map((line, idx) => (
                    <p key={`${line}-${idx}`} className="text-xs text-gray-400">
                      {line}
                    </p>
                  ))
                )}
              </div>
            </section>
          </aside>

          <section className="space-y-4">
            <div className="rounded-xl border border-gray-800 bg-black/30 p-4">
              <div className="flex flex-wrap items-center gap-2">
                {["badges", "tree", "stats"].map((nextTab) => (
                  <button
                    key={nextTab}
                    type="button"
                    onClick={() => setPageState({ tab: nextTab })}
                    className={
                      tab === nextTab
                        ? "rounded-md border border-indigo-500 bg-indigo-600/30 px-3 py-1.5 text-xs font-semibold uppercase tracking-wide text-indigo-100"
                        : "rounded-md border border-gray-700 bg-gray-900 px-3 py-1.5 text-xs font-semibold uppercase tracking-wide text-gray-400 hover:text-gray-200"
                    }
                  >
                    {nextTab}
                  </button>
                ))}
                <input
                  type="text"
                  value={filter}
                  onInput={(event) => setPageState({ filter: event?.target?.value || "" })}
                  placeholder="filter…"
                  className="ml-auto min-w-[180px] rounded-md border border-gray-700 bg-gray-900 px-3 py-1.5 text-xs text-gray-200 focus:border-indigo-500 focus:outline-none"
                />
              </div>
            </div>

            {tab === "badges" ? (
              <div className="grid gap-3 md:grid-cols-3">
                {filteredBadges.map((badge, idx) => (
                  <article key={badge.id} className="rounded-xl border border-gray-800 bg-black/30 p-4">
                    <p className="text-xs text-gray-500">#{idx + 1}</p>
                    <h3 className="mt-1 text-lg font-semibold text-gray-100">{badge.label}</h3>
                    <p
                      className={
                        badge.score > 80
                          ? "mt-2 text-sm text-green-300"
                          : badge.score > 60
                            ? "mt-2 text-sm text-yellow-300"
                            : "mt-2 text-sm text-red-300"
                      }
                    >
                      score: {badge.score}
                    </p>
                  </article>
                ))}
              </div>
            ) : null}

            {tab === "tree" ? (
              <div className="space-y-3">
                {TREE.map((root) => (
                  <article key={root.id} className="rounded-xl border border-gray-800 bg-black/30 p-4">
                    <h3 className="text-sm font-semibold text-indigo-300">{root.label}</h3>
                    <div className="mt-3 space-y-2">
                      {root.children.map((child) => (
                        <div key={child.id} className="rounded-md border border-gray-800 bg-gray-900/70 px-3 py-2">
                          <p className="text-xs font-semibold text-gray-200">{child.label}</p>
                          <p className="mt-1 text-xs text-gray-500">
                            {child.leaves.length > 0 ? child.leaves.join(", ") : "no leaves"}
                          </p>
                        </div>
                      ))}
                    </div>
                  </article>
                ))}
              </div>
            ) : null}

            {tab === "stats" ? (
              <div className="grid gap-3 md:grid-cols-2">
                {[12, 38, 67, 91].map((value, idx) => (
                  <div key={`stat-${idx}`} className="rounded-xl border border-gray-800 bg-black/30 p-4">
                    <p className="text-xs uppercase tracking-widest text-gray-500">Pipeline {idx + 1}</p>
                    <p className="mt-1 text-2xl font-bold text-gray-100">{value}%</p>
                    <div className="mt-3 h-2 rounded bg-gray-800">
                      <div className="h-2 rounded bg-indigo-500" style={{ width: `${value}%` }} />
                    </div>
                  </div>
                ))}
              </div>
            ) : null}
          </section>
        </main>
      </div>
    </div>
  );
}
