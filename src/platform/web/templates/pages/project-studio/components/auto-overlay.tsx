import { useState, useEffect, cx } from "zeb";
import { subscribeOverlay, autoOverlayState } from "@/pages/project-studio/components/studio-shell-behavior";

export function AutoOverlay() {
  const [s, setS] = useState(autoOverlayState);

  useEffect(() => {
    subscribeOverlay(() => setS({ ...autoOverlayState }));
  }, []);

  if (!s.active) return null;

  return (
    <div className="fixed inset-0 z-[99998] cursor-not-allowed bg-transparent">
      <div
        className="fixed z-[99999] pointer-events-none will-change-transform"
        style={{ transform: `translate(${s.cursorX}px, ${s.cursorY}px)` }}
      >
        <div
          className={cx(
            "h-3 w-3 rounded-full bg-white/90 shadow-lg transition-transform duration-75",
            s.clicking && "scale-[0.82]",
          )}
        />
      </div>
      <div className="fixed bottom-6 left-1/2 z-[99999] -translate-x-1/2 pointer-events-none whitespace-nowrap rounded-full px-[18px] py-[7px] text-xs text-[#e8e8f0] shadow-[0_4px_16px_rgba(0,0,0,0.3)] backdrop-blur-sm bg-[rgba(15,15,25,0.88)]">
        {s.label}
      </div>
      <div
        className="pointer-events-none fixed left-1/2 top-1/2 z-[99999] h-11 w-11 -translate-x-1/2 -translate-y-1/2 rounded-full border-[3px] border-[rgba(255,255,255,0.08)] border-t-brand-blue"
        style={{ animation: "zf-spin 0.75s linear infinite" }}
      />
    </div>
  );
}
