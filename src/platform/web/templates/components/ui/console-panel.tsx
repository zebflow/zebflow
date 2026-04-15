import { useEffect, cx } from "zeb";
import Button from "@/components/ui/button";
import Kbd from "@/components/ui/kbd";
import Checkbox from "@/components/ui/checkbox";

/**
 * Bottom-docked project console panel.
 *
 * Rendered inside #__rwe_root by the shell template, then teleported to
 * document.body by project-shell.ts so it survives SPA navigations.
 * Open/close driven by isOpen prop from StudioChromeContext via shell.tsx ConsoleSlot.
 */
export default function ConsolePanel({ owner, project, isOpen, children }) {
  // Focus the CLI input after the panel becomes visible
  useEffect(() => {
    if (!isOpen) return;
    const input = document.querySelector<HTMLInputElement>("[data-cli-input]");
    setTimeout(() => input?.focus(), 40);
  }, [isOpen]);

  return (
    <div
      className={cx(
        "fixed bottom-0 left-0 right-0 z-[1000] flex flex-col bg-[#080b10] border-t border-white/10 transition",
        isOpen ? "max-h-[40vh]" : "max-h-0 overflow-hidden",
      )}
      tw-variants="max-h-0 overflow-hidden max-h-[40vh]"
      data-console-panel
      data-owner={owner}
      data-project={project}
      aria-hidden={isOpen ? "false" : "true"}
    >
      {/* Header row */}
      <div className="flex items-center gap-2 px-4 py-1.5 border-b border-white/[0.06] min-h-[2rem] select-none">
        <span className="text-xs font-bold text-gray-500 font-mono">Console</span>
        <span className="inline-flex items-center gap-1 text-gray-700 text-[0.65rem] font-mono">
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
          className="ml-1 text-gray-700 hover:text-gray-400 size-6 text-[0.9rem]"
        >✕</Button>
      </div>

      {/* Output area — ConsoleOutput Preact component rendered as children from layout */}
      <div data-cli-output className="flex-1 min-h-0 overflow-y-auto">{children}</div>

      {/* Autocomplete suggestions — shown above the input row when typing */}
      <div data-cli-autocomplete hidden className="border-t border-white/[0.04] bg-[#090d14] overflow-hidden" />

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
        {/* Raw input — avoids Input component's bg-white base class overriding bg-transparent */}
        <input
          type="text"
          className="flex-1 min-w-0 bg-transparent border-none outline-none font-mono text-[0.82rem] text-green-300 placeholder:text-gray-600 caret-green-400"
          data-cli-input
          placeholder="ask or type commands"
          autoComplete="off"
          spellcheck={false}
        />
      </form>
    </div>
  );
}
