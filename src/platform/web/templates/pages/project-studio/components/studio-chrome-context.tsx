import { createContext, useContext, useEffect, useMemo, useState } from "zeb";
import { registerStudioChrome } from "@/pages/project-studio/components/studio-chrome-bridge";

type StudioChromeValue = {
  repoEpoch: number;
  activePanel: string | null;
  setActivePanel: (id: string | null) => void;
  openHeaderPanel: (id: "git-repo" | "session") => void;
  consoleOpen: boolean;
  toggleConsole: () => void;
};

const StudioChromeContext = createContext(null as StudioChromeValue | null);

export function StudioChromeProvider({ children }) {
  const [repoEpoch, setRepoEpoch] = useState(0);
  const [activePanel, setActivePanel] = useState(null as string | null);
  const [consoleOpen, setConsoleOpen] = useState(false);

  useEffect(() => {
    registerStudioChrome({
      touchRepo: () => setRepoEpoch((n) => n + 1),
      setActivePanel,
      openConsole: () => setConsoleOpen(true),
      closeConsole: () => setConsoleOpen(false),
    });
    return () => registerStudioChrome(null);
  }, []);

  const value = useMemo((): StudioChromeValue => {
    return {
      repoEpoch,
      activePanel,
      setActivePanel,
      openHeaderPanel: (id) => setActivePanel(id),
      consoleOpen,
      toggleConsole: () => setConsoleOpen((v) => !v),
    };
  }, [repoEpoch, activePanel, consoleOpen]);

  return <StudioChromeContext.Provider value={value}>{children}</StudioChromeContext.Provider>;
}

export function useStudioChrome(): StudioChromeValue {
  const ctx = useContext(StudioChromeContext);
  if (!ctx) {
    throw new Error("useStudioChrome must be used inside StudioChromeProvider");
  }
  return ctx;
}
