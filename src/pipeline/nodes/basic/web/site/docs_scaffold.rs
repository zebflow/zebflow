//! The page template a docs site is scaffolded with when its `--template`
//! file does not exist yet. The owner edits the written file from then on.

pub(super) fn default_template_source(name: Option<&str>) -> String {
    let fallback_title = name
        .filter(|s| !s.trim().is_empty())
        .unwrap_or("Docs")
        .to_string();
    format!(
        r##"import {{ useState, useEffect, useMemo, useCallback }} from "zeb/react";
import Markdown from "zeb/markdown";

const DOCS_MARKDOWN_CSS = `
.docs-markdown {{
  color: #1f2937;
  font-size: 16px;
  line-height: 1.8;
}}
.docs-markdown h1,
.docs-markdown h2,
.docs-markdown h3,
.docs-markdown h4,
.docs-markdown h5,
.docs-markdown h6 {{
  color: #020617;
  font-weight: 800;
  letter-spacing: -0.02em;
  line-height: 1.2;
  scroll-margin-top: 96px;
}}
.docs-markdown h1 {{
  font-size: 2.4rem;
  margin: 0 0 1.25rem;
}}
.docs-markdown h2 {{
  font-size: 1.7rem;
  margin: 3rem 0 1rem;
  padding-top: 0.25rem;
  border-top: 1px solid #e2e8f0;
}}
.docs-markdown h3 {{
  font-size: 1.25rem;
  margin: 2rem 0 0.85rem;
}}
.docs-markdown p,
.docs-markdown ul,
.docs-markdown ol,
.docs-markdown pre,
.docs-markdown table,
.docs-markdown blockquote {{
  margin: 1rem 0;
}}
.docs-markdown ul,
.docs-markdown ol {{
  padding-left: 1.4rem;
}}
.docs-markdown ul {{
  list-style: disc;
}}
.docs-markdown ol {{
  list-style: decimal;
}}
.docs-markdown li + li {{
  margin-top: 0.35rem;
}}
.docs-markdown li > ul,
.docs-markdown li > ol {{
  margin-top: 0.5rem;
}}
.docs-markdown a {{
  color: #c2410c;
  text-decoration: underline;
  text-underline-offset: 0.18em;
}}
.docs-markdown a:hover {{
  color: #9a3412;
}}
.docs-markdown strong {{
  color: #020617;
  font-weight: 700;
}}
.docs-markdown code {{
  background: #fff7ed;
  color: #9a3412;
  border-radius: 0.375rem;
  padding: 0.12rem 0.38rem;
  font-size: 0.92em;
}}
.docs-markdown pre {{
  overflow-x: auto;
  background: #0f172a;
  color: #e2e8f0;
  border-radius: 0.75rem;
  padding: 1rem 1.1rem;
}}
.docs-markdown pre code {{
  background: transparent;
  color: inherit;
  padding: 0;
  border-radius: 0;
}}
.docs-markdown blockquote {{
  border-left: 3px solid #fdba74;
  padding-left: 1rem;
  color: #475569;
}}
.docs-markdown hr {{
  border: 0;
  border-top: 1px solid #e2e8f0;
  margin: 2rem 0;
}}
.docs-markdown table {{
  width: 100%;
  border-collapse: collapse;
  font-size: 0.95rem;
}}
.docs-markdown th,
.docs-markdown td {{
  border: 1px solid #e2e8f0;
  padding: 0.7rem 0.8rem;
  text-align: left;
  vertical-align: top;
}}
.docs-markdown th {{
  background: #f8fafc;
  color: #0f172a;
  font-weight: 700;
}}
.docs-markdown img {{
  border-radius: 0.75rem;
}}
`;

export const page = {{
  html: {{
    lang: "en",
  }},
}};

export function getPage(input) {{
  const page = input?.page || {{}};
  const site = input?.site || {{}};
  return {{
    head: {{
      title: site.title ? `${{page.title || "Untitled"}} | ${{site.title}}` : (page.title || "Untitled"),
      description: page.description || "",
      links: page.canonical ? [
        {{ rel: "canonical", href: page.canonical }}
      ] : [],
      meta: [
        {{ property: "og:title", content: page.title || "Untitled" }},
        {{ property: "og:description", content: page.description || "" }}
      ]
    }}
  }};
}}

function MarkdownBlock(props) {{
  return (
    <div class="docs-markdown">
      <Markdown
        content={{props.content || ""}}
        class="max-w-none"
      />
    </div>
  );
}}

function SidebarItem(props) {{
  const item = props.item || {{}};
  const kids = Array.isArray(item.children) ? item.children : [];
  if (kids.length === 0) {{
    return (
      <li>
        <a
          href={{item.href || "#"}}
          class={{item.active ? "font-semibold text-orange-600" : "text-slate-700 hover:text-slate-950"}}
        >
          {{item.title}}
        </a>
      </li>
    );
  }}
  return (
    <li>
      <details open={{item.expanded}}>
        <summary class="cursor-pointer font-semibold text-slate-900">
          {{item.href ? <a href={{item.href}} class="hover:text-orange-600">{{item.title}}</a> : item.title}}
        </summary>
        <ul class="mt-2 ml-3 space-y-2 border-l border-slate-200 pl-4">
          {{kids.map((child) => <SidebarItem item={{child}} />)}}
        </ul>
      </details>
    </li>
  );
}}

export default function DocsTemplate(input) {{
  const page = input.page || {{}};
  const sidebar = Array.isArray(input.sidebar) ? input.sidebar : [];
  const toc = Array.isArray(page.headings) ? page.headings.filter((item) => Number(item.level) >= 2 && Number(item.level) <= 3) : [];
  const breadcrumbs = Array.isArray(page.breadcrumbs) ? page.breadcrumbs : [];
  const [query, setQuery] = useState("");
  const [searchIndex, setSearchIndex] = useState([]);
  const [searchState, setSearchState] = useState("idle");
  const searchHref = input.site?.search_index_href || null;
  const jumpToHeading = useCallback((event, id) => {{
    if (!id) return;
    event?.preventDefault?.();
    const target = document.getElementById(id);
    if (!target) return;
    target.scrollIntoView({{ block: "start", behavior: "smooth" }});
    if (window?.history?.replaceState) {{
      window.history.replaceState(null, "", "#" + id);
    }} else {{
      window.location.hash = id;
    }}
  }}, []);

  useEffect(() => {{
    if (!searchHref || searchIndex.length > 0 || searchState === "loading") return;
    setSearchState("loading");
    fetch(searchHref)
      .then((response) => response.ok ? response.json() : [])
      .then((payload) => {{
        setSearchIndex(Array.isArray(payload) ? payload : []);
        setSearchState("ready");
      }})
      .catch(() => {{
        setSearchIndex([]);
        setSearchState("error");
      }});
  }}, [searchHref, searchIndex.length, searchState]);

  const searchResults = useMemo(() => {{
    const value = String(query || "").trim().toLowerCase();
    if (!value) return [];
    const terms = value.split(/\s+/).filter(Boolean);
    return (Array.isArray(searchIndex) ? searchIndex : [])
      .map((entry) => {{
        const title = String(entry.title || "");
        const description = String(entry.description || "");
        const keywords = Array.isArray(entry.keywords) ? entry.keywords.join(" ") : "";
        const headingsText = Array.isArray(entry.headings) ? entry.headings.join(" ") : "";
        const body = String(entry.excerpt || "");
        const haystack = (title + " " + description + " " + keywords + " " + headingsText + " " + body).toLowerCase();
        let score = 0;
        for (const term of terms) {{
          if (!haystack.includes(term)) return null;
          if (title.toLowerCase() === term) score += 120;
          else if (title.toLowerCase().startsWith(term)) score += 80;
          else if (title.toLowerCase().includes(term)) score += 50;
          if (keywords.toLowerCase().includes(term)) score += 25;
          if (headingsText.toLowerCase().includes(term)) score += 18;
          if (description.toLowerCase().includes(term)) score += 12;
          if (body.toLowerCase().includes(term)) score += 6;
        }}
        return {{ ...entry, score }};
      }})
      .filter(Boolean)
      .sort((a, b) => b.score - a.score || String(a.title).localeCompare(String(b.title)))
      .slice(0, 16);
  }}, [query, searchIndex]);

  return (
    <Page>
      <style>{{DOCS_MARKDOWN_CSS}}</style>
      <div class="min-h-screen bg-white text-slate-900 lg:h-screen lg:overflow-hidden">
        <div class="border-b border-slate-200 px-4 py-3 lg:hidden">
          <a href={{input.site?.home_path || input.site?.deploy_base_path || input.site?.base_path || "/"}} class="text-xs font-semibold uppercase tracking-[0.22em] text-orange-600">{fallback_title}</a>
          <div class="mt-1 text-lg font-bold tracking-tight">{{page.title || input.site?.title || "Docs"}}</div>
        </div>
        <div class="lg:grid lg:h-screen lg:grid-cols-12">
          <aside class="border-b border-slate-200 px-4 py-4 lg:col-span-3 lg:overflow-y-auto lg:border-b-0 lg:border-r lg:px-5 lg:py-5">
            <div class="text-xs font-semibold uppercase tracking-[0.22em] text-orange-600">{fallback_title}</div>
              <div class="mt-3">
                <input
                  type="search"
                  value={{query}}
                  onInput={{(event) => setQuery(event.currentTarget.value)}}
                  placeholder="Search the docs"
                  class="h-10 w-full rounded-md border border-slate-300 px-3 text-sm outline-none ring-0 focus:border-orange-500"
                />
              <div class="mt-2 text-[11px] text-slate-500">
                {{query.trim() ? `${{searchResults.length}} result${{searchResults.length === 1 ? "" : "s"}}` : "Type to search headings, keywords, and content"}}
              </div>
            </div>
            {{query.trim() ? (
              <div class="mt-4 border-b border-slate-200 pb-4">
                <ul class="space-y-3">
                  {{searchResults.length === 0 ? (
                    <li class="text-sm text-slate-500">
                      {{searchState === "loading" ? "Loading search index..." : "No matching pages."}}
                    </li>
                  ) : searchResults.map((entry) => (
                    <li>
                      <a href={{entry.href}} class="block">
                        <div class="text-sm font-semibold text-slate-900 hover:text-orange-600">{{entry.title}}</div>
                        {{entry.section ? <div class="mt-0.5 text-[11px] uppercase tracking-[0.16em] text-slate-400">{{entry.section}}</div> : null}}
                        {{entry.description ? <div class="mt-1 text-sm text-slate-600">{{entry.description}}</div> : null}}
                      </a>
                    </li>
                  ))}}
                </ul>
              </div>
            ) : null}}
            <nav class="mt-4 pb-8">
              <div class="mb-3 text-[11px] font-semibold uppercase tracking-[0.18em] text-slate-400">Contents</div>
              <ul class="space-y-2 text-[15px]">
                {{sidebar.map((item) => <SidebarItem item={{item}} />)}}
              </ul>
            </nav>
          </aside>
          <main class="min-w-0 px-4 py-5 lg:col-span-6 lg:overflow-y-auto lg:px-8 lg:py-8">
            <div class="mx-auto max-w-3xl">
              <nav class="mb-3 flex flex-wrap items-center gap-2 text-xs text-slate-500">
                {{breadcrumbs.map((crumb, index) => (
                  <span class="inline-flex items-center gap-2">
                    {{index > 0 ? <span>/</span> : null}}
                    <a href={{crumb.href}} class="hover:text-slate-800">{{crumb.title}}</a>
                  </span>
                ))}}
              </nav>
              <header class="mb-8 border-b border-slate-200 pb-5">
                <div class="text-xs font-semibold uppercase tracking-[0.22em] text-orange-600">{{input.site?.title || "Docs"}}</div>
                <h1 class="mt-2 text-3xl font-black tracking-tight text-slate-950">{{page.title || input.site?.title || "Docs"}}</h1>
                {{page.description ? <p class="mt-3 max-w-2xl text-base leading-7 text-slate-600">{{page.description}}</p> : null}}
              </header>
              <article class="pb-8">
                <MarkdownBlock content={{page.markdown || ""}} />
              </article>
              <div class="mt-8 grid gap-3 border-t border-slate-200 pt-5 md:grid-cols-2">
                {{page.prev?.href ? (
                  <a href={{page.prev.href}} class="block border border-slate-200 px-4 py-4 hover:border-orange-300">
                    <div class="text-[11px] uppercase tracking-[0.18em] text-slate-400">Previous</div>
                    <div class="mt-1 font-semibold">{{page.prev.title}}</div>
                  </a>
                ) : <div />}}
                {{page.next?.href ? (
                  <a href={{page.next.href}} class="block border border-slate-200 px-4 py-4 text-right hover:border-orange-300">
                    <div class="text-[11px] uppercase tracking-[0.18em] text-slate-400">Next</div>
                    <div class="mt-1 font-semibold">{{page.next.title}}</div>
                  </a>
                ) : <div />}}
              </div>
            </div>
          </main>
          <aside class="hidden border-l border-slate-200 px-5 py-8 lg:col-span-3 lg:block lg:overflow-y-auto">
            <div class="text-[11px] font-semibold uppercase tracking-[0.18em] text-slate-400">On this page</div>
            <ul class="mt-4 space-y-2 text-sm">
              {{toc.length === 0 ? <li class="text-slate-400">No sections</li> : toc.map((item) => (
                <li class={{Number(item.level) === 3 ? "ml-4" : ""}}>
                  <a
                    href={{"#" + item.id}}
                    onClick={{(event) => jumpToHeading(event, item.id)}}
                    class="text-slate-700 hover:text-slate-900"
                  >
                    {{item.text}}
                  </a>
                </li>
              ))}}
            </ul>
          </aside>
        </div>
      </div>
    </Page>
  );
}}
"##
    )
}

