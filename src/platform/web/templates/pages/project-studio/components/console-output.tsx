import { useState, useEffect, useRef, cx } from "zeb";
import { subscribeConsole, consoleLines, navigate } from "@/pages/project-studio/components/studio-shell-behavior";

export function ConsoleOutput() {
  const [lines, setLines] = useState(consoleLines);
  const bottomRef = useRef(null);

  useEffect(() => {
    subscribeConsole(() => setLines([...consoleLines]));
  }, []);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "instant" });
  }, [lines]);

  return (
    <div className="cli-output-list" data-cli-mount>
      {lines.map((line) =>
        line.isLink ? (
          <div key={line.id} className={cx("cli-line", line.cls)}>
            <a
              href={line.isLink}
              className="cli-link"
              onClick={(e) => { e.preventDefault(); navigate(line.isLink); }}
            >
              {line.text}
            </a>
          </div>
        ) : (
          <div key={line.id} className={cx("cli-line", line.cls)}>{line.text}</div>
        )
      )}
      <div ref={bottomRef} />
    </div>
  );
}
