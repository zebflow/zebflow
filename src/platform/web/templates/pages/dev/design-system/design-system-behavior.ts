/**
 * Design System page behavior.
 * Handles copy-to-clipboard for code blocks only.
 * Section nav is managed by Preact useState in the page component.
 */

export function initDesignSystemBehavior() {
  if (typeof document === "undefined") return;
  const run = () => {
    initCopyButtons();
  };
  if (typeof requestAnimationFrame === "function") {
    requestAnimationFrame(run);
  } else {
    setTimeout(run, 0);
  }
}

function initCopyButtons() {
  document.querySelectorAll<HTMLElement>("[data-copy-btn]").forEach((btn) => {
    if ((btn as any)._copyBound) return;
    (btn as any)._copyBound = true;

    btn.addEventListener("click", async () => {
      const wrapper = btn.closest("[data-code-wrapper]");
      const codeEl = wrapper?.querySelector("[data-code-block]");
      const text = (codeEl?.textContent ?? "").trim();
      if (!text) return;

      try {
        await navigator.clipboard.writeText(text);
        const orig = btn.textContent;
        btn.textContent = "copied!";
        setTimeout(() => {
          btn.textContent = orig;
        }, 1500);
      } catch {
        console.warn("[design-system] clipboard write failed");
      }
    });
  });
}
