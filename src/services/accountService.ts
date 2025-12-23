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
  return await apiCall("add_account", { email, refreshToken });
}

export async function deleteAccount(accountId: string): Promise<void> {
  return await apiCall("delete_account", { accountId });
}

export async function switchAccount(accountId: string): Promise<void> {
  return await apiCall("switch_account", { accountId });
}

export async function fetchAccountQuota(accountId: string): Promise<QuotaData> {
  return await apiCall("fetch_account_quota", { accountId });
}

export interface RefreshStats {
  total: number;
  success: number;
  failed: number;
  details: string[];
}

export async function refreshAllQuotas(): Promise<RefreshStats> {
  return await apiCall("refresh_all_quotas");
}

// OAuth - 仅 Tauri 环境可用
export async function startOAuthLogin(): Promise<Account> {
  if (!isTauri()) {
    throw new Error(
      "OAuth 登录仅在桌面应用中可用。请使用 Refresh Token 方式添加账户。"
    );
  }
  return await apiCall("start_oauth_login");
}

export async function cancelOAuthLogin(): Promise<void> {
  if (!isTauri()) {
    return; // Web 环境无需取消
  }
  return await apiCall("cancel_oauth_login");
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
