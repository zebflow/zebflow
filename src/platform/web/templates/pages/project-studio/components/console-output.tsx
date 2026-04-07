import { useState, useEffect, useRef, cx } from "zeb";
import { subscribeConsole, getConsoleLines, navigate } from "@/pages/project-studio/components/studio-shell-behavior";

const LINE_STYLES: Record<string, string> = {
  "cli-echo":     "text-slate-400",
  "cli-info":     "text-slate-500 italic",
  "cli-error":    "text-red-400",
  "cli-success":  "text-green-400",
  "cli-muted":    "text-slate-500",
  "cli-blank":    "block h-[0.6em]",
  "cli-ai":       "text-sky-300 whitespace-pre-wrap break-words",
  "cli-tool":     "text-indigo-400 italic",
  "cli-thinking": "text-slate-600 italic",
  "cli-nav":      "",
};

function lineClass(cls?: string) {
  const base = "text-slate-400 whitespace-pre break-all";
  if (!cls) return base;
  const extra = cls.split(/\s+/).map((c) => LINE_STYLES[c] ?? "").join(" ");
  return cx(base, extra);
}

export function ConsoleOutput() {
  const [lines, setLines] = useState(getConsoleLines);
  const bottomRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    // Read from window-persisted store on every notify
    subscribeConsole(() => setLines([...getConsoleLines()]));
  }, []);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "instant" });
  }, [lines]);

  return (
    <div className="px-4 py-1.5 font-mono text-[0.78rem] leading-[1.65]" data-cli-mount>
      {lines.map((line) =>
        line.isLink ? (
          <div key={line.id} className={lineClass(line.cls)}>
            <a
              href={line.isLink}
              className="text-sky-400 no-underline hover:underline hover:text-sky-300"
              onClick={(e) => { e.preventDefault(); navigate(line.isLink); }}
            >
              {line.text}
            </a>
          </div>
        ) : (
          <div key={line.id} className={lineClass(line.cls)}>{line.text}</div>
        )
      )}
      <div ref={bottomRef} />
    </div>
  );
}
