export function createSplitPane(root, options = {}) {
  const handle = root?.querySelector(options.handleSelector || "[data-split-handle]");
  const target = root?.querySelector(options.targetSelector || "[data-split-target]");
  if (!root || !handle || !target) {
    return { destroy() {} };
  }

  const min = options.min ?? 220;
  const max = options.max ?? 420;
  const variable = options.variable ?? "--template-sidebar-width";

  const startDrag = (event) => {
    event.preventDefault();

    const move = (nextEvent) => {
      const rect = root.getBoundingClientRect();
      const width = Math.max(min, Math.min(max, nextEvent.clientX - rect.left));
      root.style.setProperty(variable, `${width}px`);
    };

    const stop = () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", stop);
    };

    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", stop, { once: true });
  };

  handle.addEventListener("pointerdown", startDrag);

  return {
    destroy() {
      handle.removeEventListener("pointerdown", startDrag);
    }
  };
}

export const interact = {
  createSplitPane,
};

