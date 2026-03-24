/** Imperative hooks for non-React code (e.g. pipeline editor, behavior files) to sync with studio chrome — no window.* events. */

export type StudioPanelId = "git-repo" | "session";

type StudioChromeApi = {
  touchRepo: () => void;
  setActivePanel: (id: StudioPanelId | null) => void;
  openConsole: () => void;
  closeConsole: () => void;
};

let registered: StudioChromeApi | null = null;

export function registerStudioChrome(api: StudioChromeApi | null) {
  registered = api;
}

/** Bump repo refresh generation so git status refetches. */
export function notifyStudioRepoChanged() {
  registered?.touchRepo();
}

/** Mutex: opening one header panel closes others. */
export function notifyStudioPanelOpened(panelId: StudioPanelId) {
  registered?.setActivePanel(panelId);
}

/** Open the bottom console panel (calls into React state). */
export function notifyConsoleOpen() {
  registered?.openConsole();
}

/** Close the bottom console panel (calls into React state). */
export function notifyConsoleClose() {
  registered?.closeConsole();
}
