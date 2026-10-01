import { invoke } from "@tauri-apps/api/core";

// Fallback provided at compile-time by Vite from package.json
declare const __APP_VERSION__: string;
export const APP_VERSION_FALLBACK = typeof __APP_VERSION__ !== "undefined" ? __APP_VERSION__ : "0.0.0";

/**
 * Dynamically queries the application version from the Tauri backend (Cargo.toml).
 * Falls back to the build-time package.json version if IPC fails or webview runs standalone.
 */
export async function getAppVersion(): Promise<string> {
  try {
    const ver = await invoke<string>("get_app_version");
    if (ver && ver.trim().length > 0) {
      return ver.trim();
    }
  } catch (err) {
    console.debug("Could not fetch app version from Tauri backend, using fallback:", err);
  }
  return APP_VERSION_FALLBACK;
}
