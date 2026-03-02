//! Reference RWE engine implementation.
//!
//! Features included in this engine:
//!
//! - inline `<style>` extraction from TSX/page markup
//! - optional component expansion from PascalCase tags (`<Button />`)
//! - SSR placeholder interpolation (`{{input.*}}`, `{{ctx.*}}`)
//! - reactive binding scan (`@click`, `z-text`, `z-model`, etc.)
//! - lightweight client runtime injection

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use crate::language::{
    COMPILE_TARGET_FRONTEND, CompileOptions, LanguageEngine, ModuleSource, SourceKind,
};
use crate::rwe::class_notation::{
    collect_untyped_placeholders_from_class_value, resolve_typed_class_macros,
};
use crate::rwe::interface::ReactiveWebEngine;
use crate::rwe::model::{
    CompiledScript, CompiledScriptScope, CompiledTemplate, ComponentOptions, ReactiveBinding,
    ReactiveMode, ReactiveWebDiagnostic, ReactiveWebError, ReactiveWebOptions, RenderContext,
    RenderOutput, RuntimeBundle, RuntimeMode, TemplateSource,
};
use crate::rwe::processors;
use crate::rwe::processors::tailwind::collect_tw_variants;
use crate::rwe::tsx_frontend::{
    looks_like_tsx_source, lower_tsx_source_to_parts, resolve_component_imports,
};

/// Default RWE engine used for Zebflow scaffolding and integration tests.
#[derive(Default)]
pub struct NoopReactiveWebEngine;

impl NoopReactiveWebEngine {
    /// Compiles template parts directly without requiring `.tsx` block extraction.
    ///
    /// This is useful for alternate authoring frontends (for example TSX) that
    /// already parsed source and can provide:
    ///
    /// - HTML template body
    /// - optional control script source
    ///
    /// The method still runs the same RWE compile stages:
    ///
    /// - compile processors (Tailwind/Markdown/etc.)
    /// - reactive binding scan
    /// - language parse/compile for control script
    /// - dynamic tailwind diagnostics
    pub fn compile_direct_parts(
        &self,
        template_id: &str,
        source_path: Option<PathBuf>,
        html_template: &str,
        control_script_source: Option<&str>,
        options: &ReactiveWebOptions,
        language: &dyn LanguageEngine,
    ) -> Result<CompiledTemplate, ReactiveWebError> {
        let mut diagnostics = Vec::new();

        let mut html = inject_template_styles(html_template, options, &mut diagnostics)?;
        html = processors::apply_compile_processors(&html, options, &mut diagnostics);
        html = enforce_resource_allow_list(&html, options, &mut diagnostics);

        let reactive_bindings = if options.reactive_mode == ReactiveMode::Bindings {
            collect_reactive_bindings(&html)
        } else {
            Vec::new()
        };

        let control_script_source = control_script_source
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(std::string::ToString::to_string);

        let compiled_logic = if let Some(script_source) = &control_script_source {
            let source = ModuleSource {
                id: format!("tsx:{template_id}"),
                source_path,
                kind: SourceKind::Tsx,
                code: script_source.clone(),
            };

            let ir = language.parse(&source).map_err(|err| {
                ReactiveWebError::new(
                    "RWE_LANG_PARSE",
                    format!("failed parsing template logic '{template_id}': {err}"),
                )
            })?;

            let compile_options = CompileOptions {
                target: COMPILE_TARGET_FRONTEND.to_string(),
                optimize_level: if options.minify_html { 2 } else { 1 },
                emit_trace_hints: true,
            };

            let compiled_logic = language.compile(&ir, &compile_options).map_err(|err| {
                ReactiveWebError::new(
                    "RWE_LANG_COMPILE",
                    format!("failed compiling template logic '{template_id}': {err}"),
                )
            })?;
            Some(compiled_logic)
        } else {
            None
        };

        let tw_variants = collect_tw_variants(&html);
        let tw_variant_exact_tokens: Vec<String> =
            tw_variants.exact_tokens.iter().cloned().collect();
        let tw_variant_patterns: Vec<String> =
            tw_variants.wildcard_patterns.iter().cloned().collect();

        let dynamic_class_placeholders = collect_dynamic_class_placeholders(&html);
        let missing_dynamic_contract =
            !dynamic_class_placeholders.is_empty() && tw_variants.is_empty();
        let needs_runtime_tailwind_rebuild =
            missing_dynamic_contract || !tw_variant_patterns.is_empty();
        if missing_dynamic_contract {
            for placeholder in dynamic_class_placeholders {
                diagnostics.push(ReactiveWebDiagnostic {
                    code: "RWE_TAILWIND_DYNAMIC_CLASS_WARN".to_string(),
                    message: format!(
                        "dynamic class placeholder '{placeholder}' may not be fully traceable at compile time; add `tw-variants` hints on parent/element scope or use inline style for predictable CSS output"
                    ),
                });
            }
        }
        if !tw_variant_patterns.is_empty() {
            diagnostics.push(ReactiveWebDiagnostic {
                code: "RWE_TAILWIND_VARIANTS_RUNTIME".to_string(),
                message: format!(
                    "dynamic tailwind runtime enabled for wildcard patterns: {}",
                    tw_variant_patterns.join(", ")
                ),
            });
        }

        Ok(CompiledTemplate {
            engine_id: self.id().to_string(),
            template_id: template_id.to_string(),
            html_ir: html,
            control_script_source,
            compiled_logic,
            runtime_bundle: runtime_bundle(&options.runtime_mode),
            reactive_bindings,
            diagnostics,
            needs_runtime_tailwind_rebuild,
            tailwind_variant_exact_tokens: tw_variant_exact_tokens,
            tailwind_variant_patterns: tw_variant_patterns,
            options: options.clone(),
        })
    }
}

