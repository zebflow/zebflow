import { useState, useEffect, useRef, useMemo, usePageState, useNavigate, Link } from "zeb";

// ─── Inline sub-components ─────────────────────────────────────────────────

function NavBar({ active }: any) {
  return (
    <nav class="bg-zinc-900 border-b border-zinc-800 px-6 py-3 flex items-center justify-between sticky top-0 z-10">
      <span class="text-zinc-100 font-bold text-sm tracking-tight">RWE Demo</span>
      <div class="flex gap-5 text-sm">
        <Link href="/" class={active === "/" ? "text-emerald-400 font-medium" : "text-zinc-400 hover:text-zinc-100"}>Home</Link>
        <Link href="/blog" class={active === "/blog" ? "text-emerald-400 font-medium" : "text-zinc-400 hover:text-zinc-100"}>Blog</Link>
        <Link href="/todo" class={active === "/todo" ? "text-emerald-400 font-medium" : "text-zinc-400 hover:text-zinc-100"}>Todo</Link>
      </div>
    </nav>
  );
}

function Badge({ label }: any) {
  return (
    <span class="inline-flex items-center px-2 py-0.5 text-xs font-mono font-semibold text-emerald-300 bg-emerald-950 border border-emerald-800 rounded">
      {label}
    </span>
  );
}

function Card({ badge, title, children }: any) {
  return (
    <section class="bg-zinc-900 border border-zinc-800 rounded-xl p-6">
      <div class="flex items-center gap-2 mb-4">
        <Badge label={badge} />
        <span class="text-sm text-zinc-400">{title}</span>
      </div>
      {children}
    </section>
  );
}

// ─── Page ──────────────────────────────────────────────────────────────────

export const page = {
  head: { title: "RWE — Spec Showcase" },
  html: { lang: "en" },
};

