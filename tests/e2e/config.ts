/**
 * Plain values only. Importing from a file that declares a test would register
 * that test into every spec that imports it, so this module must stay inert.
 */
export const BASE_URL = process.env.ZEBFLOW_BASE_URL ?? "http://localhost:10610";
export const OWNER = process.env.ZEBFLOW_OWNER ?? "superadmin";
export const PASSWORD = process.env.ZEBFLOW_PASSWORD ?? "admin123";
export const PROJECT = process.env.ZEBFLOW_PROJECT ?? "default";
export const STORAGE_STATE = "./.auth/state.json";