const RWE_RUNTIME_PROD_JS: &str = r#"
(function(w){
  const R = w.__ZEBFLOW_RWE__ || {};
  R.mode = "prod";
  R.app = {};
  R.state = {};
  R.memo = {};
  R.bootstrap = {};
  R.input = {};
  R._forBlocks = [];
  R._effectsOnce = new Set();
  R._bound = false;

  function clone(v){ return JSON.parse(JSON.stringify(v)); }
  function segs(path){
    const src = String(path || "");
    const out = [];
    let buf = "";
    for (let i = 0; i < src.length; i++) {
      const ch = src[i];
      if (ch === ".") {
        if (buf) out.push(buf);
        buf = "";
        continue;
      }
      if (ch === "[") {
        if (buf) out.push(buf);
        buf = "";
        const end = src.indexOf("]", i + 1);
        if (end === -1) break;
        let inner = src.slice(i + 1, end).trim();
        if ((inner.startsWith("'") && inner.endsWith("'")) || (inner.startsWith('"') && inner.endsWith('"'))) {
          inner = inner.slice(1, -1);
        }
        if (inner) out.push(inner);
        i = end;
        continue;
      }
      buf += ch;
    }
    if (buf) out.push(buf);
    return out;
  }
  function readPathFrom(root, path){
    const s = segs(path);
    let cur = root;
    for (const p of s) { if (cur == null) return undefined; cur = cur[p]; }
    return cur;
  }
  function readPath(path){
    const raw = String(path || "");
    if (raw === "input") return R.input;
    if (raw.startsWith("input.")) return readPathFrom(R.input, raw.slice("input.".length));
    if (raw === "ctx") return (R.bootstrap && R.bootstrap.metadata) || {};
    if (raw.startsWith("ctx.")) return readPathFrom((R.bootstrap && R.bootstrap.metadata) || {}, raw.slice("ctx.".length));
    return readPathFrom(R.state, raw);
  }
  function truthy(v){
    if (v == null) return false;
    if (typeof v === "boolean") return v;
    if (typeof v === "number") return v !== 0 && !Number.isNaN(v);
    if (typeof v === "string") return v.length > 0;
    return true;
  }
  function exprTokens(src){
    const out = [];
    const s = String(src || "");
    let i = 0;
    while (i < s.length) {
      const ch = s[i];
      if (/\s/.test(ch)) { i++; continue; }
      if (ch === "&" && s[i + 1] === "&") { out.push("&&"); i += 2; continue; }
      if (ch === "|" && s[i + 1] === "|") { out.push("||"); i += 2; continue; }
      if (ch === "!") { out.push("!"); i += 1; continue; }
      if (ch === "(" || ch === ")") { out.push(ch); i += 1; continue; }
      let j = i;
      while (j < s.length) {
        const cj = s[j];
        if (/\s/.test(cj)) break;
        if (cj === "(" || cj === ")" || cj === "!") break;
        if ((cj === "&" && s[j + 1] === "&") || (cj === "|" && s[j + 1] === "|")) break;
        j++;
      }
      out.push(s.slice(i, j));
      i = j;
    }
    return out;
  }
  function evalBindingExpr(expr){
    const tokens = exprTokens(expr);
    if (!tokens.length) return undefined;
    let idx = 0;
    function parseOr(){
      let left = parseAnd();
      while (tokens[idx] === "||") {
        idx++;
        left = truthy(left) || truthy(parseAnd());
      }
      return left;
    }
    function parseAnd(){
      let left = parseUnary();
      while (tokens[idx] === "&&") {
        idx++;
        left = truthy(left) && truthy(parseUnary());
      }
      return left;
    }
    function parseUnary(){
      if (tokens[idx] === "!") {
        idx++;
        return !truthy(parseUnary());
      }
      return parsePrimary();
    }
    function parsePrimary(){
      const token = tokens[idx++];
      if (token == null) return undefined;
      if (token === "(") {
        const value = parseOr();
        if (tokens[idx] === ")") idx++;
        return value;
      }
      if (token === "true") return true;
      if (token === "false") return false;
      if (token === "null" || token === "undefined") return undefined;
      return readPath(token);
    }
    const value = parseOr();
    return idx === tokens.length ? value : undefined;
  }
  function evalBindingTruthy(expr){
    return truthy(evalBindingExpr(expr));
  }
  function writePath(path, value){
    const s = segs(path);
    if (!s.length) return;
    let cur = R.state;
    for (let i = 0; i < s.length - 1; i++) {
      if (!cur[s[i]] || typeof cur[s[i]] !== "object") cur[s[i]] = {};
      cur = cur[s[i]];
    }
    cur[s[s.length - 1]] = value;
  }
  function ctx(extra){
    return Object.assign({
      state: R.state,
      memo: R.memo,
      get: readPath,
      set: writePath,
      update: function(path, fn){ writePath(path, fn(readPath(path))); }
    }, extra || {});
  }
  function runMemos(){
    const defs = (R.app && R.app.memo) || {};
    for (const k of Object.keys(defs)) {
      const fn = defs[k];
      if (typeof fn !== "function") continue;
      R.memo[k] = fn(ctx({ memo: R.memo }));
    }
  }
  function depHit(deps, changedPath){
    if (!Array.isArray(deps) || deps.length === 0) return true;
    if (!changedPath) return false;
    return deps.some((d) => String(changedPath).startsWith(String(d)) || String(d).startsWith(String(changedPath)));
  }
  function nearestHydrateRoot(el){
    let cur = el;
    while (cur && cur !== document.body) {
      if (cur.getAttribute && cur.getAttribute("data-rwe-hydrate")) return cur;
      cur = cur.parentElement;
    }
    return null;
  }
  function isNodeHydrated(el){
    const root = nearestHydrateRoot(el);
    if (!root) return true;
    return root.getAttribute("data-rwe-hydrate-active") === "1";
  }
  function activateHydrateRoot(root){
    if (!root || !root.setAttribute) return;
    root.setAttribute("data-rwe-hydrate-active", "1");
    renderForBlocks(null);
    flushBindings();
  }
  function initHydrationIslands(){
    if (!document || !document.body) return;
    const islands = document.querySelectorAll("[hydrate]");
    for (const el of islands) {
      const mode = String(el.getAttribute("hydrate") || "off").trim().toLowerCase();
      el.setAttribute("data-rwe-hydrate", mode);
      if (mode === "immediate") {
        el.setAttribute("data-rwe-hydrate-active", "1");
        continue;
      }
      el.setAttribute("data-rwe-hydrate-active", "0");
      if (mode === "idle") {
        const fn = function(){ activateHydrateRoot(el); };
        if (typeof w.requestIdleCallback === "function") w.requestIdleCallback(fn);
        else setTimeout(fn, 1);
        continue;
      }
      if (mode === "visible") {
        if (typeof w.IntersectionObserver === "function") {
          const io = new w.IntersectionObserver(function(entries){
            for (const e of entries) {
              if (e.isIntersecting) {
                activateHydrateRoot(el);
                io.disconnect();
                break;
              }
            }
          });
          io.observe(el);
        } else {
          activateHydrateRoot(el);
        }
        continue;
      }
      if (mode === "interaction") {
        const onWake = function(){
          activateHydrateRoot(el);
          el.removeEventListener("pointerdown", onWake, true);
          el.removeEventListener("focusin", onWake, true);
          el.removeEventListener("keydown", onWake, true);
        };
        el.addEventListener("pointerdown", onWake, true);
        el.addEventListener("focusin", onWake, true);
        el.addEventListener("keydown", onWake, true);
        continue;
      }
    }
  }
  function stripAttrValue(src, name){
    const re = new RegExp("\\s+" + name + "\\s*=\\s*\"[^\"]*\"", "gi");
    return String(src).replace(re, "");
  }
  function parseForExpr(expr){
    const m = String(expr || "").match(/^\s*([A-Za-z_$][\w$]*)\s+in\s+([A-Za-z0-9_.$]+)\s*$/);
    if (!m) return null;
    return { itemVar: m[1], listPath: m[2] };
  }
  function readLocalPath(root, path){
    const s = segs(path);
    let cur = root;
    for (const p of s) { if (cur == null) return undefined; cur = cur[p]; }
    return cur;
  }
  function renderLoopTemplate(src, itemVar, item, idx){
    return String(src).replace(/\{\{\s*([^}]+)\s*\}\}/g, function(_, exprRaw){
      const expr = String(exprRaw || "").trim();
      if (expr === "$index") return String(idx);
      if (expr === itemVar) return item == null ? "" : String(item);
      if (expr.startsWith(itemVar + ".")) {
        const v = readLocalPath(item, expr.slice(itemVar.length + 1));
        return v == null ? "" : String(v);
      }
      return "{{" + exprRaw + "}}";
    });
  }
  function htmlHash(src){
    let h = 2166136261 >>> 0;
    for (let i = 0; i < src.length; i++) {
      h ^= src.charCodeAt(i);
      h = Math.imul(h, 16777619);
    }
    return String(h >>> 0);
  }
  function initForBlocks(){
    if (!document || !document.body) return;
    const nodes = Array.from(document.querySelectorAll("[z-for]"));
    for (let i = 0; i < nodes.length; i++) {
      const el = nodes[i];
      if (el.__rweForInit) continue;
      el.__rweForInit = true;
      const parsed = parseForExpr(el.getAttribute("z-for"));
      if (!parsed) continue;
      const blockId = String(el.getAttribute("data-rwe-for-id") || ("rwefor_" + i));
      const keyExpr = String(el.getAttribute("z-key") || "").trim();

      let scan = el.nextSibling;
      while (scan) {
        if (scan.nodeType === 1 && scan.getAttribute && scan.getAttribute("data-rwe-for-seeded") === blockId) {
          const next = scan.nextSibling;
          scan.remove();
          scan = next;
          continue;
        }
        if (scan.nodeType === 3 && String(scan.textContent || "").trim() === "") {
          const next = scan.nextSibling;
          scan.remove();
          scan = next;
          continue;
        }
        break;
      }

      const templateEl = el.cloneNode(true);
      templateEl.removeAttribute("z-for");
      templateEl.removeAttribute("z-key");
      templateEl.removeAttribute("data-rwe-for-id");
      templateEl.removeAttribute("data-rwe-for-template");
      templateEl.removeAttribute("data-rwe-for-seeded");
      templateEl.removeAttribute("data-rwe-for-key");
      templateEl.removeAttribute("hidden");
      templateEl.removeAttribute("style");
      const templateSrc = templateEl.outerHTML;

      const parent = el.parentNode;
      if (!parent) continue;
      const start = document.createComment("rwe-for-start:" + blockId);
      const end = document.createComment("rwe-for-end:" + blockId);
      parent.insertBefore(start, el);
      parent.insertBefore(end, el.nextSibling);
      el.remove();

      R._forBlocks.push({
        id: blockId,
        start: start,
        end: end,
        itemVar: parsed.itemVar,
        listPath: parsed.listPath,
        keyExpr: keyExpr,
        templateSrc: templateSrc,
        nodeByKey: new Map(),
        lastHashes: new Map()
      });
    }
  }
  function renderForBlocks(changedPath){
    if (!Array.isArray(R._forBlocks) || R._forBlocks.length === 0) return;
    for (const block of R._forBlocks) {
      if (changedPath) {
        const cp = String(changedPath);
        if (!cp.startsWith(block.listPath) && !String(block.listPath).startsWith(cp)) continue;
      }
      const parentNode = block.start && block.start.parentNode;
      if (!parentNode) continue;
      const probe = block.start.parentElement || block.end.parentElement;
      if (probe && !isNodeHydrated(probe)) continue;

      const rawList = readPath(block.listPath);
      const list = Array.isArray(rawList) ? rawList : [];
      const frag = document.createDocumentFragment();
      const nextNodeByKey = new Map();
      const nextHashes = new Map();

      for (let idx = 0; idx < list.length; idx++) {
        const item = list[idx];
        let key = String(idx);
        if (block.keyExpr) {
          if (block.keyExpr === block.itemVar) key = String(item);
          else if (block.keyExpr.startsWith(block.itemVar + ".")) {
            const kv = readLocalPath(item, block.keyExpr.slice(block.itemVar.length + 1));
            key = kv == null ? String(idx) : String(kv);
          }
        }
        const html = renderLoopTemplate(block.templateSrc, block.itemVar, item, idx);
        const hash = htmlHash(html);
        let node = block.nodeByKey.get(key);
        if (!node || block.lastHashes.get(key) !== hash) {
          const t = document.createElement("template");
          t.innerHTML = html.trim();
          node = t.content.firstElementChild || t.content.firstChild;
        }
        if (!node) continue;
        if (node.nodeType === 1 && node.setAttribute) {
          node.setAttribute("data-rwe-for-key", key);
          node.setAttribute("data-rwe-for-hash", hash);
        }
        frag.appendChild(node);
        nextNodeByKey.set(key, node);
        nextHashes.set(key, hash);
      }

      let cur = block.start.nextSibling;
      while (cur && cur !== block.end) {
        const next = cur.nextSibling;
        cur.remove();
        cur = next;
      }
      parentNode.insertBefore(frag, block.end);
      block.nodeByKey = nextNodeByKey;
      block.lastHashes = nextHashes;
    }
  }
  function runEffects(changedPath, phase){
    const defs = (R.app && R.app.effect) || {};
    for (const k of Object.keys(defs)) {
      const def = defs[k];
      if (typeof def === "function") {
        if (phase === "mount" || phase === "action") def(ctx({ changedPath, phase }));
        continue;
      }
      if (!def || typeof def !== "object" || typeof def.run !== "function") continue;
      if (def.once && R._effectsOnce.has(k)) continue;
      if (phase === "mount" && !def.immediate) continue;
      if (phase === "action" && !depHit(def.deps, changedPath)) continue;
      def.run(ctx({ changedPath, phase }));
      if (def.once) R._effectsOnce.add(k);
    }
  }
  function flushBindings(){
    if (!document || !document.body) return;
    const textNodes = document.querySelectorAll("[z-text]");
    for (const el of textNodes) {
      if (!isNodeHydrated(el)) continue;
      const p = el.getAttribute("z-text");
      const v = readPath(p || "");
      el.textContent = v == null ? "" : String(v);
    }
    const modelNodes = document.querySelectorAll("[z-model]");
    for (const el of modelNodes) {
      if (!isNodeHydrated(el)) continue;
      const p = el.getAttribute("z-model");
      const v = readPath(p || "");
      if (el.value !== String(v == null ? "" : v)) el.value = String(v == null ? "" : v);
    }
    const classNodes = document.querySelectorAll("[z-attr\\:class]");
    for (const el of classNodes) {
      if (!isNodeHydrated(el)) continue;
      const p = el.getAttribute("z-attr:class");
      const v = readPath(p || "");
      if (typeof v === "string") el.setAttribute("class", v);
    }
    const showNodes = document.querySelectorAll("[z-show]");
    for (const el of showNodes) {
      if (!isNodeHydrated(el)) continue;
      const p = el.getAttribute("z-show");
      const v = evalBindingTruthy(p || "");
      if (v) {
        el.removeAttribute("hidden");
      } else {
        el.setAttribute("hidden", "");
      }
    }
    const hideNodes = document.querySelectorAll("[z-hide]");
    for (const el of hideNodes) {
      if (!isNodeHydrated(el)) continue;
      const p = el.getAttribute("z-hide");
      const v = evalBindingTruthy(p || "");
      if (v) {
        el.setAttribute("hidden", "");
      } else {
        el.removeAttribute("hidden");
      }
    }
  }
  function bindDom(){
    if (R._bound || !document || !document.body) return;
    R._bound = true;
    document.addEventListener("click", function(ev){
      let el = ev.target;
      while (el && el !== document.body) {
        const act = el.getAttribute && (el.getAttribute("@click") || el.getAttribute("on:click"));
        if (act) {
          if (!isNodeHydrated(el)) {
            const root = nearestHydrateRoot(el);
            if (root) activateHydrateRoot(root);
            if (!isNodeHydrated(el)) break;
          }
          R.dispatch(act, { event: "click" });
          break;
        }
        el = el.parentElement;
      }
    });
    document.addEventListener("input", function(ev){
      const el = ev.target;
      if (!el || !el.getAttribute) return;
      const path = el.getAttribute("z-model");
      if (!path) return;
      if (!isNodeHydrated(el)) return;
      writePath(path, el.value);
      runMemos();
      runEffects(path, "action");
      renderForBlocks(path);
      flushBindings();
    });
  }

  R.mount = function(app, bootstrap){
    R.app = app || {};
    R.bootstrap = bootstrap || {};
    R.input = (R.bootstrap && R.bootstrap.input) || {};
    R.state = clone((R.app && R.app.state) || {});
    R.memo = {};
    initHydrationIslands();
    initForBlocks();
    runMemos();
    runEffects(null, "mount");
    renderForBlocks(null);
    flushBindings();
    bindDom();
    return { ok: true, state: R.state, memo: R.memo, bootstrap: bootstrap || {} };
  };

  R.dispatch = function(actionName, payload){
    const actions = (R.app && R.app.actions) || {};
    const fn = actions[actionName];
    if (typeof fn !== "function") return { ok: false, error: "action_not_found" };
    const changedPath = fn(ctx({ payload: payload || {} }), payload || {});
    runMemos();
    runEffects(changedPath, "action");
    renderForBlocks(changedPath);
    flushBindings();
    return { ok: true, state: R.state, memo: R.memo };
  };

  w.__ZEBFLOW_RWE__ = R;
})(window);
"#;

