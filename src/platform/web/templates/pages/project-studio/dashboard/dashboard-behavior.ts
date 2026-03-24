function fmtBytes(n) {
  if (!Number.isFinite(Number(n)) || n <= 0) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  let val = Number(n);
  let idx = 0;
  while (val >= 1024 && idx + 1 < units.length) {
    val = val / 1024;
    idx += 1;
  }
  return val.toFixed(1) + " " + units[idx];
}

function fmtUptime(secs) {
  const s = Math.floor(Number(secs) || 0);
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const sec = s % 60;
  if (h > 0) return h + "h " + m + "m";
  if (m > 0) return m + "m " + sec + "s";
  return sec + "s";
}

function clamp01(v) {
  const n = Number(v) / 100;
  if (n < 0) return 0;
  if (n > 1) return 1;
  return n;
}

function setBar(root, selector, pct) {
  const bar = root.querySelector(selector);
  if (!bar) return;
  bar.style.width = Math.round(clamp01(pct) * 100) + "%";
}

function setText(root, selector, text) {
  const el = root.querySelector(selector);
  if (!el) return;
  el.textContent = String(text || "");
}

function escHtml(s) {
  return String(s || "")
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");
}

function renderCapItems(items) {
  return items.map(function(item) {
    const dotCls = item.installed ? "dash-cap-dot-on" : "dash-cap-dot-off";
    const statusText = item.installed ? "Installed" : "Not installed";
    const detail = item.detail ? " \u00b7 " + escHtml(item.detail) : "";
    return (
      '<div class="dash-cap-item">' +
        '<span class="' + dotCls + '"></span>' +
        '<span class="dash-cap-name">' + escHtml(item.label) + '</span>' +
        '<span class="dash-cap-status">' + statusText + detail + '</span>' +
      '</div>'
    );
  }).join("");
}

function renderCapSection(label, items) {
  return '<p class="dash-cap-section-label">' + escHtml(label) + '</p>' + renderCapItems(items);
}

function capOf(obj, key) {
  const c = obj && obj[key];
  return { installed: !!(c && c.installed), detail: (c && c.version) || "" };
}

function renderCapabilities(root, caps) {
  const grid = root.querySelector("[data-caps-grid]");
  if (!grid) return;

  const sec = caps.security || {};

  const runtimeItems = [
    { label: "Python (system)",  ...capOf({ python: { installed: caps.python && caps.python.available, version: caps.python && caps.python.version } }, "python") },
    { label: "Python (managed)", installed: !!(caps.python_managed && caps.python_managed.available), detail: "" },
    { label: "Lightpanda",       ...capOf(caps, "lightpanda") },
    { label: "Chromium",         installed: !!(caps.chromium && caps.chromium.installed), detail: "" },
    { label: "Ollama",           ...capOf(caps, "ollama") },
    { label: "SearXNG",          installed: !!(caps.searxng && caps.searxng.installed), detail: "" },
    { label: "vips",             ...capOf(caps, "vips") },
  ];

  const securityItems = [
    { label: "nmap",    ...capOf(sec, "nmap")    },
    { label: "nuclei",  ...capOf(sec, "nuclei")  },
    { label: "httpx",   ...capOf(sec, "httpx")   },
    { label: "trivy",   ...capOf(sec, "trivy")   },
    { label: "masscan", ...capOf(sec, "masscan") },
    { label: "ffuf",    ...capOf(sec, "ffuf")    },
    { label: "sqlmap",  ...capOf(sec, "sqlmap")  },
    { label: "nikto",   ...capOf(sec, "nikto")   },
  ];

  grid.innerHTML =
    renderCapSection("Runtime", runtimeItems) +
    renderCapSection("Security", securityItems);
}

function renderSysinfo(root, data) {
  if (!data || !data.ok) {
    setText(root, "[data-dash-status]", "Failed to load system info.");
    return;
  }

  const sys = data.system || {};
  const os = sys.os || {};
  const cpu = sys.cpu || {};
  const mem = sys.memory || {};
  const disk = sys.disk || {};
  const proc = data.process || {};
  const caps = data.capabilities || {};

  setText(root, "[data-os-variant]", os.variant || "-");
  setText(root, "[data-os-name]", (os.name || "-") + " " + (os.version || "").trim());
  setText(root, "[data-os-arch]", os.arch || "-");
  setText(root, "[data-os-hostname]", os.hostname || "-");
  const container = os.container || {};
  if (container.in_docker) {
    const gw = container.host_gateway ? " \u00b7 host " + container.host_gateway : "";
    setText(root, "[data-env-container]", "Docker" + gw);
  } else {
    setText(root, "[data-env-container]", "");
  }

  setText(root, "[data-cpu-brand]", cpu.brand || "-");
  setText(root, "[data-cpu-cores]", cpu.cores ? cpu.cores + " cores" : "-");
  setText(root, "[data-cpu-pct]", (cpu.usage_pct || 0).toFixed(1) + "%");
  setBar(root, "[data-cpu-bar]", cpu.usage_pct || 0);

  setText(root, "[data-mem-used]", fmtBytes(mem.used_bytes));
  setText(root, "[data-mem-total]", fmtBytes(mem.total_bytes));
  setText(root, "[data-mem-pct]", (mem.usage_pct || 0).toFixed(1) + "%");
  setBar(root, "[data-mem-bar]", mem.usage_pct || 0);

  setText(root, "[data-disk-used]", fmtBytes(disk.used_bytes));
  setText(root, "[data-disk-total]", fmtBytes(disk.total_bytes));
  setText(root, "[data-disk-pct]", (disk.usage_pct || 0).toFixed(1) + "%");
  setBar(root, "[data-disk-bar]", disk.usage_pct || 0);

  setText(root, "[data-proc-pid]", proc.pid || "-");
  setText(root, "[data-proc-cpu]", (proc.cpu_pct || 0).toFixed(1) + "%");
  setText(root, "[data-proc-mem]", fmtBytes(proc.memory_bytes));
  setText(root, "[data-proc-threads]", proc.threads || "-");
  setText(root, "[data-proc-uptime]", fmtUptime(proc.uptime_seconds));

  renderCapabilities(root, caps);
  setText(root, "[data-dash-status]", "");
}

async function fetchAndRender(root, apiUrl) {
  try {
    setText(root, "[data-dash-status]", "Refreshing...");
    const res = await fetch(apiUrl, { headers: { Accept: "application/json" } });
    const data = await res.json().catch(function() { return null; });
    renderSysinfo(root, data);
  } catch (err) {
    setText(root, "[data-dash-status]", "Error: " + (err && err.message ? err.message : String(err)));
  }
}

const _dashSet = new WeakSet();

function scanDashboardRoots() {
  document.querySelectorAll("[data-dashboard-root]").forEach(function(root) {
    if (_dashSet.has(root)) return;
    _dashSet.add(root);
    const apiUrl = root.getAttribute("data-api-system-info") || "/api/system/info";
    fetchAndRender(root, apiUrl);
    setInterval(function() {
      fetchAndRender(root, apiUrl);
    }, 30000);
  });
}

export function initDashboardBehavior() {
  if (typeof (globalThis as Record<string, unknown>)["Deno"] !== "undefined") return;
  if (typeof document === "undefined") return;
  if (typeof requestAnimationFrame === "function") {
    requestAnimationFrame(scanDashboardRoots);
  } else {
    setTimeout(scanDashboardRoots, 0);
  }
}
