import { cx, useState } from "zeb/react";

/**
 * CodeBlock — source with syntax colour, a language label, optional line
 * numbers and a copy button. No highlighter library: one small tokenizer
 * with a keyword table per language, run on the server too, so the coloured
 * markup is in the first HTML byte. Colours are theme roles, so a block
 * follows light and dark like everything else.
 *
 * Languages today: tsx · ts · jsx · js · json · python · shell · rust · sql ·
 * sekejap. Add one by adding a row to LANGUAGES.
 *
 *   <CodeBlock language="tsx" title="pages/home.tsx" code={source} lineNumbers />
 */

const JS_KEYWORDS =
  "import export from default function return const let var if else for while do switch case break continue new class extends super this typeof instanceof in of try catch finally throw async await yield static get set null undefined true false as interface type enum implements declare readonly public private protected namespace keyof satisfies";
const PY_KEYWORDS =
  "def class return if elif else for while in not and or is None True False import from as with try except finally raise lambda yield pass break continue global nonlocal assert del async await match case";
const SH_KEYWORDS = "if then else elif fi for while do done case esac in function return exit export local set unset echo cd source sudo";
const RS_KEYWORDS =
  "fn let mut pub use mod struct enum impl trait for in if else match while loop return break continue where as ref move async await dyn type const static crate self Self super unsafe extern true false Some None Ok Err";
const SQL_KEYWORDS =
  "select from where and or not in is null as join left right inner outer on group by order asc desc limit offset insert into values update set delete create table drop alter add primary key unique index if exists default returning with union all distinct having between like case when then else end count sum avg min max true false";
const SEKEJAP_EXTRA = "upsert conflict do nothing returning read only stream subscribe channel notify listen";

const LANGUAGES = {
  tsx: { keywords: JS_KEYWORDS, line: "//", block: ["/*", "*/"], jsx: true, label: "tsx" },
  ts: { keywords: JS_KEYWORDS, line: "//", block: ["/*", "*/"], jsx: false, label: "ts" },
  jsx: { keywords: JS_KEYWORDS, line: "//", block: ["/*", "*/"], jsx: true, label: "jsx" },
  js: { keywords: JS_KEYWORDS, line: "//", block: ["/*", "*/"], jsx: false, label: "js" },
  json: { keywords: "true false null", line: null, block: null, jsx: false, label: "json" },
  python: { keywords: PY_KEYWORDS, line: "#", block: null, jsx: false, label: "python" },
  shell: { keywords: SH_KEYWORDS, line: "#", block: null, jsx: false, label: "shell" },
  rust: { keywords: RS_KEYWORDS, line: "//", block: ["/*", "*/"], jsx: false, label: "rust" },
  sql: { keywords: SQL_KEYWORDS, line: "--", block: ["/*", "*/"], jsx: false, label: "sql", caseless: true },
  sekejap: { keywords: SQL_KEYWORDS + " " + SEKEJAP_EXTRA, line: "--", block: ["/*", "*/"], jsx: false, label: "sekejap ql", caseless: true },
};
const ALIASES = { typescript: "ts", javascript: "js", py: "python", sh: "shell", bash: "shell", zsh: "shell", rs: "rust", sekejapql: "sekejap", ql: "sekejap" };

const TOKEN_CLASSES = {
  comment: "italic text-muted-foreground",
  string: "text-success",
  number: "text-warning",
  keyword: "text-primary",
  tag: "text-info",
  attr: "text-chart-4",
  type: "text-chart-2",
  fn: "text-chart-2",
  variable: "text-warning",
  punct: "text-muted-foreground",
  plain: "",
};

function isWordChar(c) {
  return /[A-Za-z0-9_$]/.test(c);
}