const RWE_RUNTIME_DEV_JS: &str = r#"
window.__ZEBFLOW_RWE__ = window.__ZEBFLOW_RWE__ || {};
window.__ZEBFLOW_RWE__.mode = "dev";
window.__ZEBFLOW_RWE__.debug = function(msg){ console.debug("[RWE][dev]", msg); };
"#;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ControlScriptKind {
    ControlScript,
    ApplicationZebflowJson,
}

#[derive(Clone, Debug)]
struct ControlScript {
    kind: ControlScriptKind,
    body: String,
}

#[derive(Clone, Debug, Default)]
struct PreparedControlScript {
    source: Option<String>,
}

#[derive(Clone, Debug, Default)]
struct PreparedDocument {
    html: String,
    control_scripts: Vec<ControlScript>,
    inline_styles: Vec<String>,
}

struct SsrRenderResult {
    html: String,
    replacements: usize,
}

struct JForSsrResult {
    html: String,
    loop_count: usize,
    seeded_items: usize,
}

impl ReactiveWebEngine for NoopReactiveWebEngine {
    fn id(&self) -> &'static str {
        "rwe.noop"
    }

    fn compile_template(
        &self,
        template: &TemplateSource,
        language: &dyn LanguageEngine,
        options: &ReactiveWebOptions,
    ) -> Result<CompiledTemplate, ReactiveWebError> {
        if looks_like_tsx_source(&template.markup, template.source_path.as_deref()) {
            let mut diagnostics = Vec::new();
            let mut compile_components = options.components.clone();
            if let (Some(template_root), Some(source_path)) = (
                options.templates.template_root.as_deref(),
                template.source_path.as_deref(),
            ) {
                let imported =
                    resolve_component_imports(&template.markup, source_path, template_root)
                        .map_err(|err| {
                            ReactiveWebError::new(
                                "RWE_TEMPLATE_IMPORTS",
                                format!(
                                    "failed resolving TSX imports for '{}': {err}",
                                    template.id
                                ),
                            )
                        })?;
                for (name, markup) in imported.registry {
                    compile_components.registry.insert(name, markup);
                }
            }
            let lowered = lower_tsx_source_to_parts(&template.markup).map_err(|err| {
                ReactiveWebError::new(
                    "RWE_TSX_LOWER",
                    format!("failed lowering tsx source for '{}': {err}", template.id),
                )
            })?;
            let expanded_html = expand_registered_components(
                &lowered.html_template,
                &compile_components,
                &mut diagnostics,
            )
            .map_err(|err| {
                ReactiveWebError::new(
                    "RWE_COMPONENTS",
                    format!(
                        "failed expanding component registry for '{}': {err}",
                        template.id
                    ),
                )
            })?;

            let mut compiled = self.compile_direct_parts(
                &template.id,
                template.source_path.clone(),
                &expanded_html,
                lowered.control_script_source.as_deref(),
                options,
                language,
            )?;
            compiled.diagnostics.extend(diagnostics);
            return Ok(compiled);
        }

        let mut diagnostics = Vec::new();
        let expanded_markup =
            expand_registered_components(&template.markup, &options.components, &mut diagnostics)
                .map_err(|err| {
                ReactiveWebError::new(
                    "RWE_COMPONENTS",
                    format!(
                        "failed expanding component registry for '{}': {err}",
                        template.id
                    ),
                )
            })?;
        let mut doc = prepare_document(&expanded_markup, &mut diagnostics);
        let mut html = inject_template_styles(&doc.html, options, &mut diagnostics)?;
        html = processors::apply_compile_processors(&html, options, &mut diagnostics);
        html = inject_inline_styles(&html, &doc.inline_styles);
        html = enforce_resource_allow_list(&html, options, &mut diagnostics);

        let reactive_bindings = if options.reactive_mode == ReactiveMode::Bindings {
            collect_reactive_bindings(&html)
        } else {
            Vec::new()
        };

        let prepared_script = build_control_script_source(&doc.control_scripts, &mut diagnostics)
            .map_err(|err| {
            ReactiveWebError::new(
                "RWE_SCRIPT_SOURCE",
                format!(
                    "failed preparing control script source for '{}': {err}",
                    template.id
                ),
            )
        })?;

        let control_script_source = prepared_script.source.clone();
        let compiled_logic = if let Some(script_source) = prepared_script.source {
            let source = ModuleSource {
                id: format!("tsx:{}", template.id),
                source_path: template.source_path.clone(),
                kind: SourceKind::Tsx,
                code: script_source,
            };

            let ir = language.parse(&source).map_err(|err| {
                ReactiveWebError::new(
                    "RWE_LANG_PARSE",
                    format!("failed parsing template logic '{}': {err}", template.id),
                )
            })?;

            let compile_options = CompileOptions {
                target: COMPILE_TARGET_FRONTEND.to_string(),
                optimize_level: if options.minify_html { 2 } else { 1 },
                emit_trace_hints: true,
            };

            let compiled_logic = language.compile(&ir, &compile_options).map_err(|err| {
                ReactiveWebError::new(
                    "RWE_LANG_COMPILE",
                    format!("failed compiling template logic '{}': {err}", template.id),
                )
            })?;
            Some(compiled_logic)
        } else {
            None
        };

        doc.html = html;
        let tw_variants = collect_tw_variants(&doc.html);
        let tw_variant_exact_tokens: Vec<String> =
            tw_variants.exact_tokens.iter().cloned().collect();
        let tw_variant_patterns: Vec<String> =
            tw_variants.wildcard_patterns.iter().cloned().collect();

        let dynamic_class_placeholders = collect_dynamic_class_placeholders(&doc.html);
        let missing_dynamic_contract =
            !dynamic_class_placeholders.is_empty() && tw_variants.is_empty();
        let needs_runtime_tailwind_rebuild =
            missing_dynamic_contract || !tw_variant_patterns.is_empty();
        if missing_dynamic_contract {
            for placeholder in dynamic_class_placeholders {
                diagnostics.push(ReactiveWebDiagnostic {
                    code: "RWE_TAILWIND_DYNAMIC_CLASS_WARN".to_string(),
                    message: format!(
                        "dynamic class placeholder '{placeholder}' may not be fully traceable at compile time; add `tw-variants` hints on parent/element scope or use inline style for predictable CSS output"
                    ),
                });
            }
        }
        if !tw_variant_patterns.is_empty() {
            diagnostics.push(ReactiveWebDiagnostic {
                code: "RWE_TAILWIND_VARIANTS_RUNTIME".to_string(),
                message: format!(
                    "dynamic tailwind runtime enabled for wildcard patterns: {}",
                    tw_variant_patterns.join(", ")
                ),
            });
        }
        Ok(CompiledTemplate {
            engine_id: self.id().to_string(),
            template_id: template.id.clone(),
            html_ir: doc.html,
            control_script_source,
            compiled_logic,
            runtime_bundle: runtime_bundle(&options.runtime_mode),
            reactive_bindings,
            diagnostics,
            needs_runtime_tailwind_rebuild,
            tailwind_variant_exact_tokens: tw_variant_exact_tokens,
            tailwind_variant_patterns: tw_variant_patterns,
            options: options.clone(),
        })
    }

    fn render(
        &self,
        compiled: &CompiledTemplate,
        state: Value,
        _language: &dyn LanguageEngine,
        ctx: &RenderContext,
    ) -> Result<RenderOutput, ReactiveWebError> {
        let mut trace = vec![
            format!("engine={}", self.id()),
            format!("route={}", ctx.route),
            format!("request_id={}", ctx.request_id),
        ];

        let mut metadata = json!({
            "route": ctx.route,
            "requestId": ctx.request_id,
            "requestMetadata": ctx.metadata,
            "rwe": {
                "runtimeMode": match compiled.options.runtime_mode {
                    RuntimeMode::Prod => "prod",
                    RuntimeMode::Dev => "dev",
                }
            }
        });
        if let Some(patch) = &compiled.options.language.run_patch {
            metadata["languageRunPatch"] = patch.clone();
            metadata["denoSandboxRunPatch"] = patch.clone();
        }
        let hydration_payload = json!({
            "input": state.clone(),
            "metadata": metadata,
        });

        let ssr_scope = build_ssr_scope(&state, ctx);
        let for_result = apply_ssr_for_loops(&compiled.html_ir, &ssr_scope);
        if for_result.loop_count > 0 {
            trace.push(format!(
                "ssr_for_loops={} seeded_items={}",
                for_result.loop_count, for_result.seeded_items
            ));
        }

        let visibility_result = apply_ssr_visibility(&for_result.html, &ssr_scope);
        if visibility_result.replacements > 0 {
            trace.push(format!(
                "ssr_visibility_updates={}",
                visibility_result.replacements
            ));
        }

        let ssr_result = apply_ssr_placeholders(&visibility_result.html, &ssr_scope);
        if ssr_result.replacements > 0 {
            trace.push(format!("ssr_replacements={}", ssr_result.replacements));
        }

        let runtime_source = compiled.runtime_bundle.source.clone();
        let mut page_source = String::new();
        if !compiled.reactive_bindings.is_empty() {
            page_source.push_str(&reactive_bindings_bootstrap_snippet(
                &compiled.reactive_bindings,
            ));
        }
        if let Some(source) = &compiled.control_script_source {
            if !source.trim().is_empty() {
                page_source.push_str(&control_mount_script_snippet(source, &hydration_payload));
                trace.push("rwe_control_script=mounted".to_string());
            }
        }
        if !compiled.tailwind_variant_patterns.is_empty() {
            page_source.push_str(&tailwind_dynamic_runtime_script_snippet(
                &compiled.tailwind_variant_patterns,
            ));
            trace.push(format!(
                "rwe_tw_variants_runtime={}",
                compiled.tailwind_variant_patterns.join(",")
            ));
        }

        let mut compiled_scripts = vec![CompiledScript {
            id: "runtime".to_string(),
            scope: CompiledScriptScope::Shared,
            content_type: "text/javascript; charset=utf-8".to_string(),
            content_hash: stable_content_hash(&runtime_source),
            suggested_file_name: format!(
                "{}.{}",
                compiled.runtime_bundle.name.trim_end_matches(".js"),
                "js"
            ),
            content: runtime_source.clone(),
        }];
        if !page_source.is_empty() {
            compiled_scripts.push(CompiledScript {
                id: "page".to_string(),
                scope: CompiledScriptScope::Page,
                content_type: "text/javascript; charset=utf-8".to_string(),
                content_hash: stable_content_hash(&page_source),
                suggested_file_name: format!("{}.page.js", compiled.template_id),
                content: page_source.clone(),
            });
        }

        // Backward compatibility: current HTML still receives one inline script
        // while platform-level external script routing is rolled out.
        let inline_source = format!("{runtime_source}{page_source}");
        let runtime_bundle = RuntimeBundle {
            name: compiled.runtime_bundle.name.clone(),
            source: inline_source,
        };
        let html = inject_runtime_bundle(&ssr_result.html, &runtime_bundle);

        Ok(RenderOutput {
            html,
            compiled_scripts,
            hydration_payload,
            trace,
        })
    }
}

