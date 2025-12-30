import { request } from "../api/client";
import { Account, QuotaData } from "../types/account";

export async function listAccounts(): Promise<Account[]> {
  return await request("/accounts");
}

export async function getCurrentAccount(): Promise<Account | null> {
  return await request("/accounts/current");
}

export async function addAccount(
  email: string,
  refreshToken: string
): Promise<Account> {
  return await request("/accounts", {
    method: "POST",
    body: JSON.stringify({ email, refresh_token: refreshToken, name: null }),
  });
}

export async function deleteAccount(accountId: string): Promise<void> {
  return await request(`/accounts/${accountId}`, { method: "DELETE" });
}

export async function deleteAccounts(accountIds: string[]): Promise<void> {
  return await request("/accounts/batch-delete", {
    method: "POST",
    body: JSON.stringify({ account_ids: accountIds }),
  });
}

export async function switchAccount(accountId: string): Promise<void> {
  return await request("/accounts/switch", {
    method: "POST",
    body: JSON.stringify({ account_id: accountId }),
  });
}

export async function fetchAccountQuota(accountId: string): Promise<QuotaData> {
  return await request(`/accounts/${accountId}/quota`);
}

export interface RefreshStats {
  total: number;
  success: number;
  failed: number;
  details: string[];
}

export async function refreshAllQuotas(): Promise<RefreshStats> {
  const response = await request<{
    success_count?: number;
    error_count?: number;
  }>("/accounts/quota/refresh", { method: "POST" });

  const success = response.success_count || 0;
  const failed = response.error_count || 0;

  return {
    total: success + failed,
    success,
    failed,
    details: [], // Backend simple response doesn't give details yet without API change
  };
}

export interface ImportResult {
  total: number;
  success: number;
  failed: number;
  errors: string[];
}

export async function importJsonAccounts(
  jsonContent: any
): Promise<ImportResult> {
  return await request("/import/json", {
    method: "POST",
    body: JSON.stringify(jsonContent),
  });
}
