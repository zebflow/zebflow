import PlatformSidebar from "@/components/platform-sidebar";

export const app = {};

export default function AdminWrapper(props) {
  return (
<div>
    <PlatformSidebar />

    <main className="ml-16 min-h-screen">
      <header className="sticky top-0 z-10 bg-white/95 backdrop-blur-sm border-b border-slate-200 px-4 lg:px-5 py-3">
        <div className="flex items-end justify-between gap-4">
          <div className="min-w-0">
            <div className="flex items-center gap-2 text-[11px] font-mono uppercase tracking-[0.16em] text-slate-500">
              <a href="/home" className="inline-flex items-center justify-center w-4 h-4 text-slate-500 hover:text-[#005B9A] transition-colors">
                <svg viewBox="0 0 24 24" fill="none" className="w-3.5 h-3.5">
                  <path d="M4 10.5L12 4l8 6.5V20H4z" stroke="currentColor" stroke-width="1.8" stroke-linejoin="round"/>
                </svg>
              </a>
              <span>/</span>
              <a href="{props.backHref}" className="truncate text-[#005B9A] hover:underline">{props.backLabel}</a>
            </div>
            <div className="mt-1 flex items-baseline gap-3 min-w-0">
              <h1 className="truncate text-lg lg:text-xl font-bold tracking-tight text-slate-900">{props.title}</h1>
              <p className="truncate text-xs text-slate-500">{props.meta}</p>
            </div>
          </div>

          <a
            href="{props.actionHref}"
            className="{props.actionClass} shrink-0 px-3 py-1.5 rounded-md bg-[#005B9A] text-white text-[11px] font-mono uppercase tracking-[0.16em] hover:bg-[#004A7A] transition-colors"
          >
            {props.actionLabel}
          </a>
        </div>
      </header>

      <section className="p-3 lg:p-4 space-y-3">
        {props.children}
      </section>
    </main>

    <details className="{props.chatClass} fixed right-5 bottom-5 z-40">
      <summary className="list-none cursor-pointer">
        <span className="inline-flex items-center justify-center w-12 h-12 rounded-full bg-[#005B9A] text-white shadow-xl hover:bg-[#004A7A] transition-colors">
          <svg viewBox="0 0 24 24" fill="none" className="w-5 h-5">
            <path d="M8 10h8M8 14h5M7 5h10a3 3 0 013 3v6a3 3 0 01-3 3h-4l-4 3v-3H7a3 3 0 01-3-3V8a3 3 0 013-3z" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>
        </span>
      </summary>

      <div className="mt-2 w-[320px] rounded-xl border border-slate-200 bg-white shadow-2xl overflow-hidden">
        <div className="px-3 py-2.5 border-b border-slate-200 bg-slate-50">
          <p className="text-sm font-bold tracking-tight text-slate-900">Zebflow Assistant</p>
          <p className="text-xs text-slate-500 mt-1">Context-aware project help, pipeline guidance, and template support.</p>
        </div>

        <div className="p-3 space-y-2">
          <div className="rounded-lg border border-slate-200 bg-slate-50 p-3">
            <p className="text-xs font-mono uppercase tracking-widest text-slate-500">Suggested Actions</p>
            <ul className="mt-3 space-y-2 text-sm text-slate-700">
              <li>Inspect active pipelines</li>
              <li>Open the current template tree</li>
              <li>Summarize schema decisions</li>
            </ul>
          </div>

          <div className="rounded-lg border border-slate-200 p-3">
            <p className="text-xs font-mono uppercase tracking-widest text-slate-500">Chat Runtime</p>
            <p className="mt-2 text-sm text-slate-700">
              Interactive assistant wiring lives here. The launcher is platform-wide, while project context stays scoped to the current route.
            </p>
          </div>
        </div>
      </div>
    </details>
  </div>
  );
}