fn runtime_bundle(mode: &RuntimeMode) -> RuntimeBundle {
    match mode {
        RuntimeMode::Prod => RuntimeBundle {
            name: "rwe-runtime.js".to_string(),
            source: RWE_RUNTIME_PROD_JS.to_string(),
        },
        RuntimeMode::Dev => RuntimeBundle {
            name: "rwe-runtime-dev.js".to_string(),
            source: format!("{RWE_RUNTIME_PROD_JS}\n{RWE_RUNTIME_DEV_JS}"),
        },
    }
}

fn inject_runtime_bundle(html: &str, bundle: &RuntimeBundle) -> String {
    let tag = format!(
        "<script data-rwe-runtime=\"{}\">{}</script>",
        bundle.name, bundle.source
    );
    inject_before_body_end(html, &tag)
}

fn collect_dynamic_class_placeholders(html: &str) -> Vec<String> {
    let mut found = std::collections::BTreeSet::new();
    let mut cursor = 0usize;
    while let Some(start) = html[cursor..].find("class=\"") {
        let class_start = cursor + start + "class=\"".len();
        let Some(end_rel) = html[class_start..].find('"') else {
            return found.into_iter().collect();
        };
        let class_value = &html[class_start..class_start + end_rel];
        for placeholder in collect_untyped_placeholders_from_class_value(class_value) {
            found.insert(placeholder);
        }
        cursor = class_start + end_rel + 1;
    }
    found.into_iter().collect()
}

fn reactive_bindings_bootstrap_snippet(bindings: &[ReactiveBinding]) -> String {
    format!(
        "window.__ZEBFLOW_RWE_BINDINGS__={};",
        serde_json::to_string(bindings).unwrap_or("[]".to_string())
    )
}

fn control_mount_script_snippet(source: &str, hydration_payload: &Value) -> String {
    let bootstrap = json!({
        "input": hydration_payload.get("input").cloned().unwrap_or(Value::Null),
        "metadata": hydration_payload.get("metadata").cloned().unwrap_or(Value::Null),
    });
    let source_json = serde_json::to_string(source).unwrap_or_else(|_| "\"\"".to_string());
    let bootstrap_json = serde_json::to_string(&bootstrap).unwrap_or_else(|_| "{}".to_string());
    format!(
        "(function(){{try{{const __tj_bootstrap={};const __tj_factory=new Function('input','metadata',{});const __tj_app=__tj_factory(__tj_bootstrap.input,__tj_bootstrap.metadata)||{{}};if(window.__ZEBFLOW_RWE__&&typeof window.__ZEBFLOW_RWE__.mount==='function'){{window.__ZEBFLOW_RWE__.mount(__tj_app,__tj_bootstrap);}}}}catch(err){{console.error('[ZEBFLOW][RWE] control script failed',err);}}}})();",
        bootstrap_json, source_json
    )
}

fn inject_before_body_end(html: &str, snippet: &str) -> String {
    if let Some(pos) = html.rfind("</body>") {
        let mut out = html.to_string();
        out.insert_str(pos, snippet);
        out
    } else {
        format!("{html}{snippet}")
    }
}

fn stable_content_hash(input: &str) -> String {
    // Deterministic FNV-1a 64-bit (lightweight, no extra dependency).
    let mut h: u64 = 0xcbf29ce484222325;
    for b in input.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x00000100000001B3);
    }
    format!("{h:016x}")
}

fn tailwind_dynamic_runtime_script_snippet(patterns: &[String]) -> String {
    let patterns_json = serde_json::to_string(patterns).unwrap_or_else(|_| "[]".to_string());
    format!(
        "(function(){{const patterns={patterns_json};if(!Array.isArray(patterns)||patterns.length===0)return;const hasBg=patterns.indexOf('bg-[*]')!==-1;const hasText=patterns.indexOf('text-[*]')!==-1;const hasBorder=patterns.indexOf('border-[*]')!==-1;if(!hasBg&&!hasText&&!hasBorder)return;function sanitize(raw){{if(typeof raw!=='string')return null;const v=raw.replace(/_/g,' ').trim();if(!v||v.length>128)return null;if(/[;{{}}]/.test(v))return null;if(/url\\s*\\(/i.test(v))return null;if(/expression\\s*\\(/i.test(v))return null;return v;}}function matchToken(token,prefix){{if(!token.startsWith(prefix+'-[')||!token.endsWith(']'))return null;return sanitize(token.slice(prefix.length+2,-1));}}function uniq(tokens){{const out=[];const seen=new Set();for(const t of tokens){{if(!t||seen.has(t))continue;seen.add(t);out.push(t);}}return out;}}function applyEl(el){{if(!el||!el.getAttribute)return;const classValue=el.getAttribute('class');if(!classValue)return;const parts=classValue.split(/\\s+/).filter(Boolean);if(parts.length===0)return;let changed=false;const next=[];for(const token of parts){{let consumed=false;if(hasBg){{const v=matchToken(token,'bg');if(v!==null){{next.push('tw-bg-dyn');el.style.setProperty('--tw-bg',v);changed=true;consumed=true;}}}}if(!consumed&&hasText){{const v=matchToken(token,'text');if(v!==null){{next.push('tw-text-dyn');el.style.setProperty('--tw-text',v);changed=true;consumed=true;}}}}if(!consumed&&hasBorder){{const v=matchToken(token,'border');if(v!==null){{next.push('tw-border-dyn');el.style.setProperty('--tw-border',v);changed=true;consumed=true;}}}}if(!consumed)next.push(token);}}if(changed){{el.setAttribute('class',uniq(next).join(' '));}}}}function scan(root){{const target=root&&root.querySelectorAll?root:document;if(!target)return;if(root&&root.nodeType===1&&root.getAttribute&&root.getAttribute('class'))applyEl(root);const nodes=target.querySelectorAll?target.querySelectorAll('[class]'):[];for(const el of nodes)applyEl(el);}}scan(document);if(typeof MutationObserver==='function'){{const mo=new MutationObserver((records)=>{{for(const r of records){{if(r.type==='attributes'&&r.attributeName==='class'){{applyEl(r.target);continue;}}if(r.type==='childList'){{for(const n of r.addedNodes){{if(n&&n.nodeType===1)scan(n);}}}}}});}});mo.observe(document.documentElement||document.body,{{subtree:true,childList:true,attributes:true,attributeFilter:['class']}});}}window.__ZEBFLOW_TW_DYN__={{patterns:patterns,scan:scan}};}})();"
    )
}

fn build_ssr_scope(input: &Value, ctx: &RenderContext) -> Value {
    json!({
        "input": input,
        "ctx": {
            "route": ctx.route,
            "requestId": ctx.request_id,
            "metadata": ctx.metadata,
        }
    })
}

fn apply_ssr_for_loops(html: &str, scope: &Value) -> JForSsrResult {
    let mut current = html.to_string();
    let mut loop_count = 0usize;
    let mut seeded_items = 0usize;
    while let Some(next) = expand_first_ssr_for_loop(&current, scope) {
        current = next.html;
        loop_count += 1;
        seeded_items += next.seeded_items;
    }
    JForSsrResult {
        html: current,
        loop_count,
        seeded_items,
    }
}

fn expand_first_ssr_for_loop(html: &str, scope: &Value) -> Option<JForSsrResult> {
    let for_attr_start_rel = html.find("z-for=\"")?;
    let for_attr_start = for_attr_start_rel;
    let tag_start = html[..for_attr_start].rfind('<')?;
    let open_end_rel = html[tag_start..].find('>')?;
    let open_end = tag_start + open_end_rel;
    let open_tag = &html[tag_start..=open_end];
    let tag_name = extract_tag_name(open_tag)?;
    if open_tag.trim_end().ends_with("/>") {
        return None;
    }

    let close_tag = format!("</{tag_name}>");
    let (close_start, close_end) = find_matching_close_tag(html, open_end + 1, tag_name)?;
    let inner = &html[open_end + 1..close_start];

    let for_expr = attr_value(open_tag, "z-for")?;
    let (item_var, list_expr) = parse_for_expr(for_expr)?;
    let key_expr = attr_value(open_tag, "z-key");

    let seed_id = format!("rwefor_{tag_start}");
    let mut template_open = remove_attr_from_open_tag(open_tag, "z-for");
    template_open = add_attr_to_open_tag(&template_open, "data-rwe-for-id", &seed_id);
    template_open = add_attr_to_open_tag(&template_open, "data-rwe-for-template", "1");
    template_open = add_bool_attr_to_open_tag(&template_open, "hidden");
    let template_block = format!("{template_open}{inner}{close_tag}");

    let rows = resolve_value_path(scope, list_expr)
        .and_then(Value::as_array)
        .map(|items| {
            let mut rendered = String::new();
            for (idx, item) in items.iter().enumerate() {
                let mut row_open = remove_attr_from_open_tag(open_tag, "z-for");
                row_open = remove_attr_from_open_tag(&row_open, "z-key");
                row_open = add_attr_to_open_tag(&row_open, "data-rwe-for-seeded", &seed_id);
                if let Some(expr) = key_expr {
                    let key = resolve_loop_key(expr, item_var, item, idx);
                    row_open = add_attr_to_open_tag(&row_open, "data-rwe-for-key", &key);
                }
                // Resolve loop placeholders in opening tag attributes (for example href/src).
                row_open = replace_loop_placeholders(&row_open, item_var, item, idx);
                let row_inner = replace_loop_placeholders(inner, item_var, item, idx);
                let row_markup = format!("{row_open}{row_inner}{close_tag}");
                let local_scope = build_loop_scope(scope, item_var, item, idx);
                let row_markup = apply_ssr_visibility(&row_markup, &local_scope).html;
                let row_markup = strip_visibility_attrs(&row_markup);
                rendered.push_str(&row_markup);
            }
            (rendered, items.len())
        })
        .unwrap_or_default();

    let replacement = format!("{template_block}{}", rows.0);
    let mut next = String::new();
    next.push_str(&html[..tag_start]);
    next.push_str(&replacement);
    next.push_str(&html[close_end..]);
    Some(JForSsrResult {
        html: next,
        loop_count: 1,
        seeded_items: rows.1,
    })
}