/** Split `code` into { type, text } tokens for `lang`. Pure; runs anywhere. */
export function tokenize(code, lang) {
  const rules = LANGUAGES[ALIASES[lang] ?? lang] ?? LANGUAGES.tsx;
  const keywords = new Set(rules.keywords.split(/\s+/));
  const out = [];
  const push = (type, text) => {
    const last = out[out.length - 1];
    if (last && last.type === type && (type === "plain" || type === "punct")) last.text += text;
    else out.push({ type, text });
  };
  let i = 0;
  let inTag = false;
  const n = code.length;
  while (i < n) {
    const c = code[i];
    const two = code.slice(i, i + 2);
    // comments
    if (rules.line && code.startsWith(rules.line, i) && !(rules.line === "#" && c === "#" && code[i + 1] === "!" && i !== 0)) {
      let j = code.indexOf("\n", i);
      if (j < 0) j = n;
      push("comment", code.slice(i, j));
      i = j;
      continue;
    }
    if (rules.block && code.startsWith(rules.block[0], i)) {
      let j = code.indexOf(rules.block[1], i + 2);
      j = j < 0 ? n : j + rules.block[1].length;
      push("comment", code.slice(i, j));
      i = j;
      continue;
    }
    // strings (with escapes; template literals kept whole)
    if (c === '"' || c === "'" || c === "`") {
      const triple = rules.label === "python" && code.startsWith(c + c + c, i);
      const close = triple ? c + c + c : c;
      let j = i + close.length;
      while (j < n) {
        if (code[j] === "\\") { j += 2; continue; }
        if (code.startsWith(close, j)) { j += close.length; break; }
        j += 1;
      }
      push("string", code.slice(i, j));
      i = j;
      continue;
    }
    // jsx tags: `<Name`, `</Name`, and the closing `>` / `/>`
    if (rules.jsx && c === "<" && /[A-Za-z/>]/.test(code[i + 1] ?? "")) {
      let j = i + 1;
      if (code[j] === "/") j += 1;
      const start = j;
      while (j < n && /[A-Za-z0-9_.:-]/.test(code[j])) j += 1;
      push("punct", code.slice(i, start));
      if (j > start) push("tag", code.slice(start, j));
      inTag = true;
      i = j;
      continue;
    }
    if (inTag && (c === ">" || two === "/>")) {
      push("punct", two === "/>" ? two : c);
      i += two === "/>" ? 2 : 1;
      inTag = false;
      continue;
    }
    // shell variables, sql parameters, rust lifetimes/macros, python decorators
    if ((rules.label === "shell" && c === "$") || ((rules.label === "sql" || rules.label === "sekejap ql") && c === "$" && /\d/.test(code[i + 1] ?? ""))) {
      let j = i + 1;
      if (code[j] === "{") { j = code.indexOf("}", j); j = j < 0 ? n : j + 1; }
      else while (j < n && isWordChar(code[j])) j += 1;
      push("variable", code.slice(i, j));
      i = j;
      continue;
    }
    if (rules.label === "python" && c === "@") {
      let j = i + 1;
      while (j < n && /[A-Za-z0-9_.]/.test(code[j])) j += 1;
      push("fn", code.slice(i, j));
      i = j;
      continue;
    }
    if (rules.label === "shell" && c === "-" && /[A-Za-z-]/.test(code[i + 1] ?? "") && (i === 0 || /\s/.test(code[i - 1]))) {
      let j = i + 1;
      while (j < n && /[A-Za-z0-9-]/.test(code[j])) j += 1;
      push("attr", code.slice(i, j));
      i = j;
      continue;
    }
    // numbers
    if (/[0-9]/.test(c) && !(i > 0 && isWordChar(code[i - 1]))) {
      let j = i;
      while (j < n && /[0-9a-fA-FxX._]/.test(code[j])) j += 1;
      push("number", code.slice(i, j));
      i = j;
      continue;
    }
    // words: keyword / attribute (inside a tag) / type / function call / plain
    if (/[A-Za-z_$]/.test(c)) {
      let j = i;
      while (j < n && (isWordChar(code[j]) || (inTag && code[j] === "-"))) j += 1;
      const word = code.slice(i, j);
      const key = rules.caseless ? word.toLowerCase() : word;
      let type = "plain";
      if (inTag) type = "attr";
      else if (keywords.has(key)) type = "keyword";
      else if (rules.label === "rust" && code[j] === "!") type = "fn";
      else if (/^[A-Z]/.test(word) && rules.label !== "shell") type = "type";
      else if (code[j] === "(") type = "fn";
      push(type, word);
      i = j;
      continue;
    }
    // punctuation and whitespace
    if (/\s/.test(c)) { push("plain", c); i += 1; continue; }
    push("punct", c);
    i += 1;
  }
  return out;
}

function CopyButton({ code }) {
  const [copied, setCopied] = useState(false);
  return (
    <button
      type="button"
      aria-label="Copy code"
      onClick={() => {
        if (typeof navigator === "undefined" || !navigator.clipboard) return;
        navigator.clipboard.writeText(code).then(() => {
          setCopied(true);
          setTimeout(() => setCopied(false), 1500);
        });
      }}
      className="inline-flex h-7 items-center gap-1.5 rounded-md border border-border bg-background px-2 font-mono text-[0.68rem] text-muted-foreground transition-colors hover:bg-accent hover:text-accent-foreground"
    >
      {copied ? (
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" className="size-3"><path d="M20 6L9 17l-5-5" /></svg>
      ) : (
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" className="size-3"><rect x="9" y="9" width="13" height="13" rx="2" /><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" /></svg>
      )}
      {copied ? "Copied" : "Copy"}
    </button>
  );
}

export function CodeBlock({ code = "", language = "tsx", title, lineNumbers = false, copy = true, maxHeight, className, ...props }) {
  const rules = LANGUAGES[ALIASES[language] ?? language] ?? LANGUAGES.tsx;
  const source = String(code).replace(/\n$/, "");
  const lines = source.split("\n");
  const tokensByLine = [];
  // Tokenize the whole source once, then split tokens at newlines so a
  // multi-line comment or string keeps its colour on every line.
  let line = [];
  for (const t of tokenize(source, language)) {
    const parts = t.text.split("\n");
    parts.forEach((part, k) => {
      if (k > 0) { tokensByLine.push(line); line = []; }
      if (part) line.push({ type: t.type, text: part });
    });
  }
  tokensByLine.push(line);

  return (
    <div data-slot="code-block" className={cx("overflow-hidden rounded-lg border border-border bg-muted text-sm", className)} {...props}>
      <div className="flex h-9 items-center gap-3 border-b border-border px-3">
        {title ? <span className="truncate font-mono text-xs text-foreground">{title}</span> : null}
        <span className="rounded bg-background px-1.5 py-0.5 font-mono text-[0.62rem] uppercase tracking-wider text-muted-foreground">{rules.label}</span>
        <span className="flex-1" />
        {copy ? <CopyButton code={source} /> : null}
      </div>
      <pre className="overflow-auto px-4 py-3 font-mono text-[0.8rem] leading-6" style={maxHeight ? { maxHeight } : undefined}>
        <code>
          {tokensByLine.map((toks, idx) => (
            <span key={idx} className="block">
              {lineNumbers ? <span className="mr-4 inline-block w-6 select-none text-right text-muted-foreground/60">{idx + 1}</span> : null}
              {toks.length === 0 ? "​" : toks.map((t, k) => (
                <span key={k} className={TOKEN_CLASSES[t.type] ?? ""}>{t.text}</span>
              ))}
            </span>
          ))}
        </code>
      </pre>
    </div>
  );
}

export default CodeBlock;
