import Button from "@/components/ui/button";
import Input from "@/components/ui/input";
import Kbd from "@/components/ui/kbd";
import Checkbox from "@/components/ui/checkbox";

/**
 * Bottom-docked project console panel.
 *
 * Rendered inside #__rwe_root by the shell template, then teleported to
 * document.body by project-shell.ts so it survives SPA navigations.
 * JS finds elements via data-cli-* attributes.
 */
export default function ConsolePanel({ owner, project, children }) {
  return (
    <div
      className="zf-console-panel bg-[#080b10] border-t border-white/10"
      data-console-panel
      data-owner={owner}
      data-project={project}
      aria-hidden="true"
    >
      {/* Header row */}
      <div className="flex items-center gap-2 px-4 py-1.5 border-b border-white/[0.06] min-h-[2rem] select-none">
        <span className="text-xs font-bold text-slate-500 font-mono">Console</span>
        <span className="inline-flex items-center gap-1 text-slate-700 text-[0.65rem] font-mono">
          <Kbd>`</Kbd>
          <span>toggle</span>
        </span>
        <div className="flex items-center gap-2.5 ml-auto">
          <Checkbox label="High" data-assistant-use-high />
          <Checkbox label="Auto nav" data-auto-navigate defaultChecked />
        </div>
        <Button
          variant="ghost"
          size="icon"
          type="button"
          aria-label="Close console"
          data-console-close
          className="ml-1 text-slate-700 hover:text-slate-400 size-6 text-[0.9rem]"
        >✕</Button>
      </div>

      {/* Output area — ConsoleOutput Preact component rendered as children from layout */}
      <div data-cli-output>{children}</div>

      {/* Input row */}
      <form
        className="flex items-center gap-1.5 px-4 pt-1.5 pb-2 border-t border-white/[0.06] bg-[#080b10]"
        data-cli-form
        autocomplete="off"
      >
        <span
          className="text-green-500 font-mono text-[0.8rem] select-none shrink-0"
          data-cli-prompt
        >zf&gt;</span>
        <Input
          type="text"
          className="flex-1 bg-transparent border-0 outline-none shadow-none rounded-none h-auto px-0 text-slate-200 font-mono text-[0.8rem] caret-green-500 placeholder:text-slate-800 focus-visible:ring-0 focus-visible:border-0"
          data-cli-input
          placeholder="ask or type commands"
          autoComplete="off"
          spellcheck={false}
        />
      </form>
    </div>
  );
}
