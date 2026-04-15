export const page = {
  head: {
    title: "{{input.seo.title}}",
    description: "{{input.seo.description}}",
    scripts: [
      { src: "https://unpkg.com/lucide@0.469.0/dist/umd/lucide.min.js" }
    ]
  },
  html: {
    lang: "en",
  },
  body: {
    className: "min-h-screen bg-zinc-50 text-gray-900 font-sans",
  },
  navigation: "history",
};

export const app = (() => {
function setInstall(ctx, tab, cmd) {
    ctx.set("ui.installTab", tab);
    ctx.set("ui.installCommand", cmd);
    ctx.set("ui.isInstallNpm", tab === "npm");
    ctx.set("ui.isInstallCargo", tab === "cargo");
    ctx.set("ui.isInstallShell", tab === "shell");
    ctx.set("ui.isInstallPip", tab === "pip");
    return "ui.installTab";
  }

  function setCodeTab(ctx, tab) {
    ctx.set("ui.activeTab", tab);
    ctx.set("ui.isTabTsx", tab === "tsx");
    ctx.set("ui.isTabZfJson", tab === "zfjson");
    return "ui.activeTab";
  }

  function refreshIcons() {
    if (
      typeof window !== "undefined" &&
      window.lucide &&
      typeof window.lucide.createIcons === "function"
    ) {
      window.lucide.createIcons();
    }
  }

  return {
    state: {
      ui: {
        copied: false,
        installTab: "npm",
        installCommand: "npm i zebflow",
        isInstallNpm: true,
        isInstallCargo: false,
        isInstallShell: false,
        isInstallPip: false,
        activeTab: "tsx",
        isTabTsx: true,
        isTabZfJson: false,
        codeTsx: `export const app = {\n  state: {\n    ui: { title: \"Zebflow Engine\", count: 0 }\n  },\n  actions: {\n    \"counter.inc\": (ctx) => {\n      const n = Number(ctx.get(\"ui.count\") || 0) + 1;\n      ctx.set(\"ui.count\", n);\n      return \"ui.count\";\n    }\n  }\n};\n\nexport default function Page(input) {\n  return (\n    <div className=\"p-4 bg-gray-50\">\n      <h1 zText=\"ui.title\">Loading...</h1>\n      <button onClick=\"counter.inc\" className=\"btn-primary\">\n        Count: <span zText=\"ui.count\">0</span>\n      </button>\n    </div>\n  );\n}`,
        codeZfJson: `{\n  \"id\": \"analysis-pipeline\",\n  \"nodes\": [\n    { \"id\": \"fetch\", \"type\": \"x.n.http.get\" },\n    { \"id\": \"process\", \"type\": \"x.n.script.deno\" }\n  ],\n  \"edges\": [\n    { \"from\": \"fetch.out\", \"to\": \"process.in\" }\n  ]\n}`
      }
    },
    effect: {
      "lucide.refresh": {
        immediate: true,
        deps: ["ui"],
        run: () => {
          refreshIcons();
        }
      }
    },
    actions: {
      "tab.tsx": (ctx) => setCodeTab(ctx, "tsx"),
      "tab.zfjson": (ctx) => setCodeTab(ctx, "zfjson"),
      "install.npm": (ctx) => setInstall(ctx, "npm", "npm i zebflow"),
      "install.cargo": (ctx) => setInstall(ctx, "cargo", "cargo add zebflow"),
      "install.shell": (ctx) => setInstall(ctx, "shell", "curl -fsSL https://zebflow.dev/install | sh"),
      "install.pip": (ctx) => setInstall(ctx, "pip", "pip install zebflow"),
      "copy.install": (ctx) => {
        const cmd = String(ctx.get("ui.installCommand") || "");
        if (typeof navigator !== "undefined" && navigator.clipboard && navigator.clipboard.writeText) {
          navigator.clipboard.writeText(cmd);
        }
        ctx.set("ui.copied", true);
        if (typeof setTimeout === "function") {
          setTimeout(() => {
            if (window.__ZEBFLOW_RWE__ && typeof window.__ZEBFLOW_RWE__.dispatch === "function") {
              window.__ZEBFLOW_RWE__.dispatch("copy.reset");
            }
          }, 2000);
        }
        return "ui.copied";
      },
      "copy.reset": (ctx) => {
        ctx.set("ui.copied", false);
        return "ui.copied";
      }
    }
  };
})();

