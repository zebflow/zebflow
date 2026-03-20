import { useState, useMemo, useNavigate, Link } from "zeb";

// ─── Inline sub-components ────────────────────────────────────────────────

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

function BlogCard({ title, excerpt, author, date, url, tag }: any) {
  return (
    <article class="bg-zinc-900 border border-zinc-800 rounded-xl p-5 hover:border-zinc-600 transition-colors">
      {tag && (
        <span class="inline-block mb-3 px-2 py-0.5 text-xs font-semibold bg-emerald-950 text-emerald-300 border border-emerald-800 rounded-full">
          {tag}
        </span>
      )}
      <h2 class="text-base font-semibold text-zinc-100 mb-1.5">
        <Link href={url} class="hover:text-emerald-400 no-underline">{title}</Link>
      </h2>
      <p class="text-zinc-400 text-sm leading-relaxed mb-4">{excerpt}</p>
      <div class="flex items-center justify-between text-xs text-zinc-500">
        <span>{author}</span>
        <span>{date}</span>
      </div>
    </article>
  );
}

// ─── Static data ─────────────────────────────────────────────────────────

const POSTS = [
  {
    id: 1,
    title: "Building Reactive Web Templates with RWE",
    excerpt: "SSR-first reactivity in Rust — no build step, no bundler. Just TSX, hooks, and Axum.",
    author: "Mala",
    date: "2026-03-01",
    url: "/",
    tag: "Deep Dive",
  },
  {
    id: 2,
    title: "useState, useMemo, useEffect in SSR Context",
    excerpt: "How React-compatible hooks behave differently during server render vs client hydration.",
    author: "Mala",
    date: "2026-02-20",
    url: "/",
    tag: "Hooks",
  },
  {
    id: 3,
    title: "SPA Navigation Without a Framework",
    excerpt: "useNavigate and Link give SPA-style history routing on top of plain Axum routes.",
    author: "Mala",
    date: "2026-02-10",
    url: "/",
    tag: "Navigation",
  },
  {
    id: 4,
    title: "Component Modularity and @/ Imports",
    excerpt: "Reusable components with @/ path aliases that work in pages, layouts, and component files.",
    author: "Mala",
    date: "2026-01-28",
    url: "/",
    tag: "DX",
  },
  {
    id: 5,
    title: "fetch() Domain Allowlist — Security at Compile Time",
    excerpt: "Static analysis blocks fetch() calls to unlisted domains before the code ever runs.",
    author: "Mala",
    date: "2026-01-15",
    url: "/",
    tag: "Security",
  },
];

const ALL_TAGS = ["All", "Deep Dive", "Hooks", "Navigation", "DX", "Security"];

// ─── Page ─────────────────────────────────────────────────────────────────

export const page = {
  head: { title: "Blog — RWE Demo" },
  html: { lang: "en" },
};

export default function BlogPage(input: any) {
  const [search, setSearch] = useState<string>("");
  const [activeTag, setActiveTag] = useState<string>("All");

  // useMemo — filtered list recomputed only when search or tag change
  const filtered = useMemo(() => {
    return POSTS.filter((p) => {
      const matchTag = activeTag === "All" || p.tag === activeTag;
      const q = search.toLowerCase();
      const matchSearch =
        !q ||
        p.title.toLowerCase().includes(q) ||
        p.excerpt.toLowerCase().includes(q);
      return matchTag && matchSearch;
    });
  }, [search, activeTag]);

  const postCount = useMemo(() => filtered.length, [filtered]);

  return (
    <div class="min-h-screen bg-zinc-950 text-zinc-100 flex flex-col">
      <NavBar active="/blog" />

      <main class="flex-1 max-w-3xl mx-auto w-full px-6 py-10">

        {/* Hero */}
        <div class="mb-8">
          <p class="text-xs font-mono text-emerald-400 uppercase tracking-widest mb-1">Blog</p>
          <h1 class="text-3xl font-bold text-zinc-100 mb-2">Engineering Posts</h1>
          <p class="text-zinc-400 text-sm">RWE internals, hooks, navigation, and DX patterns.</p>
        </div>

        {/* Search (useState) */}
        <div class="mb-5">
          <input
            type="search"
            value={search}
            onInput={(e: any) => setSearch(e?.target?.value ?? "")}
            placeholder="Search posts…"
            class="w-full max-w-sm bg-zinc-900 border border-zinc-700 rounded-lg px-4 py-2 text-sm text-zinc-100 placeholder-zinc-600 focus:border-emerald-500 focus:outline-none"
          />
        </div>

        {/* Tag filter (useState) */}
        <div class="flex gap-2 flex-wrap mb-6">
          {ALL_TAGS.map((tag) => (
            <button
              key={tag}
              onClick={() => setActiveTag(tag)}
              class={
                activeTag === tag
                  ? "px-3 py-1 bg-emerald-950 border border-emerald-700 rounded-full text-xs font-medium text-emerald-300"
                  : "px-3 py-1 bg-zinc-900 border border-zinc-700 rounded-full text-xs font-medium text-zinc-400 hover:text-zinc-200"
              }
            >
              {tag}
            </button>
          ))}
        </div>

        {/* Result count (useMemo) */}
        <p class="text-xs text-zinc-500 mb-4">
          {postCount} post{postCount !== 1 ? "s" : ""}
          {activeTag !== "All" ? ` in "${activeTag}"` : ""}
          {search ? ` matching "${search}"` : ""}
        </p>

        {/* Posts (useMemo filtered list) */}
        {postCount === 0 ? (
          <div class="text-center py-16 text-zinc-500">
            <p class="text-base mb-3">No posts match your query.</p>
            <button
              onClick={() => { setSearch(""); setActiveTag("All"); }}
              class="text-sm text-emerald-400 hover:text-emerald-300"
            >
              Clear filters
            </button>
          </div>
        ) : (
          <div class="grid grid-cols-1 gap-4 sm:grid-cols-2">
            {filtered.map((p: any) => (
              <BlogCard
                key={String(p.id)}
                title={p.title}
                excerpt={p.excerpt}
                author={p.author}
                date={p.date}
                url={p.url}
                tag={p.tag}
              />
            ))}
          </div>
        )}

        <div class="mt-10 pt-5 border-t border-zinc-800 flex gap-4 text-sm">
          <Link href="/" class="text-zinc-400 hover:text-zinc-200 no-underline">← Home</Link>
          <Link href="/todo" class="text-zinc-400 hover:text-zinc-200 no-underline">Todo →</Link>
        </div>

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
