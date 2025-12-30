import { request } from "../api/client";
import { AppConfig } from "../types/config";

export async function loadConfig(): Promise<AppConfig> {
  return await request<AppConfig>("/config");
}

/*
 * Save config to server
 */
export async function saveConfig(config: AppConfig): Promise<void> {
  return await request<void>("/config", {
    method: "PUT",
    body: JSON.stringify(config),
  });
}
