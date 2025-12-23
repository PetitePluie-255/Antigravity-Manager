import { apiCall } from "../utils/platform";
import { AppConfig } from "../types/config";

export async function loadConfig(): Promise<AppConfig> {
  return await apiCall("load_config");
}

export async function saveConfig(config: AppConfig): Promise<void> {
  return await apiCall("save_config", { config });
}
