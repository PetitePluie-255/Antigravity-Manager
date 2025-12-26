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
 * Tauri 命令到 Web API 端点的映射
 */
interface EndpointMapping {
  path: string;
  method: "GET" | "POST" | "PUT" | "DELETE";
  /** 用于路径参数替换，如 account_id -> :id */
  pathParams?: string[];
}

const COMMAND_MAPPINGS: Record<string, EndpointMapping> = {
  // 账户管理
  list_accounts: { path: "/accounts", method: "GET" },
  add_account: { path: "/accounts", method: "POST" },
  delete_account: {
    path: "/accounts/:account_id",
    method: "DELETE",
    pathParams: ["account_id"],
  },
  delete_accounts: { path: "/accounts/batch", method: "DELETE" },
  switch_account: { path: "/accounts/current", method: "PUT" },
  get_current_account: { path: "/accounts/current", method: "GET" },
  fetch_account_quota: {
    path: "/accounts/:account_id/quota",
    method: "POST",
    pathParams: ["account_id"],
  },
  refresh_all_quotas: { path: "/accounts/quota/refresh", method: "POST" },

  // 配置管理
  load_config: { path: "/config", method: "GET" },
  save_config: { path: "/config", method: "PUT" },

  // 代理服务控制
  start_proxy_service: { path: "/proxy/start", method: "POST" },
  stop_proxy_service: { path: "/proxy/stop", method: "POST" },
  get_proxy_status: { path: "/proxy/status", method: "GET" },
  generate_api_key: { path: "/proxy/key/generate", method: "POST" },
  update_model_mapping: { path: "/proxy/mapping", method: "PUT" },

  // OAuth 登录 (Web 模式支持)
  start_oauth_login: { path: "/oauth/start", method: "POST" },
  get_oauth_status: { path: "/oauth/status", method: "GET" },

  // 其他命令 (Web 模式下暂不支持)
  cancel_oauth_login: { path: "/oauth/cancel", method: "POST" },
  // 导入 API
  import_accounts_json: { path: "/import/json", method: "POST" },
  import_accounts_file: { path: "/import/file", method: "POST" },
  import_from_database: { path: "/import/database", method: "POST" },
  // 日志 API
  get_proxy_logs: { path: "/proxy/logs", method: "GET" },
  clear_proxy_logs: { path: "/proxy/logs/clear", method: "POST" },
  // 其他
  sync_account_from_db: { path: "/sync/db", method: "POST" },
  save_text_file: { path: "/files/save", method: "POST" },
  clear_log_cache: { path: "/logs/clear", method: "POST" },
  open_data_folder: { path: "/system/open-data-folder", method: "POST" },
  get_data_dir_path: { path: "/system/data-dir", method: "GET" },
  show_main_window: { path: "/window/show", method: "POST" },
  get_antigravity_path: { path: "/system/antigravity-path", method: "GET" },
  check_for_updates: { path: "/system/check-updates", method: "GET" },
  reload_proxy_accounts: { path: "/proxy/accounts/reload", method: "POST" },
  get_proxy_stats: { path: "/proxy/stats", method: "GET" },
};

/**
 * 构建 Web API URL
 */
function buildApiUrl(
  cmd: string,
  args?: Record<string, unknown>
): { url: string; body?: Record<string, unknown> } {
  const mapping = COMMAND_MAPPINGS[cmd];
  if (!mapping) {
    // 未映射的命令，使用默认路径
    return { url: `${getApiBaseUrl()}/${cmd}`, body: args };
  }

  let path = mapping.path;
  let body = args ? { ...args } : undefined;

  // 替换路径参数
  if (mapping.pathParams && body) {
    for (const param of mapping.pathParams) {
      if (param in body) {
        path = path.replace(`:${param}`, String(body[param]));
        delete body[param];
      }
    }
    // 如果 body 为空对象，设为 undefined
    if (body && Object.keys(body).length === 0) {
      body = undefined;
    }
  }

  return { url: `${getApiBaseUrl()}${path}`, body };
}

/**
 * Universal API call function
 * - In Tauri: uses invoke
 * - In Web: uses HTTP fetch with proper endpoint mapping
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
    const mapping = COMMAND_MAPPINGS[cmd];
    const method = mapping?.method || "POST";
    const { url, body } = buildApiUrl(cmd, args);

    const options: RequestInit = {
      method,
      headers: { "Content-Type": "application/json" },
    };

    // GET 和 DELETE 请求通常不带 body
    if (body && (method === "POST" || method === "PUT")) {
      options.body = JSON.stringify(body);
    }

    const response = await fetch(url, options);

    if (!response.ok) {
      const errorText = await response.text();
      throw new Error(errorText || `HTTP ${response.status}`);
    }

    const result = await response.json();

    // Web API 返回 { success, data, error } 格式，需要解包
    if (result && typeof result === "object" && "success" in result) {
      if (result.success) {
        return result.data as T;
      } else {
        throw new Error(result.error || "Unknown error");
      }
    }

    return result as T;
  }
}
