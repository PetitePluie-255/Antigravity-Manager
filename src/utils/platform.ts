/**
 * Platform detection utilities for Tauri/Web compatibility
 */

// Check if running in Tauri environment
export const isTauri = (): boolean => {
  return (
    typeof window !== "undefined" &&
    ("__TAURI__" in window || "__TAURI_INTERNALS__" in window)
  );
};

// Get Web API base URL (for non-Tauri environments)
export const getApiBaseUrl = (): string => {
  return import.meta.env.VITE_API_URL || "/api";
};

// Dynamic Tauri invoke import (only loaded when needed)
let tauriInvoke: typeof import("@tauri-apps/api/core").invoke | null = null;

async function getTauriInvoke() {
  if (!tauriInvoke) {
    const core = await import("@tauri-apps/api/core");
    tauriInvoke = core.invoke;
  }
  return tauriInvoke;
}

/**
 * Universal API call function
 * - In Tauri: uses invoke
 * - In Web: uses HTTP fetch
 */
export async function apiCall<T>(
  cmd: string,
  args?: Record<string, unknown>
): Promise<T> {
  if (isTauri()) {
    const invoke = await getTauriInvoke();
    return invoke<T>(cmd, args);
  } else {
    // Web HTTP API
    const response = await fetch(`${getApiBaseUrl()}/${cmd}`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(args || {}),
    });

    if (!response.ok) {
      const errorText = await response.text();
      throw new Error(errorText || `HTTP ${response.status}`);
    }

    return response.json();
  }
}
