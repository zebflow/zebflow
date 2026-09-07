import { defineConfig, devices } from "@playwright/test";
import { BASE_URL, STORAGE_STATE } from "./config";

/**
 * The suite talks to a running Zebflow dev server. It never starts one on a
 * port that is already busy, so a server you launched by hand keeps its logs
 * and its data root.
 */
export default defineConfig({
  testDir: "./",
  // Zebflow renders pages server-side and hydrates after; give slow first
  // compiles room without making a genuine hang look like a pass.
  timeout: 60_000,
  expect: { timeout: 15_000 },
  // These tests create and delete real projects on a shared server, so they
  // must not race each other.
  fullyParallel: false,
  workers: 1,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 1 : 0,
  reporter: process.env.CI ? [["list"], ["html", { open: "never" }]] : [["list"]],
  use: {
    baseURL: BASE_URL,
    trace: "retain-on-failure",
    screenshot: "only-on-failure",
    video: "off",
  },
  projects: [
    { name: "setup", testMatch: /auth\.setup\.ts$/ },
    {
      name: "studio",
      testMatch: /specs\/.*\.spec\.ts$/,
      dependencies: ["setup"],
      use: { ...devices["Desktop Chrome"], storageState: STORAGE_STATE },
    },
  ],
  webServer: {
    // `dev.sh` kills whatever holds the port before it builds, so it is only
    // safe to run when nothing is listening. reuseExistingServer guarantees
    // that: an already-running dev server is adopted, never restarted.
    command: "./dev.sh",
    cwd: "../..",
    url: `${BASE_URL}/health`,
    reuseExistingServer: true,
    timeout: 240_000,
    stdout: "ignore",
    stderr: "pipe",
  },
});