export default function Page(input) {
  return (
<Page>
    <nav className="fixed top-0 w-full z-50 bg-white/95 backdrop-blur-sm shadow-sm py-3 border-b border-gray-200">
      <div className="max-w-6xl mx-auto px-6 flex justify-between items-center">
        <div className="flex items-center gap-4">
          <div className="text-xl font-bold tracking-tight text-gray-900">
            ZEBFLOW <span className="text-red-700 ml-2 text-sm">トラジュ</span>
          </div>
        </div>
        <div className="hidden md:flex items-center space-x-2">
          <a href="#" className="text-xs font-mono uppercase tracking-widest px-4 py-2 text-gray-500 hover:text-gray-900 transition-colors">Framework</a>
          <a href="#" className="text-xs font-mono uppercase tracking-widest px-4 py-2 text-gray-500 hover:text-gray-900 transition-colors">Language</a>
          <a href="#" className="text-xs font-mono uppercase tracking-widest px-4 py-2 text-gray-500 hover:text-gray-900 transition-colors">RWE</a>
          <button className="ml-6 px-4 py-2 bg-gray-900 text-white text-xs font-mono uppercase tracking-widest hover:bg-red-700 transition-colors">
            Documentation
          </button>
        </div>
      </div>
    </nav>

    <header className="relative pt-36 pb-20 overflow-hidden">
      <div className="max-w-6xl mx-auto px-6 relative z-10 flex flex-col items-center text-center">
        <div className="text-red-700 font-mono text-sm tracking-widest uppercase mb-6 inline-flex items-center gap-2">
          <i data-lucide="activity" className="w-4 h-4 animate-pulse"></i> Automation Engine
        </div>

        <h1 className="text-5xl md:text-5xl font-bold text-gray-900 leading-tight tracking-tight mb-6 uppercase max-w-5xl">
          Deploy Once,<br />
          <span className="text-red-700">Evolve Safely.</span>
        </h1>

        <p className="text-lg text-gray-600 max-w-3xl mb-10 leading-relaxed">
          Zebflow is a tiny drag-and-drop automation engine that lets you build interactive web apps on the fly.
          It combines component modularity, real-time sync, and safe script execution for deployed environments.
        </p>

        <div className="w-full max-w-4xl space-y-4">
          <div className="w-full flex justify-center">
            <button className="w-full md:w-auto px-8 py-4 bg-red-700 text-white text-sm font-bold uppercase tracking-widest hover:bg-gray-900 transition-colors inline-flex items-center justify-center gap-2">
              View Documentation <i data-lucide="arrow-right" className="w-4 h-4"></i>
            </button>
          </div>

          <div className="w-full bg-gray-900 rounded-sm overflow-hidden shadow-2xl text-left">
            <div className="flex bg-black/50 border-b border-gray-800 text-xs font-mono">
              <button onClick="install.npm" className="px-4 py-2 uppercase tracking-wider text-gray-300 hover:text-white transition-colors">
                npm <i data-lucide="circle" zShow="ui.isInstallNpm" className="w-3 h-3 text-red-700 inline-block align-middle"></i>
              </button>
              <button onClick="install.cargo" className="px-4 py-2 uppercase tracking-wider text-gray-300 hover:text-white transition-colors">
                cargo <i data-lucide="circle" zShow="ui.isInstallCargo" className="w-3 h-3 text-red-700 inline-block align-middle"></i>
              </button>
              <button onClick="install.shell" className="px-4 py-2 uppercase tracking-wider text-gray-300 hover:text-white transition-colors">
                shell <i data-lucide="circle" zShow="ui.isInstallShell" className="w-3 h-3 text-red-700 inline-block align-middle"></i>
              </button>
              <button onClick="install.pip" className="px-4 py-2 uppercase tracking-wider text-gray-300 hover:text-white transition-colors">
                pip <i data-lucide="circle" zShow="ui.isInstallPip" className="w-3 h-3 text-red-700 inline-block align-middle"></i>
              </button>
            </div>
            <div className="p-4 flex items-center justify-between group">
              <code className="text-gray-300 font-mono text-sm">
                <span className="text-red-700 mr-2">$</span><span zText="ui.installCommand">npm i zebflow</span>
              </code>
              <button onClick="copy.install" className="text-gray-400 hover:text-white transition-colors" title="Copy to clipboard">
                <i data-lucide="copy" zHide="ui.copied" className="w-4 h-4"></i>
                <i data-lucide="check-check" zShow="ui.copied" className="w-4 h-4 text-green-500"></i>
              </button>
            </div>
          </div>
        </div>
      </div>
    </header>

    <section className="py-20 bg-white border-y border-gray-200">
      <div className="max-w-6xl mx-auto px-6">
        <div className="mb-12 text-center">
          <h2 className="text-4xl font-bold text-gray-900 uppercase tracking-tight">Everything you need to evolve</h2>
          <p className="text-gray-500 mt-4 max-w-2xl mx-auto">A unified framework supporting SSR, SPA, or SSG architectures.</p>
        </div>
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
          <article className="group relative bg-white border border-gray-200 p-8 hover:border-red-700 hover:shadow-lg transition-all duration-300">
            <div className="mb-5 inline-flex items-center gap-3">
              <div className="p-3 bg-gray-50 group-hover:bg-red-700/10 transition-colors rounded-sm"><i data-lucide="globe" className="w-5 h-5 text-red-700"></i></div>
              <h3 className="text-lg font-bold text-gray-900 uppercase tracking-tight">Reactive Web</h3>
            </div>
            <p className="text-sm text-gray-600 leading-relaxed">Lean hydration, component modularity, and SSR support.</p>
          </article>
          <article className="group relative bg-white border border-gray-200 p-8 hover:border-red-700 hover:shadow-lg transition-all duration-300">
            <div className="mb-5 inline-flex items-center gap-3">
              <div className="p-3 bg-gray-50 group-hover:bg-red-700/10 transition-colors rounded-sm"><i data-lucide="zap" className="w-5 h-5 text-red-700"></i></div>
              <h3 className="text-lg font-bold text-gray-900 uppercase tracking-tight">Live State Sync</h3>
            </div>
            <p className="text-sm text-gray-600 leading-relaxed">Concurrent multi-user interactions over WebSockets.</p>
          </article>
          <article className="group relative bg-white border border-gray-200 p-8 hover:border-red-700 hover:shadow-lg transition-all duration-300">
            <div className="mb-5 inline-flex items-center gap-3">
              <div className="p-3 bg-gray-50 group-hover:bg-red-700/10 transition-colors rounded-sm"><i data-lucide="shield" className="w-5 h-5 text-red-700"></i></div>
              <h3 className="text-lg font-bold text-gray-900 uppercase tracking-tight">Agentic Pipelines</h3>
            </div>
            <p className="text-sm text-gray-600 leading-relaxed">Visually orchestrate intelligent multi-step workflows.</p>
          </article>
          <article className="group relative bg-white border border-gray-200 p-8 hover:border-red-700 hover:shadow-lg transition-all duration-300">
            <div className="mb-5 inline-flex items-center gap-3">
              <div className="p-3 bg-gray-50 group-hover:bg-red-700/10 transition-colors rounded-sm"><i data-lucide="activity" className="w-5 h-5 text-red-700"></i></div>
              <h3 className="text-lg font-bold text-gray-900 uppercase tracking-tight">Data Workers</h3>
            </div>
            <p className="text-sm text-gray-600 leading-relaxed">Run ad-hoc analysis and background processing safely.</p>
          </article>
        </div>
      </div>
    </section>

    <section className="py-20 bg-zinc-50">
      <div className="max-w-6xl mx-auto px-6">
        <div className="flex items-end justify-between mb-8 pb-4 border-b border-gray-200">
          <h2 className="text-3xl font-bold text-gray-900 uppercase tracking-tight">Core Architecture Modules</h2>
          <span className="text-xs font-mono text-gray-500 uppercase tracking-widest hidden md:block">crates/zebflow/src</span>
        </div>
        <div className="grid md:grid-cols-3 gap-6">
          <article className="bg-white border border-gray-200 p-8 shadow-sm">
            <h3 className="font-mono font-bold text-gray-900 mb-4 text-lg border-b border-gray-100 pb-3">1. framework</h3>
            <p className="text-gray-600 text-sm leading-relaxed">Execution control plane with deterministic graph tracing.</p>
          </article>
          <article className="bg-white border border-gray-200 p-8 shadow-sm">
            <h3 className="font-mono font-bold text-gray-900 mb-4 text-lg border-b border-gray-100 pb-3">2. language</h3>
            <p className="text-gray-600 text-sm leading-relaxed">Portable sandbox runtime with strict policy controls.</p>
          </article>
          <article className="bg-white border border-gray-200 p-8 shadow-sm">
            <h3 className="font-mono font-bold text-gray-900 mb-4 text-lg border-b border-gray-100 pb-3">3. rwe</h3>
            <p className="text-gray-600 text-sm leading-relaxed">Compiles `.tsx` templates with lean hydration and processors.</p>
          </article>
        </div>
      </div>
    </section>

    <section className="py-20 bg-gray-900 text-white border-y border-gray-800">
      <div className="max-w-6xl mx-auto px-6 grid lg:grid-cols-2 gap-12 items-start">
        <div>
          <h2 className="text-3xl font-bold mb-6 uppercase tracking-tight">Standard Conventions</h2>
          <p className="text-base text-gray-400 mb-8 leading-relaxed max-w-lg">
            Zebflow enforces strict boundaries through contract files, separating visual orchestration, rendering, and script execution.
          </p>
          <div className="space-y-4">
            <div className="flex items-start gap-4 p-5 border border-white/10 bg-black/20 rounded-sm">
              <div className="mt-1 text-red-700"><i data-lucide="terminal" className="w-4 h-4"></i></div>
              <div>
                <div className="font-mono text-sm mb-2 text-white">*.zf.json</div>
                <div className="text-sm text-gray-400">Pin-based edge definitions for graph execution.</div>
              </div>
            </div>
            <div className="flex items-start gap-4 p-5 border border-white/10 bg-black/20 rounded-sm">
              <div className="mt-1 text-red-700"><i data-lucide="terminal" className="w-4 h-4"></i></div>
              <div>
                <div className="font-mono text-sm mb-2 text-white">*.tsx</div>
                <div className="text-sm text-gray-400">Reactive templates containing HTML, CSS, and JS logic.</div>
              </div>
            </div>
            <div className="flex items-start gap-4 p-5 border border-white/10 bg-black/20 rounded-sm">
              <div className="mt-1 text-red-700"><i data-lucide="terminal" className="w-4 h-4"></i></div>
              <div>
                <div className="font-mono text-sm mb-2 text-white">secure script runtime</div>
                <div className="text-sm text-gray-400">Sandbox boundary for safe isolated script nodes.</div>
              </div>
            </div>
          </div>
        </div>

        <div className="bg-black border border-white/10 shadow-2xl rounded-sm overflow-hidden lg:mt-10">
          <div className="flex items-center border-b border-white/10">
            <button onClick="tab.tsx" className="flex-1 px-4 py-4 text-xs font-mono tracking-widest uppercase text-gray-400 hover:text-white transition-colors">
              ui_component.tsx <i data-lucide="circle" zShow="ui.isTabTsx" className="w-3 h-3 text-red-700 inline-block align-middle"></i>
            </button>
            <button onClick="tab.zfjson" className="flex-1 px-4 py-4 text-xs font-mono tracking-widest uppercase text-gray-400 hover:text-white transition-colors">
              agent_flow.zf.json <i data-lucide="circle" zShow="ui.isTabZfJson" className="w-3 h-3 text-red-700 inline-block align-middle"></i>
            </button>
          </div>
          <div className="p-6 font-mono text-sm leading-relaxed overflow-auto h-[360px]">
            <pre zShow="ui.isTabTsx" className="text-gray-300" zText="ui.codeTsx"><code></code></pre>
            <pre zShow="ui.isTabZfJson" className="text-gray-300" zText="ui.codeZfJson"><code></code></pre>
          </div>
        </div>
      </div>
    </section>

    <footer className="py-12 bg-white">
      <div className="max-w-6xl mx-auto px-6">
        <div className="flex flex-col md:flex-row justify-between items-center gap-6 text-xs font-mono text-gray-500">
          <div className="flex items-center gap-2">
            <span className="font-bold text-gray-900 text-sm">ZEBFLOW</span>
            <span className="text-gray-300">|</span>
            DEPLOY ONCE, EVOLVE SAFELY
          </div>
          <div className="flex flex-wrap justify-center gap-8">
            <a href="#" className="hover:text-red-700 transition-colors">GitHub</a>
            <a href="#" className="hover:text-red-700 transition-colors">Crates.io</a>
            <a href="#" className="hover:text-red-700 transition-colors">Documentation</a>
            <a href="#" className="hover:text-red-700 transition-colors">License</a>
          </div>
        </div>
      </div>
    </footer>
</Page>
  );
}
