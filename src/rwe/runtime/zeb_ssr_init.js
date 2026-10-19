// zeb_ssr_init.js — Zebflow SSR integration.
//
// Loaded once into the deno_core JsRuntime at startup.
// Sets up all globals that RWE templates expect: h, Fragment, React,
// useState, useEffect, useRef, useMemo, useCallback, useContext, useReducer,
// createContext, usePageState, useRouter, Link, and the internal
// __rweRenderToString / __rweWrapWithPageState helpers called from Rust.
// The runtime `zeb/*` libraries' server stand-ins are filed by
// zeb_ssr_libs.js under globalThis.__zebLibs, never as globals.

(function () {
  "use strict";

  // ---------------------------------------------------------------------------
  // HTML / attribute escaping
  // ---------------------------------------------------------------------------
  function escHtml(s) {
    if (s == null) return "";
    return String(s)
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;");
  }

  function escAttr(s) {
    if (s == null) return "";
    return String(s)
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;")
      .replace(/"/g, "&quot;");
  }

  // Shared Zeb React primitives; loaded before this adapter in every V8 worker.
  var { h, Fragment, renderToString, createContext, useState, useEffect,
    useLayoutEffect, useRef, useMemo, useCallback, useContext, useReducer,
    useId, useImperativeHandle, useSyncExternalStore, forwardRef, memo,
    createPortal } = globalThis.__zebReact;
  function useInsertionEffect() {} // SSR-only legacy no-op.

  // ---------------------------------------------------------------------------
  // Page-state context — shared mutable state across all components on a page
  // ---------------------------------------------------------------------------
  var PageStateContext = createContext(null);

  function createUsePageState() {
    return function usePageState(keyOrInitial, defaultValue) {
      var isKeyed = typeof keyOrInitial === "string";
      var ctx = useContext(PageStateContext);
      if (isKeyed) {
        var key = keyOrInitial;
        if (ctx && typeof ctx === "object") {
          var value = key in ctx ? ctx[key] : defaultValue;
          var setter = function (v) { if (ctx.setPageState) ctx.setPageState(function (p) { var o = {}; o[key] = v; return o; }); };
          return [value, setter];
        }
        // SSR root: no-op setter, just return default
        return [defaultValue, function () {}];
      }
      if (ctx && typeof ctx === "object") return ctx;
      return Object.assign({}, keyOrInitial || {}, { setPageState: function () {} });
    };
  }

  // ---------------------------------------------------------------------------
  // Navigation — SSR no-ops; browser hydration script has real implementations
  // ---------------------------------------------------------------------------
  // Where the request arrived. The server is rendering one fixed URL, so these
  // read the payload the pipeline injected rather than subscribing to anything
  // — there is nothing here that can change mid-render.
  function usePathname() {
    var ctx = globalThis.ctx || {};
    return typeof ctx.route === "string" && ctx.route ? ctx.route : "/";
  }

  function useSearchParams() {
    var ctx = globalThis.ctx || {};
    var entries = [];
    var query = ctx.query;
    if (query && typeof query === "object") {
      for (var key in query) {
        if (Object.prototype.hasOwnProperty.call(query, key)) {
          entries.push([key, String(query[key])]);
        }
      }
    }
    // deno_core installs no browser URLSearchParams. Keep this a local,
    // read-only query view rather than installing a partial web API globally.
    // Rust's URL implementation owns decoding and canonical form encoding.
    var snapshot = Deno.core.ops.op_rwe_search_params_snapshot(
      typeof ctx.search === "string" ? ctx.search : null,
      entries
    );
    var pairs = snapshot.entries;
    var scalarString = function(value) {
      if (typeof value === "symbol") throw new TypeError("Cannot convert a Symbol value to a string");
      return String(value).toWellFormed();
    };
    var readonly = function() { throw new TypeError("Search params are read-only; use router.push or router.replace."); };
    var view = {
      get: function(name) {
        name = scalarString(name);
        for (var pair of pairs) if (pair[0] === name) return pair[1];
        return null;
      },
      getAll: function(name) {
        name = scalarString(name);
        return pairs.filter(function(pair) { return pair[0] === name; }).map(function(pair) { return pair[1]; });
      },
      has: function(name, value) {
        name = scalarString(name);
        var matchValue = value !== undefined;
        if (matchValue) value = scalarString(value);
        return pairs.some(function(pair) { return pair[0] === name && (!matchValue || pair[1] === value); });
      },
      entries: function() { return pairs.map(function(pair) { return pair.slice(); })[Symbol.iterator](); },
      keys: function() { return pairs.map(function(pair) { return pair[0]; })[Symbol.iterator](); },
      values: function() { return pairs.map(function(pair) { return pair[1]; })[Symbol.iterator](); },
      forEach: function(callback, thisArg) {
        if (typeof callback !== "function") throw new TypeError("Search params callback must be a function");
        pairs.forEach(function(pair) { callback.call(thisArg, pair[1], pair[0], view); });
      },
      toString: function() { return snapshot.encoded; },
      size: pairs.length,
      append: readonly,
      delete: readonly,
      set: readonly,
      sort: readonly,
    };
    view[Symbol.iterator] = view.entries;
    return Object.freeze(view);
  }

  // The server has no history and nobody clicking, so every method is a
  // no-op. That is the correct behaviour for a server, not a placeholder:
  // the same component code runs in both places and must not branch.
  function useRouter() {
    return {
      push: function (_href) {},
      replace: function (_href) {},
      back: function () {},
      forward: function () {},
      refresh: function () {},
      prefetch: function (_href) {},
    };
  }

  function Link(props) {
    // Render as plain <a> for SEO / SSR.
    var href = props.href;
    var children = props.children;
    var rest = {};
    for (var k in props) {
      if (k !== "href" && k !== "children") rest[k] = props[k];
    }
    return h("a", Object.assign({ href: href }, rest), children);
  }

  // ---------------------------------------------------------------------------
  // wrapWithPageState — wraps a Page component with page-state context
  // ---------------------------------------------------------------------------
  function wrapWithPageState(Page, input) {
    input = input || {};
    // Set up context so usePageState() in any child component works.
    var ctxValue = Object.assign({}, input, { setPageState: function () {} });
    return h(PageStateContext.Provider, { value: ctxValue }, h(Page, input));
  }

  // ---------------------------------------------------------------------------
  // Install all globals
  // ---------------------------------------------------------------------------
  globalThis.h = h;
  globalThis.Fragment = Fragment;
  globalThis.React = { createElement: h, Fragment: Fragment };
  globalThis.createElement = h;

  globalThis.useState = useState;
  globalThis.useEffect = useEffect;
  globalThis.useLayoutEffect = useLayoutEffect;
  globalThis.useInsertionEffect = useInsertionEffect;
  globalThis.useRef = useRef;
  globalThis.useMemo = useMemo;
  globalThis.useCallback = useCallback;
  globalThis.useContext = useContext;
  globalThis.useReducer = useReducer;
  globalThis.useId = useId;
  globalThis.useImperativeHandle = useImperativeHandle;
  globalThis.useSyncExternalStore = useSyncExternalStore;
  globalThis.forwardRef = forwardRef;
  globalThis.memo = memo;
  globalThis.createPortal = createPortal;

  globalThis.createContext = createContext;
  globalThis.usePageState = createUsePageState();
  globalThis.useRouter = useRouter;
  globalThis.usePathname = usePathname;
  globalThis.useSearchParams = useSearchParams;
  globalThis.Link = Link;
  globalThis.cx = function cx() {
    var out = [];
    for (var i = 0; i < arguments.length; i++) {
      if (arguments[i]) out.push(arguments[i]);
    }
    return out.join(" ");
  };

  // ---------------------------------------------------------------------------
  // Devicon helpers — no-ops during SSR. The marks are a platform
  // stylesheet now; these remain so a template calling them still renders.
  // ---------------------------------------------------------------------------
  globalThis.ensureDevicons = function() {};
  globalThis.dbKindIconClass = function() { return ""; };
  globalThis.dbObjectIconClass = function() { return ""; };

  // ---------------------------------------------------------------------------
  // Page-state bridge — SSR no-ops.
  // The real implementations are installed by build_client_module in render.rs
  // inside __RweRoot after hydration. These stubs prevent ReferenceError when
  // zeb/* library bundles (e.g. zeb/prosemirror) call the bridge during SSR.
  // ---------------------------------------------------------------------------
  globalThis.__rweSetPageState = function() {};
  globalThis.__rwePageState = {};

  // Internal helpers called by Rust after loading each page module.
  // Keep RWE's existing partial-render error contract. Direct engine consumers
  // get exceptions; platform SSR retains its observable component-error marker.
  globalThis.__rweRenderToString = function (node) {
    return renderToString(node, { onError: function (error) {
      return '<!-- RWE component error: ' + escHtml(String(error)).replace(/--/g, '&#45;&#45;') + ' -->';
    } });
  };
  globalThis.__rweWrapWithPageState = wrapWithPageState;

  // ---------------------------------------------------------------------------
  // Island support for SSR — mirrors the client-side h() interceptor.
  // SSR versions always render children but emit the data-island-id marker so
  // the DOM structure matches the client vdom (preventing hydration mismatches).
  // __islandCounter is also reset from Rust in deno_worker.rs before each render.
  // ---------------------------------------------------------------------------
  globalThis.__islandCounter = 0;

  var __origSSRh = globalThis.h;

  function __IslandOff(p) { return p.children; }

  function __IslandOnView(p) {
    return __origSSRh('div', { 'data-island-id': p.id, 'data-hydrate': 'onview' }, p.children);
  }

  function __IslandOnInteract(p) {
    return __origSSRh('div', { 'data-island-id': p.id, 'data-hydrate': 'oninteract' }, p.children);
  }

  globalThis.h = function(type, props) {
    var args = Array.prototype.slice.call(arguments, 2);
    if (props && props.hydrate && props.hydrate !== 'onload') {
      var mode = props.hydrate;
      var id = 'island-' + (globalThis.__islandCounter++);
      var newProps = Object.assign({}, props);
      delete newProps.hydrate;
      newProps['data-island-id'] = id;
      var el = __origSSRh.apply(null, [type, newProps].concat(args));
      if (mode === 'off')        return __IslandOff({ children: el });
      if (mode === 'onview')     return __IslandOnView({ id: id, children: el });
      if (mode === 'oninteract') return __IslandOnInteract({ id: id, children: el });
    }
    return __origSSRh.apply(null, [type, props].concat(args));
  };
})();
