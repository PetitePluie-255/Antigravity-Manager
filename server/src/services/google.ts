// Google OAuth 配置 (与 Rust 后端相同)
import { HttpsProxyAgent } from "https-proxy-agent";
import { db } from "../db/sqlite.js";

// Helper to get proxy agent
function getProxyAgent(): HttpsProxyAgent<string> | undefined {
  try {
    const row = db
      .prepare("SELECT value FROM config WHERE key = 'app_config'")
      .get() as { value: string } | undefined;

    if (row) {
      const config = JSON.parse(row.value);
      if (
        config.proxy?.upstream_proxy?.enabled &&
        config.proxy?.upstream_proxy?.url
      ) {
        return new HttpsProxyAgent(config.proxy.upstream_proxy.url);
      }
    }
  } catch (e) {
    // Ignore
  }
  return undefined;
}
const CLIENT_ID =
  "1071006060591-tmhssin2h21lcre235vtolojh4g403ep.apps.googleusercontent.com";
const CLIENT_SECRET = "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";
const TOKEN_URL = "https://oauth2.googleapis.com/token";
const USERINFO_URL = "https://www.googleapis.com/oauth2/v2/userinfo";
const QUOTA_API_URL =
  "https://cloudcode-pa.googleapis.com/v1internal:fetchAvailableModels";
const LOAD_PROJECT_API_URL =
  "https://cloudcode-pa.googleapis.com/v1internal:loadCodeAssist";
const USER_AGENT = "antigravity/1.11.3 Darwin/arm64";

interface TokenResponse {
  access_token: string;
  expires_in: number;
  token_type?: string;
  refresh_token?: string;
}

interface UserInfo {
  email: string;
  name?: string;
  given_name?: string;
  family_name?: string;
  picture?: string;
}

interface QuotaInfo {
  remainingFraction?: number;
  resetTime?: string;
}

interface ModelInfo {
  quotaInfo?: QuotaInfo;
}

interface QuotaResponse {
  models: Record<string, ModelInfo>;
}

interface LoadProjectResponse {
  cloudaicompanionProject?: string;
}

export interface QuotaData {
  models: Array<{ name: string; percentage: number; reset_time: string }>;
  last_updated: number;
  is_forbidden?: boolean;
}

/**
 * 使用 refresh_token 刷新 access_token
 */
export async function refreshAccessToken(
  refreshToken: string
): Promise<TokenResponse> {
  const params = new URLSearchParams({
    client_id: CLIENT_ID,
    client_secret: CLIENT_SECRET,
    refresh_token: refreshToken,
    grant_type: "refresh_token",
  });

  console.log("[Google] 正在刷新 Token...");

  const response = await fetch(TOKEN_URL, {
    method: "POST",
    headers: { "Content-Type": "application/x-www-form-urlencoded" },
    body: params.toString(),
    agent: getProxyAgent(),
  } as any);

  if (!response.ok) {
    const errorText = await response.text();
    throw new Error(`刷新失败: ${errorText}`);
  }

  const tokenData = (await response.json()) as TokenResponse;
  console.log(`[Google] Token 刷新成功！有效期: ${tokenData.expires_in} 秒`);

  return tokenData;
}

/**
 * 获取用户信息
 */
export async function getUserInfo(accessToken: string): Promise<UserInfo> {
  const response = await fetch(USERINFO_URL, {
    headers: { Authorization: `Bearer ${accessToken}` },
    agent: getProxyAgent(),
  } as any);

  if (!response.ok) {
    const errorText = await response.text();
    throw new Error(`获取用户信息失败: ${errorText}`);
  }

  return response.json() as Promise<UserInfo>;
}

/**
 * 获取 Project ID (用于 quota 查询)
 */
async function fetchProjectId(accessToken: string): Promise<string | null> {
  try {
    const response = await fetch(LOAD_PROJECT_API_URL, {
      method: "POST",
      headers: {
        Authorization: `Bearer ${accessToken}`,
        "User-Agent": USER_AGENT,
        "Content-Type": "application/json",
      },
      body: JSON.stringify({
        metadata: { ideType: "ANTIGRAVITY" },
      }),
      agent: getProxyAgent(),
    } as any);

    if (response.ok) {
      const data = (await response.json()) as LoadProjectResponse;
      return data.cloudaicompanionProject || null;
    }
  } catch (error) {
    console.error("[Google] 获取 Project ID 失败:", error);
  }
  return null;
}

/**
 * 查询账号配额
 */
export async function fetchQuota(accessToken: string): Promise<QuotaData> {
  console.log("[Google] 开始查询配额...");

  // 1. 获取 Project ID
  const projectId = await fetchProjectId(accessToken);
  console.log(`[Google] Project ID: ${projectId || "none"}`);

  // 2. 构建请求体
  const payload: Record<string, string> = {};
  if (projectId) {
    payload.project = projectId;
  }

  // 3. 查询配额 (带重试)
  const maxRetries = 3;
  let lastError: Error | null = null;

  for (let attempt = 1; attempt <= maxRetries; attempt++) {
    try {
      const response = await fetch(QUOTA_API_URL, {
        method: "POST",
        headers: {
          Authorization: `Bearer ${accessToken}`,
          "User-Agent": USER_AGENT,
          "Content-Type": "application/json",
        },
        body: JSON.stringify(payload),
        agent: getProxyAgent(),
      } as any);

      // 特殊处理 403 Forbidden
      if (response.status === 403) {
        console.warn("[Google] 账号无权限 (403 Forbidden)");
        return {
          models: [],
          last_updated: Date.now(),
          is_forbidden: true,
        };
      }

      if (!response.ok) {
        const text = await response.text();
        throw new Error(`HTTP ${response.status} - ${text}`);
      }

      const quotaResponse = (await response.json()) as QuotaResponse;

      // 解析配额数据
      const quotaData: QuotaData = {
        models: [],
        last_updated: Date.now(),
      };

      console.log(
        `[Google] Quota API 返回了 ${
          Object.keys(quotaResponse.models || {}).length
        } 个模型`
      );

      for (const [name, info] of Object.entries(quotaResponse.models || {})) {
        if (info.quotaInfo) {
          const percentage = info.quotaInfo.remainingFraction
            ? Math.round(info.quotaInfo.remainingFraction * 100)
            : 0;
          const resetTime = info.quotaInfo.resetTime || "";

          // 只保存 gemini 和 claude 模型
          if (name.includes("gemini") || name.includes("claude")) {
            quotaData.models.push({ name, percentage, reset_time: resetTime });
            console.log(`   - ${name}: ${percentage}%`);
          }
        }
      }

      return quotaData;
    } catch (error) {
      console.warn(`[Google] 请求失败 (尝试 ${attempt}/${maxRetries}):`, error);
      lastError = error as Error;
      if (attempt < maxRetries) {
        await new Promise((r) => setTimeout(r, 1000));
      }
    }
  }

  throw lastError || new Error("配额查询失败");
}
