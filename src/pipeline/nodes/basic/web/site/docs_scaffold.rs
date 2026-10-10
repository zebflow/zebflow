//! The page template a docs site is scaffolded with when its `--template`
//! file does not exist yet. The owner edits the written file from then on.

pub(super) fn default_template_source(name: Option<&str>) -> String {
    // Written into the TSX as a JavaScript string literal, never as markup:
    // a name holding `{`, `}`, `<` or a quote stays text.
    let fallback_title = serde_json::to_string(name.filter(|s| !s.trim().is_empty()).unwrap_or("Docs"))
        .unwrap_or_else(|_| "\"Docs\"".to_string());
    format!(
        r##"import {{ useState, useEffect, useCallback }} from "zeb/react";
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
.docs-block > .docs-markdown > *:first-child {{
  margin-top: 0;
}}
.docs-block > .docs-markdown > *:last-child {{
  margin-bottom: 0;
}}
.docs-code {{
  font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  font-size: 0.9rem;
  line-height: 1.7;
  overflow-x: auto;
  white-space: pre;
  tab-size: 2;
}}
/* One element per line, so a highlighted or changed line can be tinted
   across the whole block and not only as far as its own text reaches. */
.docs-code-line {{
  display: block;
  min-height: 1.7em;
  padding-left: 1.25rem;
  padding-right: 1.25rem;
}}
/* A line is as wide as the widest line in the block, so a tint reaches the
   end of the scroll and not the end of its own text. A block child takes its
   width from its container, so the container is the one measured. */
.docs-code-body {{
  display: block;
  min-width: max-content;
}}
.docs-code-wrap .docs-code-body {{
  min-width: 0;
}}
.docs-code-wrap {{
  white-space: pre-wrap;
  overflow-wrap: anywhere;
}}
.docs-code-mark {{
  background: rgba(251, 146, 60, 0.14);
  box-shadow: inset 2px 0 0 #fb923c;
}}
.docs-code-add {{
  background: rgba(16, 185, 129, 0.16);
}}
.docs-code-remove {{
  background: rgba(244, 63, 94, 0.16);
}}
.docs-code-meta {{
  color: #94a3b8;
}}
/* The diff's own column. Unselectable, so dragging across a diff picks up
   the code and not the patch. */
.docs-code-sign {{
  display: inline-block;
  width: 1ch;
  user-select: none;
  -webkit-user-select: none;
}}
.docs-code-add > .docs-code-sign {{
  color: #6ee7b7;
}}
.docs-code-remove > .docs-code-sign {{
  color: #fda4af;
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


// ---------------------------------------------------------------------------
// Search over the chunked index the build wrote (search/manifest.json,
// search/t-<prefix>.json, search/m-<block>.json).
//
// Everything here treats the query as text and the index as data: no path is
// built from what the reader typed, no regular expression is compiled from it,
// no result is written as markup, and every number that bounds work is a
// constant below.
// ---------------------------------------------------------------------------
const SEARCH_MAX_QUERY_CHARS = 128;
const SEARCH_MAX_TERMS = 8;
const SEARCH_MAX_RESULTS = 12;
const SEARCH_SNIPPET_CHARS = 150;
const SEARCH_PREFIX_TERM_LIMIT = 24;
const searchCache = new Map();

// The build's tokeniser, in JavaScript: lower case, split on everything that
// is not a letter or a digit, two characters to thirty-two. The two must stay
// identical, or a word the build indexed cannot be typed into this box.
function searchTokens(text) {{
  const out = [];
  let current = "";
  for (const ch of String(text || "").slice(0, SEARCH_MAX_QUERY_CHARS)) {{
    if (/[\p{{L}}\p{{N}}]/u.test(ch)) {{
      current += ch.toLowerCase();
    }} else if (current) {{
      out.push(current);
      current = "";
    }}
  }}
  if (current) out.push(current);
  return out.filter((term) => term.length >= 2 && term.length <= 32).slice(0, SEARCH_MAX_TERMS);
}}

function searchFetchJson(href) {{
  if (searchCache.has(href)) return searchCache.get(href);
  const pending = fetch(href)
    .then((response) => (response.ok ? response.json() : null))
    .catch(() => null);
  searchCache.set(href, pending);
  return pending;
}}

// A chunk id is only ever one the manifest lists, and only ever one or two
// letters: the term picks from that list, it never spells a name.
function searchChunkId(term, ids) {{
  const two = term.slice(0, 2);
  if (ids.indexOf(two) !== -1) return two;
  const one = term.slice(0, 1);
  return ids.indexOf(one) !== -1 ? one : null;
}}

async function runSearch(manifestHref, typed) {{
  const terms = searchTokens(typed);
  if (!terms.length) return [];
  const base = String(manifestHref).replace(/manifest\.json$/, "");
  const manifest = await searchFetchJson(manifestHref);
  if (!manifest || typeof manifest !== "object") return [];
  const ids = (Array.isArray(manifest.term_chunks) ? manifest.term_chunks : [])
    .filter((id) => typeof id === "string" && /^[\p{{L}}\p{{N}}]{{1,2}}$/u.test(id));
  const perBlock = Number(manifest.pages_per_meta_chunk) > 0 ? Number(manifest.pages_per_meta_chunk) : 64;

  // One pass per term; a page has to answer every term to be a result.
  let hits = null;
  const trailing = terms[terms.length - 1];
  for (const term of terms) {{
    const id = searchChunkId(term, ids);
    if (!id) return [];
    const chunk = await searchFetchJson(base + "t-" + id + ".json");
    if (!chunk || typeof chunk !== "object") return [];
    const scores = new Map();
    const exact = Array.isArray(chunk[term]) ? chunk[term] : null;
    if (exact) {{
      for (const posting of exact) {{
        if (!Array.isArray(posting)) continue;
        scores.set(posting[0], Math.max(scores.get(posting[0]) || 0, Number(posting[1]) || 0));
      }}
    }}
    // The word being typed is still half a word, so it also matches as a
    // prefix — inside the one chunk already fetched, and bounded.
    if (term === trailing) {{
      let seen = 0;
      for (const key of Object.keys(chunk)) {{
        if (seen >= SEARCH_PREFIX_TERM_LIMIT) break;
        if (key === term || key.indexOf(term) !== 0) continue;
        seen += 1;
        for (const posting of chunk[key]) {{
          if (!Array.isArray(posting)) continue;
          const partial = (Number(posting[1]) || 0) / 2;
          scores.set(posting[0], Math.max(scores.get(posting[0]) || 0, partial));
        }}
      }}
    }}
    if (!scores.size) return [];
    if (hits === null) {{
      hits = scores;
    }} else {{
      const merged = new Map();
      for (const [page, score] of scores) {{
        if (hits.has(page)) merged.set(page, hits.get(page) + score);
      }}
      hits = merged;
    }}
    if (!hits.size) return [];
  }}

  const ranked = Array.from(hits.entries())
    .sort((a, b) => b[1] - a[1] || a[0] - b[0])
    .slice(0, SEARCH_MAX_RESULTS);

  // Metadata for the handful being shown: one fetch per block of pages, so a
  // thousand-page site costs the same as a fifty-page one.
  const blocks = new Map();
  for (const [page] of ranked) {{
    const block = Math.floor(page / perBlock);
    if (!blocks.has(block)) blocks.set(block, searchFetchJson(base + "m-" + block + ".json"));
  }}
  const loaded = new Map();
  for (const [block, pending] of blocks) loaded.set(block, await pending);

  // The text for the matched line, one small file per result shown — the
  // largest part of any index, and never fetched for a page nobody sees.
  const texts = await Promise.all(ranked.map(([page]) => searchFetchJson(base + "x-" + page + ".json")));

  const out = [];
  ranked.forEach(([page, score], position) => {{
    const block = loaded.get(Math.floor(page / perBlock));
    const entry = block && block[String(page)];
    if (!entry) return;
    const text = texts[position] && typeof texts[position].text === "string" ? texts[position].text : "";
    out.push({{
      href: String(entry.href || ""),
      title: String(entry.title || ""),
      section: String(entry.section || ""),
      description: String(entry.description || ""),
      snippet: searchSnippet(text || String(entry.description || ""), terms),
      score,
    }});
  }});
  return out;
}}

// The matched line, as three pieces of plain text. Found by index, never by a
// pattern built from the query, and rendered as text nodes by the component —
// a page whose Markdown contains markup cannot write markup here.
function searchSnippet(text, terms) {{
  const haystack = text.toLowerCase();
  let at = -1;
  let found = "";
  for (const term of terms) {{
    const index = haystack.indexOf(term);
    if (index !== -1 && (at === -1 || index < at)) {{
      at = index;
      found = term;
    }}
  }}
  if (at === -1) return {{ before: text.slice(0, SEARCH_SNIPPET_CHARS), match: "", after: "" }};
  const start = Math.max(0, at - Math.floor(SEARCH_SNIPPET_CHARS / 3));
  const before = (start > 0 ? "…" : "") + text.slice(start, at);
  const match = text.slice(at, at + found.length);
  const after = text.slice(at + found.length, at + found.length + SEARCH_SNIPPET_CHARS);
  return {{ before, match, after }};
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


// ---------------------------------------------------------------------------
// One component per kind of block the build parsed out of the Markdown
// (`:::note`, `:::tab`, `:::cards`, `:::code-group`). The build hands this
// page data, never markup: restyle every admonition on the site by editing
// `Admonition` below, and nothing in a `.md` file can produce an element that
// is not written here.
// ---------------------------------------------------------------------------
const ADMONITIONS = {{
  note: {{ label: "Note", edge: "border-slate-300", tint: "bg-slate-50", ink: "text-slate-600" }},
  tip: {{ label: "Tip", edge: "border-emerald-300", tint: "bg-emerald-50/70", ink: "text-emerald-700" }},
  info: {{ label: "Info", edge: "border-sky-300", tint: "bg-sky-50/70", ink: "text-sky-700" }},
  warning: {{ label: "Warning", edge: "border-amber-300", tint: "bg-amber-50/70", ink: "text-amber-700" }},
  caution: {{ label: "Caution", edge: "border-amber-300", tint: "bg-amber-50/70", ink: "text-amber-700" }},
  danger: {{ label: "Danger", edge: "border-rose-300", tint: "bg-rose-50/70", ink: "text-rose-700" }},
}};

// The components a page of this site may call by name.
//
// `<PotoPlayer src="stories/intro.poto" controls />` in a `.md` file renders
// the entry of the same name here. This is the whole of what a Markdown page
// can reach: a name that is not a key below is shown as a note, and the build
// only ever passes literal props — a string, a number or a flag — so there is
// no expression to evaluate.
//
// It starts empty on purpose. A library reaches the site because this file
// imports it, so a site that calls no component ships no library:
//
//   import {{ PotoPlayer, PotoSnapshot }} from "zeb/potoru";
//   const COMPONENTS = {{ PotoPlayer, PotoSnapshot }};
//
// A file a prop names (`src="stories/intro.poto"`) is the project's own
// `static/stories/intro.poto`, copied into the site beside the pages.
const COMPONENTS = {{}};

function ComponentBlock(props) {{
  const block = props.block || {{}};
  const Component = COMPONENTS[String(block.name || "")];
  if (!Component) {{
    // Naming a component does not conjure one. Say so where the author will
    // see it, rather than leaving a hole in the page.
    return (
      <div class="docs-block my-6 border border-dashed border-slate-300 px-4 py-3 text-sm text-slate-500">
        No component named <code class="font-mono text-slate-700">{{String(block.name || "")}}</code> on this site.
      </div>
    );
  }}
  return (
    <div class="docs-block my-6">
      <Component {{...(block.props || {{}})}} />
    </div>
  );
}}

function Blocks(props) {{
  const blocks = Array.isArray(props.blocks) ? props.blocks : [];
  return (
    <div class="docs-blocks">
      {{blocks.map((block) => <Block block={{block}} />)}}
    </div>
  );
}}

function Block(props) {{
  const block = props.block || {{}};
  const kind = String(block.kind || "");
  if (kind === "markdown") return <MarkdownBlock content={{block.text || ""}} />;
  if (kind === "admonition") return <Admonition block={{block}} />;
  if (kind === "tabs") return <TabbedPanes panes={{block.panes}} />;
  if (kind === "cards") return <CardGrid items={{block.items}} />;
  if (kind === "code_group") return <CodeGroup tabs={{block.tabs}} />;
  if (kind === "code") return <CodeBlock block={{block}} />;
  if (kind === "component") return <ComponentBlock block={{block}} />;
  // A directive this site has no component for is still the author's words.
  return <Blocks blocks={{block.blocks}} />;
}}

function Admonition(props) {{
  const block = props.block || {{}};
  const style = ADMONITIONS[String(block.variant || "note")] || ADMONITIONS.note;
  return (
    <div class={{"docs-block my-6 border-l-4 px-5 py-4 " + style.edge + " " + style.tint}}>
      <div class={{"mb-2 text-[11px] font-bold uppercase tracking-[0.18em] " + style.ink}}>
        {{block.title || style.label}}
      </div>
      <Blocks blocks={{block.blocks}} />
    </div>
  );
}}

function TabbedPanes(props) {{
  const panes = Array.isArray(props.panes) ? props.panes : [];
  const [open, setOpen] = useState(0);
  if (panes.length === 0) return null;
  const shown = panes[Math.min(open, panes.length - 1)];
  return (
    <div class="docs-block my-6 border border-slate-200">
      <div class="flex flex-wrap border-b border-slate-200 bg-slate-50">
        {{panes.map((pane, index) => (
          <button
            type="button"
            onClick={{() => setOpen(index)}}
            class={{index === open
              ? "border-b-2 border-orange-500 px-4 py-2 text-sm font-semibold text-slate-900"
              : "border-b-2 border-transparent px-4 py-2 text-sm text-slate-500 hover:text-slate-800"}}
          >
            {{pane.label}}
          </button>
        ))}}
      </div>
      <div class="px-5 py-4">
        <Blocks blocks={{shown?.blocks}} />
      </div>
    </div>
  );
}}

function CardGrid(props) {{
  const items = Array.isArray(props.items) ? props.items : [];
  if (items.length === 0) return null;
  return (
    <div class="docs-block my-6 grid gap-4 sm:grid-cols-2">
      {{items.map((item) => {{
        const body = (
          <span class="block">
            <span class="block font-semibold text-slate-900">{{item.title}}</span>
            {{item.description ? <span class="mt-1 block text-sm leading-6 text-slate-600">{{item.description}}</span> : null}}
          </span>
        );
        return item.href ? (
          <a href={{item.href}} class="block border border-slate-200 px-5 py-4 no-underline hover:border-orange-300 hover:bg-orange-50/40">
            {{body}}
          </a>
        ) : (
          <div class="block border border-dashed border-slate-200 px-5 py-4">{{body}}</div>
        );
      }})}}
    </div>
  );
}}

function CodeGroup(props) {{
  const tabs = Array.isArray(props.tabs) ? props.tabs : [];
  const [open, setOpen] = useState(0);
  const [wrap, setWrap] = useState(false);
  if (tabs.length === 0) return null;
  const shown = tabs[Math.min(open, tabs.length - 1)];
  return (
    <div class="docs-block my-6 border border-slate-200 bg-slate-950">
      <div class="flex flex-wrap items-center gap-1 border-b border-slate-800 px-2 py-1">
        {{tabs.map((tab, index) => (
          <button
            type="button"
            onClick={{() => setOpen(index)}}
            class={{index === open
              ? "px-3 py-1.5 text-xs font-semibold text-white"
              : "px-3 py-1.5 text-xs text-slate-400 hover:text-slate-200"}}
          >
            {{tab.label}}
          </button>
        ))}}
        <span class="ml-auto flex items-center gap-1">
          <WrapToggle on={{wrap}} onChange={{setWrap}} />
          <CopyButton text={{codeSource(shown?.lines)}} />
        </span>
      </div>
      <CodeLines lines={{shown?.lines}} diff={{shown?.diff}} wrap={{wrap}} />
    </div>
  );
}}

// A fenced block at the left margin of a page, with the furniture a reader
// expects around one: the filename the fence named, the lines it highlighted,
// a copy button, and a wrap toggle.
function CodeBlock(props) {{
  const block = props.block || {{}};
  const lines = Array.isArray(block.lines) ? block.lines : [];
  const [wrap, setWrap] = useState(false);
  if (lines.length === 0) return null;
  const label = String(block.title || "") || String(block.language || "");
  return (
    <div class="docs-block my-6 border border-slate-200 bg-slate-950">
      <div class="flex items-center gap-2 border-b border-slate-800 px-5 py-1.5">
        <span class="font-mono text-xs text-slate-300">{{label}}</span>
        <span class="ml-auto flex items-center gap-1">
          <WrapToggle on={{wrap}} onChange={{setWrap}} />
          <CopyButton text={{codeSource(lines)}} />
        </span>
      </div>
      <CodeLines lines={{lines}} diff={{block.diff}} wrap={{wrap}} />
    </div>
  );
}}

// What the copy button puts on the clipboard: the code, not the patch.
//
// A reader copies a block to use it, and the result of a diff is the file
// after the change — so a removed line and a `@@` header are left out, and
// what lands on the clipboard is a file that parses. The block still shows
// them: a diff is read as a change and copied as a result.
function codeSource(lines) {{
  return (Array.isArray(lines) ? lines : [])
    .filter((line) => line?.mark !== "remove" && line?.mark !== "meta")
    .map((line) => String(line?.text || ""))
    .join("\n");
}}

const DIFF_SIGNS = {{ add: "+", remove: "-" }};

function CodeLines(props) {{
  const lines = Array.isArray(props.lines) ? props.lines : [];
  const diff = !!props.diff;
  return (
    <pre class={{"docs-code py-4 text-slate-100" + (props.wrap ? " docs-code-wrap" : "")}}>
      <span class="docs-code-body">
      {{lines.map((line) => {{
        const mark = String(line?.mark || "");
        const tint = (line?.highlight ? " docs-code-mark" : "")
          + (mark === "add" ? " docs-code-add" : "")
          + (mark === "remove" ? " docs-code-remove" : "")
          + (mark === "meta" ? " docs-code-meta" : "");
        return (
          <span class={{"docs-code-line" + tint}}>
            {{diff ? <span class="docs-code-sign">{{DIFF_SIGNS[mark] || " "}}</span> : null}}
            {{String(line?.text || "") || " "}}
          </span>
        );
      }})}}
      </span>
    </pre>
  );
}}

// Off by default, and per block rather than per site: a wrapped command line
// is a misread command line, but a long line nobody can scroll to is worse.
function WrapToggle(props) {{
  const on = !!props.on;
  return (
    <button
      type="button"
      onClick={{() => props.onChange(!on)}}
      aria-pressed={{on ? "true" : "false"}}
      class={{on
        ? "px-2 py-1 text-xs font-semibold text-slate-100"
        : "px-2 py-1 text-xs text-slate-400 hover:text-slate-100"}}
    >
      Wrap
    </button>
  );
}}

// Copies the source the author wrote, not the markup around it. The label is
// the only thing that changes, so nothing moves when it does.
function CopyButton(props) {{
  const [copied, setCopied] = useState(false);
  const copy = useCallback(() => {{
    const text = String(props.text || "");
    const done = () => {{
      setCopied(true);
      setTimeout(() => setCopied(false), 1600);
    }};
    if (navigator?.clipboard?.writeText) {{
      navigator.clipboard.writeText(text).then(done).catch(() => {{}});
    }}
  }}, [props.text]);
  return (
    <button
      type="button"
      onClick={{copy}}
      class="w-16 px-2 py-1.5 text-xs text-slate-400 hover:text-slate-100"
    >
      {{copied ? "Copied" : "Copy"}}
    </button>
  );
}}

// `1`, `1.4`, `1.4.1`, on a site whose root `_meta.yaml` says
// `numbered: true`. The build does the counting, from the same tree the
// sidebar is drawn from; nothing here derives a number from a loop index,
// which would disagree the moment a template showed a different set.
function SectionNumber(props) {{
  if (!props.number) return null;
  return (
    <span class="mr-2 tabular-nums text-[12px] font-normal text-slate-400">{{props.number}}</span>
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
          <SectionNumber number={{item.number}} />
          {{item.title}}
        </a>
      </li>
    );
  }}
  return (
    <li>
      <details open={{item.expanded}}>
        <summary class="cursor-pointer font-semibold text-slate-900">
          <SectionNumber number={{item.number}} />
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
  const [results, setResults] = useState([]);
  const [searchState, setSearchState] = useState("idle");
  const [activeHeading, setActiveHeading] = useState("");
  const manifestHref = input.site?.search_manifest_href || null;
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

  // Which heading the reader is on, so the outline says where they are. The
  // column scrolls, not the window, so the line to cross is measured from the
  // column's own top.
  useEffect(() => {{
    const ids = toc.map((item) => item.id).filter(Boolean);
    if (!ids.length) return;
    const scroller = document.querySelector("main");
    const read = () => {{
      const box = scroller ? scroller.getBoundingClientRect() : null;
      const line = (box ? box.top : 0) + 120;
      let current = ids[0];
      for (const id of ids) {{
        const el = document.getElementById(id);
        if (el && el.getBoundingClientRect().top <= line) current = id;
      }}
      // At the end there is nothing left to scroll, so the last heading on
      // screen is the one being read. Without this a page shorter than the
      // column never advances past its first heading, however far it scrolls.
      const atEnd = scroller && scroller.scrollTop >= scroller.scrollHeight - scroller.clientHeight - 2;
      if (atEnd && box) {{
        for (const id of ids) {{
          const el = document.getElementById(id);
          if (el && el.getBoundingClientRect().top < box.bottom) current = id;
        }}
      }}
      setActiveHeading(current);
    }};
    read();
    const target = scroller || window;
    target.addEventListener("scroll", read, {{ passive: true }});
    window.addEventListener("resize", read);
    return () => {{
      target.removeEventListener("scroll", read);
      window.removeEventListener("resize", read);
    }};
  }}, [page.route_path]);

  // One query reads the chunks its own terms live in. Every path comes from
  // the manifest's own list — never from what the reader typed — so a typed
  // term can only select a chunk that exists, never name a file.
  useEffect(() => {{
    const typed = String(query || "").trim();
    if (!typed) {{
      setResults([]);
      setSearchState("idle");
      return;
    }}
    if (!manifestHref) return;
    let cancelled = false;
    setSearchState("loading");
    const timer = setTimeout(() => {{
      runSearch(manifestHref, typed)
        .then((found) => {{
          if (cancelled) return;
          setResults(found);
          setSearchState("ready");
        }})
        .catch(() => {{
          if (cancelled) return;
          setResults([]);
          setSearchState("error");
        }});
    }}, 120);
    return () => {{
      cancelled = true;
      clearTimeout(timer);
    }};
  }}, [manifestHref, query]);


  return (
    <Page>
      <style>{{DOCS_MARKDOWN_CSS}}</style>
      <div class="min-h-screen bg-white text-slate-900 lg:h-screen lg:overflow-hidden">
        <div class="border-b border-slate-200 px-4 py-3 lg:hidden">
          <a href={{input.site?.home_path || input.site?.deploy_base_path || input.site?.base_path || "/"}} class="text-xs font-semibold uppercase tracking-[0.22em] text-orange-600">{{{fallback_title}}}</a>
          <div class="mt-1 text-lg font-bold tracking-tight">{{page.title || input.site?.title || "Docs"}}</div>
        </div>
        <div class="lg:grid lg:h-screen lg:grid-cols-12">
          <aside class="border-b border-slate-200 px-4 py-4 lg:col-span-3 lg:overflow-y-auto lg:border-b-0 lg:border-r lg:px-5 lg:py-5">
            <div class="text-xs font-semibold uppercase tracking-[0.22em] text-orange-600">{{{fallback_title}}}</div>
              <div class="mt-3">
                <input
                  type="search"
                  value={{query}}
                  onInput={{(event) => setQuery(event.currentTarget.value)}}
                  placeholder="Search the docs"
                  class="h-10 w-full rounded-md border border-slate-300 px-3 text-sm outline-none ring-0 focus:border-orange-500"
                />
              <div class="mt-2 text-[11px] text-slate-500">
                {{query.trim() ? `${{results.length}} result${{results.length === 1 ? "" : "s"}}` : "Type to search headings, keywords, and content"}}
              </div>
            </div>
            {{query.trim() ? (
              <div class="mt-4 border-b border-slate-200 pb-4">
                <ul class="space-y-3">
                  {{results.length === 0 ? (
                    <li class="text-sm text-slate-500">
                      {{searchState === "loading" ? "Loading search index..." : "No matching pages."}}
                    </li>
                  ) : results.map((entry) => (
                    <li>
                      <a href={{entry.href}} class="block">
                        <div class="text-sm font-semibold text-slate-900 hover:text-orange-600">{{entry.title}}</div>
                        {{entry.section ? <div class="mt-0.5 text-[11px] uppercase tracking-[0.16em] text-slate-400">{{entry.section}}</div> : null}}
                        {{entry.snippet?.match ? (
                          <div class="mt-1 text-sm text-slate-600">
                            {{entry.snippet.before}}
                            <mark class="bg-orange-100 text-slate-900">{{entry.snippet.match}}</mark>
                            {{entry.snippet.after}}
                          </div>
                        ) : entry.description ? (
                          <div class="mt-1 text-sm text-slate-600">{{entry.description}}</div>
                        ) : null}}
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
                <Blocks blocks={{page.blocks}} />
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
                    class={{activeHeading === item.id
                      ? "font-semibold text-orange-600"
                      : "text-slate-700 hover:text-slate-900"}}
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

