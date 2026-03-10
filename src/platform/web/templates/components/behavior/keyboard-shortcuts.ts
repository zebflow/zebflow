/**
 * Extendable keyboard shortcut registry.
 *
 * Usage:
 *   registerShortcut({ key: "`", description: "Toggle console", action: () => ... });
 *   initKeyboardShortcuts(); // call once
 *
 * Shortcuts do NOT fire when the user is typing in an input/textarea,
 * with the exception of the console input itself (so ` closes it from there too).
 */

export interface ShortcutDef {
  /** The key value (e.g. "`", "Escape", "k"). */
  key: string;
  ctrl?: boolean;
  meta?: boolean;
  shift?: boolean;
  description: string;
  action: () => void;
}

const registry: ShortcutDef[] = [];
let installed = false;

/** Register a global keyboard shortcut. Safe to call before initKeyboardShortcuts(). */
export function registerShortcut(def: ShortcutDef): void {
  registry.push(def);
}

/** Install the global keydown listener. Idempotent — safe to call multiple times. */
export function initKeyboardShortcuts(): void {
  if (installed || typeof window === "undefined") return;
  installed = true;

  window.addEventListener(
    "keydown",
    (e: KeyboardEvent) => {
      const active = document.activeElement;
      const inInput =
        active instanceof HTMLInputElement ||
        active instanceof HTMLTextAreaElement ||
        (active instanceof HTMLElement && active.isContentEditable);

      // Inside the console input, only allow the toggle key to close it.
      // For all other inputs, skip shortcuts entirely.
      const inConsoleInput =
        inInput && !!(active as HTMLElement).closest?.(".zf-console-panel");

      if (inInput && !inConsoleInput) return;

      for (const s of registry) {
        if (s.key !== e.key) continue;
        if (!!s.ctrl !== e.ctrlKey) continue;
        if (!!s.meta !== e.metaKey) continue;
        if (!!s.shift !== e.shiftKey) continue;
        e.preventDefault();
        s.action();
        break;
      }
    },
    { capture: true },
  );
}
