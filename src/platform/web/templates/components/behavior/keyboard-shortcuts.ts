/**
 * Extendable keyboard shortcut registry.
 *
 * Usage:
 *   registerShortcut({ key: "`", description: "Toggle console", action: () => ... });
 *   initKeyboardShortcuts(); // call once
 *
 * Shortcuts do NOT fire when the user is typing in an input/textarea,
 * with the exception of the console input itself (so ` closes it from there too).
 *
 * Registry and installed-flag are stored on window so they survive SPA navigation,
 * which re-executes module bundles on every page change.
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

const WIN: any = typeof window !== "undefined" ? window : {};

function getRegistry(): ShortcutDef[] {
  if (!WIN.__zf_shortcuts) WIN.__zf_shortcuts = [];
  return WIN.__zf_shortcuts as ShortcutDef[];
}

/** Register a global keyboard shortcut. Safe to call multiple times — deduplicates by key+description. */
export function registerShortcut(def: ShortcutDef): void {
  const reg = getRegistry();
  const existing = reg.find((r) => r.key === def.key && r.description === def.description);
  if (existing) {
    existing.action = def.action; // update to latest action on re-registration
  } else {
    reg.push(def);
  }
}

/** Install the global keydown listener. Idempotent across module re-evaluations. */
export function initKeyboardShortcuts(): void {
  if (WIN.__zf_shortcuts_installed || typeof window === "undefined") return;
  WIN.__zf_shortcuts_installed = true;

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

      const registry = getRegistry();
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