fn build_loop_scope(scope: &Value, item_var: &str, item: &Value, idx: usize) -> Value {
    let mut map = serde_json::Map::new();
    if let Some(input) = scope.get("input") {
        map.insert("input".to_string(), input.clone());
    }
    if let Some(ctx) = scope.get("ctx") {
        map.insert("ctx".to_string(), ctx.clone());
    }
    map.insert(item_var.to_string(), item.clone());
    map.insert("$index".to_string(), Value::from(idx));
    Value::Object(map)
}

fn strip_visibility_attrs(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut cursor = 0usize;

    while let Some(start_rel) = html[cursor..].find('<') {
        let start = cursor + start_rel;
        out.push_str(&html[cursor..start]);
        let Some(end_rel) = html[start..].find('>') else {
            out.push_str(&html[start..]);
            return out;
        };
        let end = start + end_rel + 1;
        let tag = &html[start..end];
        let next = tag.as_bytes().get(1).copied().unwrap_or_default();
        if matches!(next, b'/' | b'!' | b'?') {
            out.push_str(tag);
        } else {
            let mut next_tag = remove_attr_from_open_tag(tag, "z-show");
            next_tag = remove_attr_from_open_tag(&next_tag, "z-hide");
            out.push_str(&next_tag);
        }
        cursor = end;
    }

    out.push_str(&html[cursor..]);
    out
}

fn find_matching_close_tag(
    html: &str,
    search_start: usize,
    tag_name: &str,
) -> Option<(usize, usize)> {
    let open_prefix = format!("<{tag_name}");
    let close_tag = format!("</{tag_name}>");
    let mut cursor = search_start;
    let mut depth = 1usize;

    while cursor < html.len() {
        let next_open = html[cursor..].find(&open_prefix).map(|rel| cursor + rel);
        let next_close = html[cursor..].find(&close_tag).map(|rel| cursor + rel);

        match (next_open, next_close) {
            (None, None) => return None,
            (Some(open_idx), None) => {
                let open_end = html[open_idx..].find('>')? + open_idx;
                let open_tag = &html[open_idx..=open_end];
                if !open_tag.trim_end().ends_with("/>") {
                    depth += 1;
                }
                cursor = open_end + 1;
            }
            (None, Some(close_idx)) => {
                depth = depth.saturating_sub(1);
                let close_end = close_idx + close_tag.len();
                if depth == 0 {
                    return Some((close_idx, close_end));
                }
                cursor = close_end;
            }
            (Some(open_idx), Some(close_idx)) => {
                if close_idx < open_idx {
                    depth = depth.saturating_sub(1);
                    let close_end = close_idx + close_tag.len();
                    if depth == 0 {
                        return Some((close_idx, close_end));
                    }
                    cursor = close_end;
                } else {
                    let open_end = html[open_idx..].find('>')? + open_idx;
                    let open_tag = &html[open_idx..=open_end];
                    if !open_tag.trim_end().ends_with("/>") {
                        depth += 1;
                    }
                    cursor = open_end + 1;
                }
            }
        }
    }

    None
}

fn apply_ssr_placeholders(html: &str, scope: &Value) -> SsrRenderResult {
    let html = resolve_typed_class_macros(html, |path| {
        resolve_value_path(scope, path).map(value_to_ssr_string)
    });

    let mut out = String::with_capacity(html.len());
    let mut cursor = 0usize;
    let mut replacements = 0usize;

    while let Some(start_rel) = html[cursor..].find("{{") {
        let start = cursor + start_rel;
        out.push_str(&html[cursor..start]);
        let Some(end_rel) = html[start + 2..].find("}}") else {
            out.push_str(&html[start..]);
            return SsrRenderResult {
                html: out,
                replacements,
            };
        };
        let end = start + 2 + end_rel;
        let expr = html[start + 2..end].trim();
        if let Some(value) = resolve_value_path(scope, expr) {
            out.push_str(&escape_html(&value_to_ssr_string(value)));
            replacements += 1;
        } else {
            out.push_str(&html[start..end + 2]);
        }
        cursor = end + 2;
    }
    out.push_str(&html[cursor..]);

    SsrRenderResult {
        html: out,
        replacements,
    }
}

fn apply_ssr_visibility(html: &str, scope: &Value) -> SsrRenderResult {
    let mut out = String::with_capacity(html.len());
    let mut cursor = 0usize;
    let mut replacements = 0usize;

    while let Some(start_rel) = html[cursor..].find('<') {
        let start = cursor + start_rel;
        out.push_str(&html[cursor..start]);
        let Some(end_rel) = html[start..].find('>') else {
            out.push_str(&html[start..]);
            return SsrRenderResult {
                html: out,
                replacements,
            };
        };
        let end = start + end_rel + 1;
        let tag = &html[start..end];
        let next = tag.as_bytes().get(1).copied().unwrap_or_default();
        if matches!(next, b'/' | b'!' | b'?') {
            out.push_str(tag);
            cursor = end;
            continue;
        }

        let show_expr = attr_value(tag, "z-show");
        let hide_expr = attr_value(tag, "z-hide");
        if show_expr.is_none() && hide_expr.is_none() {
            out.push_str(tag);
            cursor = end;
            continue;
        }

        let mut next_tag = tag.to_string();
        let should_show = show_expr
            .map(|expr| eval_boolean_binding_expr(scope, expr))
            .unwrap_or(true);
        let should_hide = hide_expr
            .map(|expr| eval_boolean_binding_expr(scope, expr))
            .unwrap_or(false);
        let hidden = !should_show || should_hide;

        next_tag = remove_bool_attr_from_open_tag(&next_tag, "hidden");
        if hidden {
            next_tag = add_bool_attr_to_open_tag(&next_tag, "hidden");
        }

        if next_tag != tag {
            replacements += 1;
        }
        out.push_str(&next_tag);
        cursor = end;
    }

    out.push_str(&html[cursor..]);
    SsrRenderResult {
        html: out,
        replacements,
    }
}

fn resolve_value_path<'a>(root: &'a Value, path: &str) -> Option<&'a Value> {
    if path.is_empty() {
        return None;
    }
    let mut current = root;
    for seg in parse_path_segments(path) {
        let key = seg.trim();
        if key.is_empty() {
            continue;
        }
        match current {
            Value::Object(map) => {
                current = map.get(key)?;
            }
            Value::Array(list) => {
                let idx = key.parse::<usize>().ok()?;
                current = list.get(idx)?;
            }
            _ => return None,
        }
    }
    Some(current)
}

fn parse_path_segments(path: &str) -> Vec<String> {
    let mut segments = Vec::new();
    let mut buf = String::new();
    let chars: Vec<char> = path.chars().collect();
    let mut i = 0usize;

    while i < chars.len() {
        match chars[i] {
            '.' => {
                if !buf.trim().is_empty() {
                    segments.push(buf.trim().to_string());
                }
                buf.clear();
                i += 1;
            }
            '[' => {
                if !buf.trim().is_empty() {
                    segments.push(buf.trim().to_string());
                }
                buf.clear();

                i += 1;
                let start = i;
                while i < chars.len() && chars[i] != ']' {
                    i += 1;
                }
                if i <= chars.len() {
                    let mut inner: String = chars[start..i].iter().collect();
                    inner = inner.trim().to_string();
                    if (inner.starts_with('"') && inner.ends_with('"'))
                        || (inner.starts_with('\'') && inner.ends_with('\''))
                    {
                        inner = inner[1..inner.len().saturating_sub(1)].to_string();
                    }
                    if !inner.is_empty() {
                        segments.push(inner);
                    }
                }
                if i < chars.len() && chars[i] == ']' {
                    i += 1;
                }
            }
            ch => {
                buf.push(ch);
                i += 1;
            }
        }
    }

    if !buf.trim().is_empty() {
        segments.push(buf.trim().to_string());
    }

    segments
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum BindingExprToken {
    And,
    Or,
    Not,
    LParen,
    RParen,
    Atom(String),
}

fn eval_boolean_binding_expr(scope: &Value, expr: &str) -> bool {
    let tokens = tokenize_binding_expr(expr);
    if tokens.is_empty() {
        return false;
    }
    let mut idx = 0usize;
    let value = parse_binding_expr_or(scope, &tokens, &mut idx);
    if idx != tokens.len() {
        return false;
    }
    value
}

fn tokenize_binding_expr(expr: &str) -> Vec<BindingExprToken> {
    let chars: Vec<char> = expr.chars().collect();
    let mut tokens = Vec::new();
    let mut i = 0usize;
    while i < chars.len() {
        let ch = chars[i];
        if ch.is_whitespace() {
            i += 1;
            continue;
        }
        if ch == '&' && chars.get(i + 1) == Some(&'&') {
            tokens.push(BindingExprToken::And);
            i += 2;
            continue;
        }
        if ch == '|' && chars.get(i + 1) == Some(&'|') {
            tokens.push(BindingExprToken::Or);
            i += 2;
            continue;
        }
        if ch == '!' {
            tokens.push(BindingExprToken::Not);
            i += 1;
            continue;
        }
        if ch == '(' {
            tokens.push(BindingExprToken::LParen);
            i += 1;
            continue;
        }
        if ch == ')' {
            tokens.push(BindingExprToken::RParen);
            i += 1;
            continue;
        }
        let start = i;
        while i < chars.len() {
            let current = chars[i];
            if current.is_whitespace() || matches!(current, '(' | ')' | '!') {
                break;
            }
            if current == '&' && chars.get(i + 1) == Some(&'&') {
                break;
            }
            if current == '|' && chars.get(i + 1) == Some(&'|') {
                break;
            }
            i += 1;
        }
        let atom: String = chars[start..i].iter().collect();
        if !atom.is_empty() {
            tokens.push(BindingExprToken::Atom(atom));
        }
    }
    tokens
}

fn parse_binding_expr_or(scope: &Value, tokens: &[BindingExprToken], idx: &mut usize) -> bool {
    let mut left = parse_binding_expr_and(scope, tokens, idx);
    while matches!(tokens.get(*idx), Some(BindingExprToken::Or)) {
        *idx += 1;
        let right = parse_binding_expr_and(scope, tokens, idx);
        left = left || right;
    }
    left
}

fn parse_binding_expr_and(scope: &Value, tokens: &[BindingExprToken], idx: &mut usize) -> bool {
    let mut left = parse_binding_expr_unary(scope, tokens, idx);
    while matches!(tokens.get(*idx), Some(BindingExprToken::And)) {
        *idx += 1;
        let right = parse_binding_expr_unary(scope, tokens, idx);
        left = left && right;
    }
    left
}

fn parse_binding_expr_unary(scope: &Value, tokens: &[BindingExprToken], idx: &mut usize) -> bool {
    if matches!(tokens.get(*idx), Some(BindingExprToken::Not)) {
        *idx += 1;
        return !parse_binding_expr_unary(scope, tokens, idx);
    }
    parse_binding_expr_primary(scope, tokens, idx)
}

fn parse_binding_expr_primary(scope: &Value, tokens: &[BindingExprToken], idx: &mut usize) -> bool {
    match tokens.get(*idx) {
        Some(BindingExprToken::LParen) => {
            *idx += 1;
            let value = parse_binding_expr_or(scope, tokens, idx);
            if matches!(tokens.get(*idx), Some(BindingExprToken::RParen)) {
                *idx += 1;
                value
            } else {
                false
            }
        }
        Some(BindingExprToken::Atom(atom)) => {
            *idx += 1;
            binding_expr_atom_truthy(scope, atom)
        }
        _ => false,
    }
}

fn binding_expr_atom_truthy(scope: &Value, atom: &str) -> bool {
    match atom {
        "true" => true,
        "false" => false,
        "null" | "undefined" => false,
        _ => resolve_value_path(scope, atom)
            .map(binding_value_truthy)
            .unwrap_or(false),
    }
}

fn binding_value_truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(v) => *v,
        Value::Number(v) => {
            if let Some(n) = v.as_i64() {
                n != 0
            } else if let Some(n) = v.as_u64() {
                n != 0
            } else {
                v.as_f64().map(|n| n != 0.0 && !n.is_nan()).unwrap_or(false)
            }
        }
        Value::String(v) => !v.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}

