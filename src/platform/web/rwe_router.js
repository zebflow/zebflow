/**
 * RWE SPA router — fragment swap with progress bar + page cache.
 *
 * - data-rwe-nav links navigate without full reload
 * - data-rwe-outlet is the swap target in the shell
 * - Thin top progress bar gives instant visual feedback
 * - In-memory page cache (10s TTL, max 12 entries) makes back-nav instant
 */
(function () {
  "use strict";

  var outlet = document.querySelector("[data-rwe-outlet]");
  if (!outlet) return;

  // ── Progress bar ────────────────────────────────────────────────────────────

  var bar = document.createElement("div");
  bar.id = "rwe-nav-bar";
  Object.assign(bar.style, {
    position: "fixed",
    top: "0",
    left: "0",
    height: "2px",
    width: "0%",
    background: "var(--zf-color-brand-blue, #005b9a)",
    zIndex: "99999",
    opacity: "0",
    transition: "none",
    pointerEvents: "none",
  });
  document.body.appendChild(bar);

  var barTimer = null;

  function progressStart() {
    clearTimeout(barTimer);
    bar.style.transition = "none";
    bar.style.width = "0%";
    bar.style.opacity = "1";
    bar.offsetWidth; // force reflow to reset transition
    bar.style.transition = "width 0.25s ease";
    bar.style.width = "30%";
    barTimer = setTimeout(function () {
      bar.style.transition = "width 1.5s ease";
      bar.style.width = "65%";
    }, 250);
  }

  function progressDone() {
    clearTimeout(barTimer);
    bar.style.transition = "width 0.12s ease";
    bar.style.width = "100%";
    barTimer = setTimeout(function () {
      bar.style.transition = "opacity 0.2s ease";
      bar.style.opacity = "0";
    }, 180);
  }

  function progressFail() {
    clearTimeout(barTimer);
    bar.style.transition = "opacity 0.15s ease";
    bar.style.opacity = "0";
  }

  // ── Page cache ───────────────────────────────────────────────────────────────

  var pageCache = new Map(); // url → { html: string, t: number }
  var CACHE_TTL = 12000;     // 12 seconds
  var CACHE_MAX = 12;

  function getCached(url) {
    var entry = pageCache.get(url);
    if (!entry) return null;
    if (Date.now() - entry.t > CACHE_TTL) { pageCache.delete(url); return null; }
    return entry.html;
  }

  function setCache(url, html) {
    if (pageCache.size >= CACHE_MAX) {
      pageCache.delete(pageCache.keys().next().value);
    }
    pageCache.set(url, { html: html, t: Date.now() });
  }

  /** Invalidate cache for a URL (call after mutations on that page). */
  window.rweInvalidate = function (url) {
    if (url) { pageCache.delete(url); } else { pageCache.clear(); }
  };

  /** Programmatic navigation — usable by the assistant. */
  window.rweNavigate = function (href) { navigate(href); };

  // ── DOM helpers ──────────────────────────────────────────────────────────────

  function loadNewScripts(doc) {
    // Track loaded non-module scripts by pathname to avoid duplicates
    var loadedPaths = Array.from(document.querySelectorAll("script[src]"))
      .map(function (s) {
        try { return new URL(s.src).pathname; } catch (_) { return s.getAttribute("src") || ""; }
      });

    doc.querySelectorAll("script[src]").forEach(function (old) {
      var rawSrc = old.getAttribute("src");
      if (!rawSrc) return;

      // Resolve to absolute URL using current window origin
      var absUrl;
      try { absUrl = new URL(rawSrc, window.location.href).href; }
      catch (_) { return; }

      var s = document.createElement("script");
      if (old.type) s.type = old.type;

      if (old.type === "module") {
        // Module scripts are cached by URL — add cache-bust param so they
        // re-execute and re-run Preact hydrate + behavior inits on each nav.
        s.src = absUrl + (absUrl.indexOf("?") >= 0 ? "&" : "?") + "_rwe_nav=" + Date.now();
      } else {
        // Non-module scripts: skip if same path already loaded
        var pathname;
        try { pathname = new URL(absUrl).pathname; } catch (_) { pathname = rawSrc; }
        if (loadedPaths.indexOf(pathname) !== -1) return;
        s.src = absUrl;
      }

      document.head.appendChild(s);
    });

    // Inline Preact runtime bundles (data-rwe-runtime) are not src-based so
    // innerHTML won't execute them. Re-create as a new script to run them.
    doc.querySelectorAll("script[data-rwe-runtime]").forEach(function (old) {
      var s = document.createElement("script");
      s.textContent = old.textContent;
      document.head.appendChild(s);
    });
  }

  function updateActive(url) {
    var path = url.split("?")[0];
    document.querySelectorAll("[data-rwe-nav]").forEach(function (link) {
      var href = (link.getAttribute("href") || "").split("?")[0];
      if (!href || href === "#") return;
      var active = path === href || path.startsWith(href + "/");
      link.classList.toggle("is-active", active);
    });
  }

  // Close all submenu groups after navigation so the menu collapses.
  function updateDetails() {
    document.querySelectorAll("details[data-group]").forEach(function (details) {
      details.open = false;
    });
  }

  function updateBreadcrumb(doc) {
    var next = doc.querySelector("[data-rwe-breadcrumb]");
    var curr = document.querySelector("[data-rwe-breadcrumb]");
    if (next && curr) curr.textContent = next.textContent;
  }

  function applyPage(href, html) {
    var doc = new DOMParser().parseFromString(html, "text/html");

    // Swap the full __rwe_root so Preact hydration sees correct SSR HTML,
    // AND update __rwe_payload so the re-executed module reads fresh input data.
    // (Both elements are siblings outside [data-rwe-outlet] — only updating the
    // outlet misses the payload, causing behaviors to init with stale data.)
    var newRoot = doc.getElementById("__rwe_root");
    var newPayload = doc.getElementById("__rwe_payload");
    var liveRoot = document.getElementById("__rwe_root");

    if (newRoot && liveRoot) {
      liveRoot.innerHTML = newRoot.innerHTML;
      if (newPayload) {
        var livePayload = document.getElementById("__rwe_payload");
        if (livePayload) livePayload.textContent = newPayload.textContent;
      }
      // Re-anchor outlet reference after innerHTML replaced its DOM node.
      outlet = document.querySelector("[data-rwe-outlet]") || outlet;
    } else {
      // Fallback for pages without __rwe_root (plain HTML pages).
      var newOutlet = doc.querySelector("[data-rwe-outlet]");
      if (!newOutlet) { window.location.href = href; return; }
      outlet.innerHTML = newOutlet.innerHTML;
    }

    loadNewScripts(doc);
    document.title = doc.title;
    updateActive(href);
    updateDetails();
    updateBreadcrumb(doc);
    window.scrollTo(0, 0);
    window.dispatchEvent(new CustomEvent("rwe:nav", { detail: { url: href } }));
  }

  // ── Navigation ───────────────────────────────────────────────────────────────

  function navigate(href) {
    progressStart();

    var cached = getCached(href);
    if (cached) {
      history.pushState(null, "", href);
      applyPage(href, cached);
      progressDone();
      return;
    }

    fetch(href, { credentials: "same-origin" })
      .then(function (r) {
        if (!r.ok) { progressFail(); window.location.href = href; return null; }
        return r.text();
      })
      .then(function (html) {
        if (!html) return;
        setCache(href, html);
        history.pushState(null, "", href);
        applyPage(href, html);
        progressDone();
      })
      .catch(function () { progressFail(); window.location.href = href; });
  }

  document.addEventListener("click", function (e) {
    var link = e.target.closest("[data-rwe-nav]");
    if (!link) return;
    var href = link.getAttribute("href");
    if (!href || href === "#") return;
    if (e.metaKey || e.ctrlKey || e.shiftKey || e.altKey) return;
    if (href.startsWith("http") || href.startsWith("//")) return;
    var current = window.location.pathname + window.location.search;
    if (href === current) return;
    e.preventDefault();
    navigate(href);
  });

  window.addEventListener("popstate", function () {
    navigate(window.location.pathname + window.location.search);
  });
})();
