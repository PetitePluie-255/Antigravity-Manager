import { apiCall, isTauri } from "../utils/platform";
import { Account, QuotaData } from "../types/account";

export async function listAccounts(): Promise<Account[]> {
  return await apiCall("list_accounts");
}

export async function getCurrentAccount(): Promise<Account | null> {
  return await apiCall("get_current_account");
}

export async function addAccount(
  email: string,
  refreshToken: string
): Promise<Account> {
  return await apiCall("add_account", { email, refresh_token: refreshToken });
}

export async function deleteAccount(accountId: string): Promise<void> {
  return await apiCall("delete_account", { account_id: accountId });
}

export async function deleteAccounts(accountIds: string[]): Promise<void> {
  return await apiCall("delete_accounts", { account_ids: accountIds });
}

export async function switchAccount(accountId: string): Promise<void> {
  return await apiCall("switch_account", { account_id: accountId });
}

export async function fetchAccountQuota(accountId: string): Promise<QuotaData> {
  return await apiCall("fetch_account_quota", { account_id: accountId });
}

export interface RefreshStats {
  total: number;
  success: number;
  failed: number;
  details: string[];
}

export async function refreshAllQuotas(): Promise<RefreshStats> {
  const response = await apiCall<{
    success_count?: number;
    error_count?: number;
    // Tauri 模式返回的字段
    total?: number;
    success?: number;
    failed?: number;
    details?: string[];
  }>("refresh_all_quotas");

  // 兼容 Web API 和 Tauri API 的不同响应格式
  return {
    total:
      response.total ??
      (response.success_count ?? 0) + (response.error_count ?? 0),
    success: response.success ?? response.success_count ?? 0,
    failed: response.failed ?? response.error_count ?? 0,
    details: response.details ?? [],
  };
}

// OAuth 登录状态（用于 Web 端取消）
let oauthPollInterval: ReturnType<typeof setInterval> | null = null;
let oauthAuthWindow: Window | null = null;

// OAuth 登录
export async function startOAuthLogin(): Promise<Account> {
  if (isTauri()) {
    // Tauri 环境：使用原有逻辑
    return await apiCall("start_oauth_login");
  }

  // Web 环境：打开新窗口 + 轮询状态
  interface StartOAuthResponse {
    auth_url: string;
    redirect_uri: string;
  }

  interface OAuthStatusResponse {
    status: "pending" | "success" | "error";
    account?: Account;
    error?: string;
  }

  // 1. 获取授权 URL
  const { auth_url } = await apiCall<StartOAuthResponse>("start_oauth_login");

  // 2. 打开新窗口进行授权
  oauthAuthWindow = window.open(auth_url, "_blank", "width=600,height=700");

  // 3. 轮询状态
  return new Promise((resolve, reject) => {
    let attempts = 0;
    const maxAttempts = 180; // 6分钟超时 (2秒 * 180)

    oauthPollInterval = setInterval(async () => {
      attempts++;

      // 检查窗口是否被关闭且轮询超过一定次数
      if (oauthAuthWindow && oauthAuthWindow.closed && attempts > 5) {
        // 用户手动关闭了窗口，检查最后一次状态
        try {
          const status = await apiCall<OAuthStatusResponse>("get_oauth_status");
          if (status.status === "success" && status.account) {
            if (oauthPollInterval) clearInterval(oauthPollInterval);
            oauthPollInterval = null;
            oauthAuthWindow = null;
            resolve(status.account);
            return;
          }
        } catch (e) {
          // 忽略
        }
        // 窗口关闭且没有成功，认为用户取消
        if (oauthPollInterval) clearInterval(oauthPollInterval);
        oauthPollInterval = null;
        oauthAuthWindow = null;
        reject(new Error("用户取消了授权"));
        return;
      }

      try {
        const status = await apiCall<OAuthStatusResponse>("get_oauth_status");

        if (status.status === "success" && status.account) {
          if (oauthPollInterval) clearInterval(oauthPollInterval);
          oauthPollInterval = null;
          if (oauthAuthWindow && !oauthAuthWindow.closed) {
            oauthAuthWindow.close();
          }
          oauthAuthWindow = null;
          resolve(status.account);
        } else if (status.status === "error") {
          if (oauthPollInterval) clearInterval(oauthPollInterval);
          oauthPollInterval = null;
          if (oauthAuthWindow && !oauthAuthWindow.closed) {
            oauthAuthWindow.close();
          }
          oauthAuthWindow = null;
          reject(new Error(status.error || "OAuth 授权失败"));
        } else if (attempts >= maxAttempts) {
          if (oauthPollInterval) clearInterval(oauthPollInterval);
          oauthPollInterval = null;
          if (oauthAuthWindow && !oauthAuthWindow.closed) {
            oauthAuthWindow.close();
          }
          oauthAuthWindow = null;
          reject(new Error("OAuth 授权超时"));
        }
      } catch (e) {
        // 网络错误，继续轮询
        console.warn("OAuth status poll failed:", e);
      }
    }, 2000);
  });
}

export async function completeOAuthLogin(): Promise<Account> {
  ensureTauriEnvironment();
  try {
    return await invoke("complete_oauth_login");
  } catch (error) {
    if (typeof error === "string") {
      if (error.includes("Refresh Token") || error.includes("refresh_token")) {
        throw error;
      }
      throw `OAuth 授权失败: ${error}`;
    }
    throw error;
  }
}

export async function cancelOAuthLogin(): Promise<void> {
  if (isTauri()) {
    return await apiCall("cancel_oauth_login");
  }

  // Web 环境：清理轮询和窗口
  if (oauthPollInterval) {
    clearInterval(oauthPollInterval);
    oauthPollInterval = null;
  }
  if (oauthAuthWindow && !oauthAuthWindow.closed) {
    oauthAuthWindow.close();
  }
  oauthAuthWindow = null;
}

// 导入 - 仅 Tauri 环境可用
export async function importV1Accounts(): Promise<Account[]> {
  if (!isTauri()) {
    throw new Error("V1 备份导入仅在桌面应用中可用。");
  }
  return await apiCall("import_v1_accounts");
}

export async function importFromDb(): Promise<Account> {
  if (!isTauri()) {
    throw new Error("从 IDE 数据库导入仅在桌面应用中可用。");
  }
  return await apiCall("import_from_db");
}

export async function importFromCustomDb(path: string): Promise<Account> {
  return await invoke("import_custom_db", { path });
}

export async function syncAccountFromDb(): Promise<Account | null> {
  if (!isTauri()) {
    throw new Error("从 IDE 数据库同步仅在桌面应用中可用。");
  }
  return await apiCall("sync_account_from_db");
}