fn parse_for_expr(expr: &str) -> Option<(&str, &str)> {
    let mut parts = expr.splitn(2, " in ");
    let var = parts.next()?.trim();
    let list = parts.next()?.trim();
    if var.is_empty() || list.is_empty() {
        return None;
    }
    Some((var, list))
}

fn resolve_loop_key(expr: &str, item_var: &str, item: &Value, idx: usize) -> String {
    let expr = expr.trim();
    if expr == "$index" {
        return idx.to_string();
    }
    if expr == item_var {
        return value_to_ssr_string(item);
    }
    if let Some(path) = expr.strip_prefix(&format!("{item_var}.")) {
        if let Some(v) = resolve_value_path(item, path) {
            return value_to_ssr_string(v);
        }
    }
    idx.to_string()
}

fn replace_loop_placeholders(fragment: &str, item_var: &str, item: &Value, idx: usize) -> String {
    let mut out = String::with_capacity(fragment.len());
    let mut cursor = 0usize;
    while let Some(start_rel) = fragment[cursor..].find("{{") {
        let start = cursor + start_rel;
        out.push_str(&fragment[cursor..start]);
        let Some(end_rel) = fragment[start + 2..].find("}}") else {
            out.push_str(&fragment[start..]);
            return out;
        };
        let end = start + 2 + end_rel;
        let expr = fragment[start + 2..end].trim();
        if expr == "$index" {
            out.push_str(&idx.to_string());
        } else if expr == item_var {
            out.push_str(&escape_html(&value_to_ssr_string(item)));
        } else if let Some(path) = expr.strip_prefix(&format!("{item_var}.")) {
            if let Some(value) = resolve_value_path(item, path) {
                out.push_str(&escape_html(&value_to_ssr_string(value)));
            }
        } else {
            out.push_str(&fragment[start..end + 2]);
        }
        cursor = end + 2;
    }
    out.push_str(&fragment[cursor..]);
    out
}

fn extract_tag_name(open_tag: &str) -> Option<&str> {
    let tag = open_tag.strip_prefix('<')?;
    let tag = tag.trim_start();
    let end = tag.find(|c: char| c.is_whitespace() || c == '>' || c == '/')?;
    let name = &tag[..end];
    if name.is_empty() { None } else { Some(name) }
}

fn remove_attr_from_open_tag(open_tag: &str, attr: &str) -> String {
    let pattern = format!("{attr}=\"");
    let Some(start) = open_tag.find(&pattern) else {
        return open_tag.to_string();
    };
    let mut from = start;
    while from > 0 && open_tag.as_bytes()[from - 1].is_ascii_whitespace() {
        from -= 1;
    }
    let value_from = start + pattern.len();
    let Some(end_rel) = open_tag[value_from..].find('"') else {
        return open_tag.to_string();
    };
    let to = value_from + end_rel + 1;
    let mut out = String::with_capacity(open_tag.len());
    out.push_str(&open_tag[..from]);
    out.push_str(&open_tag[to..]);
    out
}

fn add_attr_to_open_tag(open_tag: &str, attr: &str, value: &str) -> String {
    if open_tag.contains(&format!("{attr}=\"")) {
        return open_tag.to_string();
    }
    if let Some(idx) = open_tag.rfind('>') {
        let mut out = String::with_capacity(open_tag.len() + attr.len() + value.len() + 4);
        out.push_str(&open_tag[..idx]);
        out.push(' ');
        out.push_str(attr);
        out.push_str("=\"");
        out.push_str(value);
        out.push('"');
        out.push_str(&open_tag[idx..]);
        out
    } else {
        open_tag.to_string()
    }
}

fn add_bool_attr_to_open_tag(open_tag: &str, attr: &str) -> String {
    if open_tag.contains(&format!(" {attr}")) {
        return open_tag.to_string();
    }
    if let Some(idx) = open_tag.rfind('>') {
        let mut out = String::with_capacity(open_tag.len() + attr.len() + 2);
        out.push_str(&open_tag[..idx]);
        out.push(' ');
        out.push_str(attr);
        out.push_str(&open_tag[idx..]);
        out
    } else {
        open_tag.to_string()
    }
}

fn remove_bool_attr_from_open_tag(open_tag: &str, attr: &str) -> String {
    let pattern = format!(" {attr}");
    if let Some(start) = open_tag.find(&pattern) {
        let mut out = String::with_capacity(open_tag.len());
        out.push_str(&open_tag[..start]);
        out.push_str(&open_tag[start + pattern.len()..]);
        out
    } else {
        open_tag.to_string()
    }
}

fn value_to_ssr_string(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::Bool(v) => v.to_string(),
        Value::Number(v) => v.to_string(),
        Value::String(v) => v.clone(),
        Value::Array(_) | Value::Object(_) => serde_json::to_string(value).unwrap_or_default(),
    }
}

fn escape_html(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(ch),
        }
    }
    out
}

fn expand_registered_components(
    markup: &str,
    components: &ComponentOptions,
    diagnostics: &mut Vec<ReactiveWebDiagnostic>,
) -> Result<String, String> {
    if components.registry.is_empty() {
        return Ok(markup.to_string());
    }

    let mut current = markup.to_string();
    for depth in 0..8 {
        let (next, replaced) =
            expand_registered_components_once(&current, components, diagnostics)?;
        current = next;
        if !replaced {
            return Ok(current);
        }
        if depth == 7 {
            diagnostics.push(ReactiveWebDiagnostic {
                code: "RWE_COMPONENT_DEPTH_LIMIT".to_string(),
                message: "component expansion stopped at depth limit".to_string(),
            });
        }
    }
    Ok(current)
}

fn expand_registered_components_once(
    markup: &str,
    components: &ComponentOptions,
    diagnostics: &mut Vec<ReactiveWebDiagnostic>,
) -> Result<(String, bool), String> {
    let mut out = String::new();
    let mut cursor = 0usize;
    let mut replaced_any = false;

    while let Some(start_rel) = markup[cursor..].find('<') {
        let start = cursor + start_rel;
        out.push_str(&markup[cursor..start]);

        // Skip closing tags and declarations.
        let next = markup
            .as_bytes()
            .get(start + 1)
            .copied()
            .unwrap_or_default();
        if matches!(next, b'/' | b'!' | b'?') {
            out.push('<');
            cursor = start + 1;
            continue;
        }

        let Some(open_end_rel) = markup[start..].find('>') else {
            out.push_str(&markup[start..]);
            return Ok((out, replaced_any));
        };
        let open_end = start + open_end_rel + 1;
        let open_tag = &markup[start..open_end];
        let Some(tag_name) = extract_tag_name(open_tag) else {
            out.push_str(open_tag);
            cursor = open_end;
            continue;
        };
        if !is_component_tag_name(tag_name) {
            out.push_str(open_tag);
            cursor = open_end;
            continue;
        }

        let self_closing = open_tag.trim_end().ends_with("/>");
        let mut component_props = parse_component_call_attributes(open_tag, tag_name);
        let hydrate_mode = component_props
            .get("hydrate")
            .cloned()
            .unwrap_or_else(|| "off".to_string());

        let (close_start, close_end) = if self_closing {
            (open_end, open_end)
        } else {
            let close_tag = format!("</{tag_name}>");
            let Some(close_rel) = markup[open_end..].find(&close_tag) else {
                return Err(format!(
                    "malformed component <{tag_name}>: missing closing tag"
                ));
            };
            let close_start = open_end + close_rel;
            (close_start, close_start + close_tag.len())
        };
        if !self_closing {
            let children = markup[open_end..close_start].to_string();
            if !children.trim().is_empty() {
                component_props
                    .entry("children".to_string())
                    .or_insert(children);
            }
        }

        if let Some(component_markup) = components.registry.get(tag_name) {
            let rendered_component = if looks_like_tsx_source(component_markup, None) {
                match lower_tsx_source_to_parts(component_markup) {
                    Ok(lowered) => lowered.html_template,
                    Err(err) => {
                        if components.strict {
                            return Err(format!(
                                "failed lowering component '{}' TSX source: {err}",
                                tag_name
                            ));
                        }
                        diagnostics.push(ReactiveWebDiagnostic {
                            code: "RWE_COMPONENT_TSX_LOWER_WARN".to_string(),
                            message: format!(
                                "component '{}' TSX lowering failed; raw source kept: {err}",
                                tag_name
                            ),
                        });
                        component_markup.to_string()
                    }
                }
            } else {
                component_markup.to_string()
            };
            let rendered_component =
                substitute_component_props(&rendered_component, &component_props);

            if let Some(mode) = normalize_component_hydration_mode(&hydrate_mode) {
                out.push_str(&format!(
                    "<div data-rwe-component=\"{}\" hydrate=\"{}\" style=\"display:contents\">{}</div>",
                    escape_attr(tag_name),
                    escape_attr(mode),
                    rendered_component
                ));
            } else {
                out.push_str(&rendered_component);
            }
            diagnostics.push(ReactiveWebDiagnostic {
                code: "RWE_COMPONENT_RESOLVED".to_string(),
                message: format!("component '{}' resolved from registry", tag_name),
            });
            if normalize_component_hydration_mode(&hydrate_mode).is_some() {
                diagnostics.push(ReactiveWebDiagnostic {
                    code: "RWE_COMPONENT_HYDRATE".to_string(),
                    message: format!(
                        "component '{}' wrapped as hydration island with mode '{}'",
                        tag_name, hydrate_mode
                    ),
                });
            }
            replaced_any = true;
        } else if components.strict {
            return Err(format!("component '{}' not found in registry", tag_name));
        } else {
            out.push_str(&markup[start..close_end]);
            diagnostics.push(ReactiveWebDiagnostic {
                code: "RWE_COMPONENT_MISSING".to_string(),
                message: format!("component '{}' missing; tag preserved", tag_name),
            });
        }

        cursor = close_end;
    }

    out.push_str(&markup[cursor..]);
    Ok((out, replaced_any))
}