export default function HomePage(input: any) {
  // useState
  const [count, setCount] = useState<number>(input.count ?? 0);
  const [name, setName] = useState<string>(input.name ?? "World");
  const [tab, setTab] = useState<string>("hooks");

  // useRef — survives re-renders without triggering them
  const renderCount = useRef<number>(0);
  renderCount.current += 1;

  // useMemo — recomputed only when deps change
  const doubled = useMemo(() => count * 2, [count]);
  const tripled = useMemo(() => count * 3, [count]);
  const greeting = useMemo(() => `Hello, ${name}!`, [name]);

  // usePageState — shared across all components in this page tree
  const { theme = "dark", setPageState } = usePageState({ theme: "dark" });

  // useNavigate — SPA navigation
  const navigate = useNavigate();

  // useEffect — runs after each count change (no-op in SSR)
  useEffect(() => {
    // fires client-side after every count update
  }, [count]);

  const TABS = ["hooks", "navigation", "state"];

  return (
    <div class="min-h-screen bg-zinc-950 text-zinc-100 flex flex-col">
      <NavBar active="/" />

      <main class="flex-1 max-w-3xl mx-auto w-full px-6 py-10 space-y-6">

        {/* Hero */}
        <div>
          <p class="text-xs font-mono text-emerald-400 uppercase tracking-widest mb-1">Reactive Web Engine</p>
          <h1 class="text-3xl font-bold text-zinc-100 mb-2">{greeting}</h1>
          <p class="text-zinc-400 text-sm leading-relaxed">
            Every spec feature. One page. useState · useEffect · useRef · useMemo · usePageState · useNavigate · Link
          </p>
        </div>

        {/* Tab bar */}
        <div class="flex gap-1 bg-zinc-900 border border-zinc-800 rounded-lg p-1 w-fit">
          {TABS.map((t) => (
            <button
              key={t}
              onClick={() => setTab(t)}
              class={tab === t
                ? "px-4 py-1.5 rounded-md bg-zinc-700 text-zinc-100 text-sm font-medium"
                : "px-4 py-1.5 rounded-md text-zinc-500 text-sm hover:text-zinc-300"}
            >
              {t}
            </button>
          ))}
        </div>

        {/* ── Hooks tab ─────────────────────────────────────────────── */}
        {tab === "hooks" && (
          <div class="space-y-4">

            <Card badge="useState + useMemo" title="Local state with memoized derived values">
              <p class="text-5xl font-mono font-bold text-emerald-400 mb-3">{count}</p>
              <div class="flex gap-2 text-xs text-zinc-400 mb-5">
                <span class="bg-zinc-800 px-2 py-1 rounded">×2 = <strong class="text-zinc-200">{doubled}</strong></span>
                <span class="bg-zinc-800 px-2 py-1 rounded">×3 = <strong class="text-zinc-200">{tripled}</strong></span>
              </div>
              <div class="flex gap-2">
                <button onClick={() => setCount(count - 1)} class="px-4 py-2 bg-zinc-800 hover:bg-zinc-700 rounded-lg font-mono text-lg leading-none">−</button>
                <button onClick={() => setCount(count + 1)} class="px-4 py-2 bg-emerald-950 hover:bg-emerald-900 border border-emerald-700 rounded-lg font-mono text-lg leading-none text-emerald-200">+</button>
                <button onClick={() => setCount(0)} class="px-4 py-2 bg-zinc-800 hover:bg-zinc-700 rounded-lg text-sm text-zinc-400">reset</button>
              </div>
            </Card>

            <Card badge="useRef" title="Mutable ref — survives renders, doesn't cause re-renders">
              <p class="text-zinc-300 text-sm">
                This component has rendered <span class="font-mono text-emerald-400 font-bold">{renderCount.current}</span> time(s).
                Clicking + above increments both the counter and the render count.
              </p>
            </Card>

            <Card badge="useEffect" title="Side effect after count changes">
              <p class="text-zinc-400 text-sm leading-relaxed">
                <code class="text-emerald-400 bg-zinc-800 px-1.5 py-0.5 rounded text-xs">useEffect(() =&gt; {"{…}"}, [count])</code>
                <br class="mb-1" />
                SSR: skipped entirely. Browser: fires after each count update.
              </p>
            </Card>

            <Card badge="usePageState" title="Shared state across all components in this page tree">
              <p class="text-zinc-400 text-sm mb-4">
                Active theme: <span class="font-mono text-emerald-400 font-semibold">{theme}</span>
              </p>
              <div class="flex gap-2">
                {["dark", "light", "system"].map((t) => (
                  <button
                    key={t}
                    onClick={() => setPageState({ theme: t })}
                    class={theme === t
                      ? "px-3 py-1.5 bg-emerald-950 border border-emerald-700 rounded text-xs font-medium text-emerald-200"
                      : "px-3 py-1.5 bg-zinc-800 border border-zinc-700 rounded text-xs font-medium text-zinc-400 hover:text-zinc-200"}
                  >
                    {t}
                  </button>
                ))}
              </div>
            </Card>

          </div>
        )}

        {/* ── Navigation tab ──────────────────────────────────────── */}
        {tab === "navigation" && (
          <div class="space-y-4">

            <Card badge="useNavigate" title="SPA navigation via history API">
              <p class="text-zinc-400 text-sm leading-relaxed mb-4">
                Returns a navigate function. SSR: no-op. Browser: pushes to history stack without a full reload.
              </p>
              <div class="flex gap-2 flex-wrap">
                <button onClick={() => navigate("/blog")} class="px-4 py-2 bg-blue-950 border border-blue-700 hover:bg-blue-900 rounded-lg text-sm text-blue-200">
                  navigate("/blog")
                </button>
                <button onClick={() => navigate("/todo")} class="px-4 py-2 bg-purple-950 border border-purple-700 hover:bg-purple-900 rounded-lg text-sm text-purple-200">
                  navigate("/todo")
                </button>
                <button onClick={() => navigate("/")} class="px-4 py-2 bg-zinc-800 border border-zinc-700 hover:bg-zinc-700 rounded-lg text-sm text-zinc-300">
                  navigate("/")
                </button>
              </div>
            </Card>

            <Card badge="Link" title="Router-aware anchor — &lt;a&gt; in SSR, history API on client">
              <p class="text-zinc-400 text-sm leading-relaxed mb-4">
                SSR renders as a plain <code class="text-emerald-400 bg-zinc-800 px-1 rounded text-xs">&lt;a href="..."&gt;</code> for SEO.
                Client intercepts the click and uses the history API.
              </p>
              <div class="flex gap-3 flex-wrap">
                <Link href="/blog" class="inline-flex items-center px-4 py-2 bg-zinc-800 border border-zinc-700 hover:bg-zinc-700 rounded-lg text-sm text-zinc-200 no-underline">
                  /blog →
                </Link>
                <Link href="/todo" class="inline-flex items-center px-4 py-2 bg-zinc-800 border border-zinc-700 hover:bg-zinc-700 rounded-lg text-sm text-zinc-200 no-underline">
                  /todo →
                </Link>
              </div>
            </Card>

          </div>
        )}

        {/* ── State tab ────────────────────────────────────────────── */}
        {tab === "state" && (
          <div class="space-y-4">

            <Card badge="Live input" title="useState + useMemo on typed text">
              <div class="mb-4">
                <label class="block text-xs text-zinc-500 mb-1.5">Your name</label>
                <input
                  type="text"
                  value={name}
                  onInput={(e: any) => setName(e?.target?.value ?? "")}
                  placeholder="Type a name…"
                  class="w-full max-w-xs bg-zinc-800 border border-zinc-700 rounded-lg px-3 py-2 text-sm text-zinc-100 placeholder-zinc-600 focus:border-emerald-500 focus:outline-none"
                />
              </div>
              <p class="text-zinc-200 text-base">{greeting}</p>
            </Card>

            <Card badge="Server input" title="JSON payload from the Rust render handler">
              <pre class="text-xs text-zinc-400 font-mono bg-zinc-800 rounded-lg p-4 overflow-x-auto">{JSON.stringify({ count: input.count, name: input.name }, null, 2)}</pre>
            </Card>

          </div>
        )}

      </main>

      <footer class="bg-zinc-900 border-t border-zinc-800 px-6 py-4 mt-auto">
        <div class="max-w-3xl mx-auto flex items-center justify-between text-xs text-zinc-500">
          <span>RWE — Reactive Web Engine</span>
          <div class="flex gap-4">
            <Link href="/" class="hover:text-zinc-300 no-underline">Home</Link>
            <Link href="/blog" class="hover:text-zinc-300 no-underline">Blog</Link>
            <Link href="/todo" class="hover:text-zinc-300 no-underline">Todo</Link>
          </div>
        </div>
      </footer>
    </div>
  );
}
