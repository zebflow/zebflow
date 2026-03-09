import { useState } from "rwe";

export default function EngineShowcase() {
  const [showDocs, setShowDocs] = useState(true);

  return (
    <div className="min-h-screen bg-zinc-950 text-zinc-100 p-8 font-sans">
      <div className="max-w-3xl mx-auto space-y-8">
        <header className="border-b border-zinc-800 pb-6">
          <h1 className="text-4xl font-black tracking-tight text-indigo-400">RWE Engine</h1>
          <p className="text-zinc-400 mt-2 text-lg">
            The ultimate engine: Preact + Deno SSR + Built-in Markdown.
          </p>
        </header>

        <section className="bg-zinc-900/50 border border-zinc-800 rounded-2xl p-6 shadow-xl">
          <div className="flex items-center justify-between mb-6">
            <h2 className="text-xl font-bold flex items-center gap-2">
              <span className="w-2 h-6 bg-indigo-500 rounded-full"></span>
              Markdown Integration
            </h2>
            <button 
              onClick={() => setShowDocs(!showDocs)}
              className="px-4 py-2 bg-indigo-600 hover:bg-indigo-500 rounded-lg text-sm font-semibold transition-colors"
            >
              {showDocs ? 'Hide Markdown' : 'Show Markdown'}
            </button>
          </div>

          {showDocs && (
            <div className="prose prose-invert max-w-none bg-black/30 rounded-xl p-6 border border-zinc-800/50">
              <markdown>
### RWE v2 Specification
The **Engine** engine implements the following security protocols:

1.  **Strict Sandbox**: No access to `eval()` or `Function` constructors.
2.  **Prototype Protection**: Blocking `__proto__`, `.constructor`, and `.prototype`.
3.  **Scoped Hydration**:
    *   `onload`: Immediate activation.
    *   `onview`: Lazy activation on scroll.
    *   `oninteract`: Activation on first click/key.

#### Feature Matrix
| Feature | Status | Engine |
| :--- | :--- | :--- |
| Preact SSR | ✅ Stable | Engine |
| Markdown | ✅ Native | Engine |
| Security | ✅ High | Engine |

> "The smallest poem is the one that actually runs." — *Anonymous RWE Engineer*
              </markdown>
            </div>
          )}
        </section>

        <footer className="text-center text-zinc-500 text-xs tracking-widest uppercase py-8">
          Powered by rwe • Zebflow Platform 2026
        </footer>
      </div>
    </div>
  );
}