fn is_component_tag_name(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !first.is_ascii_uppercase() {
        return false;
    }
    chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

fn parse_component_call_attributes(
    open_tag: &str,
    tag_name: &str,
) -> std::collections::HashMap<String, String> {
    let mut attrs = std::collections::HashMap::new();
    let chars: Vec<char> = open_tag.chars().collect();
    if chars.len() < 3 {
        return attrs;
    }

    let mut i = 1 + tag_name.chars().count();
    while i < chars.len() {
        while i < chars.len() && chars[i].is_whitespace() {
            i += 1;
        }
        if i >= chars.len() || chars[i] == '>' || chars[i] == '/' {
            break;
        }

        let key_start = i;
        while i < chars.len()
            && (chars[i].is_ascii_alphanumeric() || matches!(chars[i], '_' | '-' | ':' | '.'))
        {
            i += 1;
        }
        if key_start == i {
            i += 1;
            continue;
        }
        let key: String = chars[key_start..i].iter().collect();

        while i < chars.len() && chars[i].is_whitespace() {
            i += 1;
        }
        if i >= chars.len() || chars[i] != '=' {
            attrs.insert(key, "true".to_string());
            continue;
        }
        i += 1;
        while i < chars.len() && chars[i].is_whitespace() {
            i += 1;
        }
        if i >= chars.len() {
            break;
        }

        let value = if chars[i] == '"' || chars[i] == '\'' {
            let quote = chars[i];
            i += 1;
            let start = i;
            while i < chars.len() && chars[i] != quote {
                i += 1;
            }
            let out: String = chars[start..i.min(chars.len())].iter().collect();
            if i < chars.len() && chars[i] == quote {
                i += 1;
            }
            out
        } else if chars[i] == '{' && i + 1 < chars.len() && chars[i + 1] == '{' {
            let start = i;
            i += 2;
            while i + 1 < chars.len() && !(chars[i] == '}' && chars[i + 1] == '}') {
                i += 1;
            }
            if i + 1 < chars.len() {
                i += 2;
            }
            chars[start..i.min(chars.len())].iter().collect()
        } else if chars[i] == '{' {
            let start = i;
            i += 1;
            let mut depth = 1usize;
            while i < chars.len() && depth > 0 {
                if chars[i] == '{' {
                    depth += 1;
                } else if chars[i] == '}' {
                    depth -= 1;
                }
                i += 1;
            }
            chars[start..i.min(chars.len())].iter().collect()
        } else {
            let start = i;
            while i < chars.len() && !chars[i].is_whitespace() && chars[i] != '>' && chars[i] != '/'
            {
                i += 1;
            }
            chars[start..i.min(chars.len())].iter().collect()
        };

        attrs.insert(key, value);
    }

    attrs
}

fn substitute_component_props(
    component_markup: &str,
    props: &std::collections::HashMap<String, String>,
) -> String {
    let mut out = resolve_typed_class_macros(component_markup, |path| {
        let key = path.strip_prefix("props.")?;
        props.get(key).cloned()
    });

    for (key, value) in props {
        let slot = format!("{{{{props.{key}}}}}");
        out = out.replace(&slot, value);
    }
    out
}

fn normalize_component_hydration_mode(mode: &str) -> Option<&'static str> {
    match mode.trim().to_ascii_lowercase().as_str() {
        "" | "off" => None,
        "immediate" => Some("immediate"),
        "interaction" => Some("interaction"),
        "visible" => Some("visible"),
        "idle" => Some("idle"),
        _ => None,
    }
}

fn escape_attr(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn enforce_resource_allow_list(
    html: &str,
    options: &ReactiveWebOptions,
    diagnostics: &mut Vec<ReactiveWebDiagnostic>,
) -> String {
    let no_scripts = strip_script_tags(html, options, diagnostics);
    strip_css_links(&no_scripts, options, diagnostics)
}

fn strip_script_tags(
    html: &str,
    options: &ReactiveWebOptions,
    diagnostics: &mut Vec<ReactiveWebDiagnostic>,
) -> String {
    let mut out = String::new();
    let mut cursor = 0usize;
    while let Some(start_rel) = html[cursor..].find("<script") {
        let start = cursor + start_rel;
        out.push_str(&html[cursor..start]);
        let Some(close_rel) = html[start..].find("</script>") else {
            out.push_str(&html[start..]);
            return out;
        };
        let end = start + close_rel + "</script>".len();
        let tag_block = &html[start..end];
        let src = attr_value(tag_block, "src");
        let keep = match src {
            Some(src) => script_allowed(src, options),
            None => false,
        };
        if keep {
            out.push_str(tag_block);
        } else {
            diagnostics.push(ReactiveWebDiagnostic {
                code: "RWE_SCRIPT_BLOCKED".to_string(),
                message: "script tag removed by allow-list".to_string(),
            });
        }
        cursor = end;
    }
    out.push_str(&html[cursor..]);
    out
}

fn strip_css_links(
    html: &str,
    options: &ReactiveWebOptions,
    diagnostics: &mut Vec<ReactiveWebDiagnostic>,
) -> String {
    let mut out = String::new();
    let mut cursor = 0usize;
    while let Some(start_rel) = html[cursor..].find("<link") {
        let start = cursor + start_rel;
        out.push_str(&html[cursor..start]);
        let Some(end_rel) = html[start..].find('>') else {
            out.push_str(&html[start..]);
            return out;
        };
        let end = start + end_rel + 1;
        let tag = &html[start..end];
        let rel = attr_value(tag, "rel");
        let href = attr_value(tag, "href");
        let is_stylesheet = rel
            .map(|r| r.eq_ignore_ascii_case("stylesheet"))
            .unwrap_or(false);
        if !is_stylesheet {
            out.push_str(tag);
            cursor = end;
            continue;
        }
        let keep = href.map(|h| css_allowed(h, options)).unwrap_or(false);
        if keep {
            out.push_str(tag);
        } else {
            diagnostics.push(ReactiveWebDiagnostic {
                code: "RWE_CSS_BLOCKED".to_string(),
                message: "stylesheet link removed by allow-list".to_string(),
            });
        }
        cursor = end;
    }
    out.push_str(&html[cursor..]);
    out
}

fn script_allowed(src: &str, options: &ReactiveWebOptions) -> bool {
    if options.load_scripts.is_empty() {
        return false;
    }
    let in_load_scripts = options
        .load_scripts
        .iter()
        .any(|r| matches_rule(src, r.as_str()));
    if !in_load_scripts {
        return false;
    }
    if options.allow_list.scripts.is_empty() && options.allow_list.urls.is_empty() {
        return true;
    }
    options
        .allow_list
        .scripts
        .iter()
        .any(|r| matches_rule(src, r.as_str()))
        || options
            .allow_list
            .urls
            .iter()
            .any(|r| matches_rule(src, r.as_str()))
}

fn css_allowed(href: &str, options: &ReactiveWebOptions) -> bool {
    if options.allow_list.css.is_empty() && options.allow_list.urls.is_empty() {
        return false;
    }
    options
        .allow_list
        .css
        .iter()
        .any(|r| matches_rule(href, r.as_str()))
        || options
            .allow_list
            .urls
            .iter()
            .any(|r| matches_rule(href, r.as_str()))
}

fn matches_rule(value: &str, rule: &str) -> bool {
    let rule = rule.trim();
    if rule == "*" {
        return true;
    }
    if let Some(prefix) = rule.strip_suffix('*') {
        return value.starts_with(prefix);
    }
    value == rule
}

fn attr_value<'a>(tag: &'a str, attr: &str) -> Option<&'a str> {
    let pattern = format!("{attr}=\"");
    let start = tag.find(&pattern)?;
    let from = start + pattern.len();
    let end = tag[from..].find('"')?;
    Some(&tag[from..from + end])
}

fn has_attr(tag: &str, attr: &str) -> bool {
    let lower = tag.to_ascii_lowercase();
    let a = attr.to_ascii_lowercase();
    lower.contains(&format!(" {a}"))
        || lower.contains(&format!("\t{a}"))
        || lower.contains(&format!("\n{a}"))
        || lower.starts_with(&format!("<{a}"))
}

fn prepare_document(
    markup: &str,
    diagnostics: &mut Vec<ReactiveWebDiagnostic>,
) -> PreparedDocument {
    let (without_control, control_scripts) = extract_control_scripts(markup, diagnostics);
    let (without_styles, inline_styles) = extract_inline_styles(&without_control, diagnostics);
    let html = extract_template_block(&without_styles).unwrap_or(without_styles);
    PreparedDocument {
        html,
        control_scripts,
        inline_styles,
    }
}

fn extract_template_block(markup: &str) -> Option<String> {
    let start = markup.find("<template")?;
    let open_end = start + markup[start..].find('>')? + 1;
    let close = markup[open_end..].find("</template>")?;
    Some(markup[open_end..open_end + close].to_string())
}

fn extract_control_scripts(
    html: &str,
    diagnostics: &mut Vec<ReactiveWebDiagnostic>,
) -> (String, Vec<ControlScript>) {
    let mut out = String::new();
    let mut scripts = Vec::new();
    let mut cursor = 0usize;

    while let Some(start_rel) = html[cursor..].find("<script") {
        let start = cursor + start_rel;
        out.push_str(&html[cursor..start]);

        let Some(open_end_rel) = html[start..].find('>') else {
            out.push_str(&html[start..]);
            return (out, scripts);
        };
        let open_end = start + open_end_rel + 1;
        let open_tag = &html[start..open_end];

        let Some(close_rel) = html[open_end..].find("</script>") else {
            out.push_str(&html[start..]);
            return (out, scripts);
        };
        let close_start = open_end + close_rel;
        let close_end = close_start + "</script>".len();
        let body = html[open_end..close_start].to_string();

        let script_kind = control_script_kind_from_tag(open_tag);
        if let Some(kind) = script_kind {
            scripts.push(ControlScript {
                kind,
                body: body.clone(),
            });
            diagnostics.push(ReactiveWebDiagnostic {
                code: "RWE_CONTROL_SCRIPT_EXTRACTED".to_string(),
                message: "control script extracted from .tsx".to_string(),
            });
        } else {
            out.push_str(&html[start..close_end]);
        }
        cursor = close_end;
    }

    out.push_str(&html[cursor..]);
    (out, scripts)
}

fn control_script_kind_from_tag(open_tag: &str) -> Option<ControlScriptKind> {
    let type_attr = attr_value(open_tag, "type")
        .map(|t| t.trim().to_ascii_lowercase())
        .unwrap_or_default();
    if type_attr == "zebflow/script" {
        return Some(ControlScriptKind::ControlScript);
    }
    if type_attr == "application/zebflow+json" {
        return Some(ControlScriptKind::ApplicationZebflowJson);
    }
    let lang_attr = attr_value(open_tag, "lang")
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_default();
    let explicit_control = has_attr(open_tag, "control")
        || attr_value(open_tag, "data-control")
            .map(|v| v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);
    let type_js = matches!(
        type_attr.as_str(),
        "" | "text/javascript" | "application/javascript" | "module"
    );
    if lang_attr == "js" || (explicit_control && type_js) {
        return Some(ControlScriptKind::ControlScript);
    }
    None
}

fn extract_inline_styles(
    html: &str,
    diagnostics: &mut Vec<ReactiveWebDiagnostic>,
) -> (String, Vec<String>) {
    let mut out = String::new();
    let mut styles = Vec::new();
    let mut cursor = 0usize;
    while let Some(start_rel) = html[cursor..].find("<style") {
        let start = cursor + start_rel;
        out.push_str(&html[cursor..start]);
        let Some(open_end_rel) = html[start..].find('>') else {
            out.push_str(&html[start..]);
            return (out, styles);
        };
        let open_end = start + open_end_rel + 1;
        let Some(close_rel) = html[open_end..].find("</style>") else {
            out.push_str(&html[start..]);
            return (out, styles);
        };
        let close_start = open_end + close_rel;
        let close_end = close_start + "</style>".len();
        let body = html[open_end..close_start].trim();
        if !body.is_empty() {
            styles.push(body.to_string());
            diagnostics.push(ReactiveWebDiagnostic {
                code: "RWE_INLINE_STYLE_EXTRACTED".to_string(),
                message: "inline style block extracted from .tsx".to_string(),
            });
        }
        cursor = close_end;
    }
    out.push_str(&html[cursor..]);
    (out, styles)
}

fn inject_inline_styles(html: &str, styles: &[String]) -> String {
    if styles.is_empty() {
        return html.to_string();
    }
    let style_tag = format!("<style data-rwe-inline>{}</style>", styles.join("\n"));
    if let Some(pos) = html.find("</head>") {
        let mut out = html.to_string();
        out.insert_str(pos, &style_tag);
        out
    } else {
        format!("{style_tag}{html}")
    }
}

fn inject_template_styles(
    html: &str,
    options: &ReactiveWebOptions,
    diagnostics: &mut Vec<ReactiveWebDiagnostic>,
) -> Result<String, ReactiveWebError> {
    let Some(template_root) = options.templates.template_root.as_deref() else {
        return Ok(html.to_string());
    };

    let styles = load_template_style_sources(template_root, &options.templates, diagnostics)?;
    if styles.is_empty() {
        return Ok(html.to_string());
    }

    let style_tag = styles
        .into_iter()
        .map(|style| {
            format!(
                "<style data-rwe-template-style=\"{}\">{}</style>",
                escape_html_attr(&style.path),
                style.css
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    if let Some(pos) = html.find("</head>") {
        let mut out = html.to_string();
        out.insert_str(pos, &style_tag);
        Ok(out)
    } else {
        Ok(format!("{style_tag}{html}"))
    }
}

#[derive(Debug)]
struct TemplateStyleSource {
    path: String,
    css: String,
}

fn load_template_style_sources(
    template_root: &Path,
    options: &crate::rwe::model::TemplateOptions,
    diagnostics: &mut Vec<ReactiveWebDiagnostic>,
) -> Result<Vec<TemplateStyleSource>, ReactiveWebError> {
    let explicit_entries = !options.style_entries.is_empty();
    let entry_paths: Vec<String> = if explicit_entries {
        options.style_entries.clone()
    } else {
        vec!["styles/main.css".to_string()]
    };

    let mut styles = Vec::new();
    for entry in entry_paths {
        let Some(style) =
            load_template_style_source(template_root, &entry, explicit_entries, diagnostics)?
        else {
            continue;
        };
        styles.push(style);
    }

    Ok(styles)
}

fn load_template_style_source(
    template_root: &Path,
    entry: &str,
    strict_missing: bool,
    diagnostics: &mut Vec<ReactiveWebDiagnostic>,
) -> Result<Option<TemplateStyleSource>, ReactiveWebError> {
    let relative = entry.trim();
    if relative.is_empty() {
        return Ok(None);
    }
    if Path::new(relative).is_absolute() {
        return Err(ReactiveWebError::new(
            "RWE_TEMPLATE_STYLE_ABSOLUTE",
            format!("template style entry '{relative}' must be relative to template_root"),
        ));
    }

    let joined = template_root.join(relative);
    if !joined.exists() {
        if strict_missing {
            return Err(ReactiveWebError::new(
                "RWE_TEMPLATE_STYLE_MISSING",
                format!(
                    "explicit template style entry '{}' does not exist under '{}'",
                    relative,
                    template_root.display()
                ),
            ));
        }
        return Ok(None);
    }

    let canonical_root = fs::canonicalize(template_root).map_err(|err| {
        ReactiveWebError::new(
            "RWE_TEMPLATE_ROOT",
            format!(
                "failed canonicalizing template root '{}': {err}",
                template_root.display()
            ),
        )
    })?;
    let canonical_path = fs::canonicalize(&joined).map_err(|err| {
        ReactiveWebError::new(
            "RWE_TEMPLATE_STYLE_RESOLVE",
            format!(
                "failed canonicalizing template style '{}': {err}",
                joined.display()
            ),
        )
    })?;

    if !canonical_path.starts_with(&canonical_root) {
        return Err(ReactiveWebError::new(
            "RWE_TEMPLATE_STYLE_BOUNDARY",
            format!(
                "template style '{}' escapes template_root '{}'",
                canonical_path.display(),
                canonical_root.display()
            ),
        ));
    }

    let css = fs::read_to_string(&canonical_path).map_err(|err| {
        ReactiveWebError::new(
            "RWE_TEMPLATE_STYLE_READ",
            format!(
                "failed reading template style '{}': {err}",
                canonical_path.display()
            ),
        )
    })?;
    diagnostics.push(ReactiveWebDiagnostic {
        code: "RWE_TEMPLATE_STYLE_INCLUDED".to_string(),
        message: format!("included template stylesheet '{relative}'"),
    });

    Ok(Some(TemplateStyleSource {
        path: relative.to_string(),
        css,
    }))
}

fn escape_html_attr(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(ch),
        }
    }
    out
}

fn build_control_script_source(
    scripts: &[ControlScript],
    diagnostics: &mut Vec<ReactiveWebDiagnostic>,
) -> Result<PreparedControlScript, String> {
    let Some(first) = scripts.first() else {
        return Ok(PreparedControlScript::default());
    };
    if scripts.len() > 1 {
        diagnostics.push(ReactiveWebDiagnostic {
            code: "RWE_MULTIPLE_CONTROL_SCRIPTS".to_string(),
            message: "multiple control scripts found; only first script is used".to_string(),
        });
    }

    let source = match first.kind {
        ControlScriptKind::ControlScript => first.body.trim().to_string(),
        ControlScriptKind::ApplicationZebflowJson => {
            let parsed: Value = serde_json::from_str(first.body.trim())
                .map_err(|e| format!("invalid zebflow+json payload: {e}"))?;
            if let Some(explicit) = parsed.get("scriptSource").and_then(Value::as_str) {
                explicit.to_string()
            } else {
                format!(
                    "return {};",
                    serde_json::to_string(&parsed)
                        .map_err(|e| format!("failed serializing zebflow+json payload: {e}"))?
                )
            }
        }
    };

    if source.trim().is_empty() {
        return Ok(PreparedControlScript { source: None });
    }
    Ok(PreparedControlScript {
        source: Some(source),
    })
}

fn collect_reactive_bindings(html: &str) -> Vec<ReactiveBinding> {
    let mut out = Vec::new();
    collect_attr_bindings(html, "@click", "event.click", &mut out);
    collect_attr_bindings(html, "@change", "event.change", &mut out);
    collect_attr_bindings(html, "z-text", "bind.text", &mut out);
    collect_attr_bindings(html, "z-model", "bind.model", &mut out);
    collect_attr_bindings(html, "z-attr:class", "bind.attr.class", &mut out);
    collect_attr_bindings(html, "z-show", "bind.show", &mut out);
    collect_attr_bindings(html, "z-hide", "bind.hide", &mut out);
    collect_attr_bindings(html, "z-for", "bind.for", &mut out);
    collect_attr_bindings(html, "z-key", "bind.for.key", &mut out);
    collect_attr_bindings(html, "hydrate", "hydrate.mode", &mut out);
    out
}

fn collect_attr_bindings(html: &str, attr: &str, kind: &str, out: &mut Vec<ReactiveBinding>) {
    let pattern = format!("{attr}=\"");
    let mut cursor = 0usize;
    while let Some(start_rel) = html[cursor..].find(&pattern) {
        let start = cursor + start_rel + pattern.len();
        let Some(end_rel) = html[start..].find('"') else {
            break;
        };
        let value = &html[start..start + end_rel];
        out.push(ReactiveBinding {
            kind: kind.to_string(),
            key: value.to_string(),
        });
        cursor = start + end_rel + 1;
    }
}
