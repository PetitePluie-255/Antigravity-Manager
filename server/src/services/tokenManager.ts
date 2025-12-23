import { db } from "../db/sqlite.js";
import { refreshAccessToken } from "./google.js";
import { HttpsProxyAgent } from "https-proxy-agent";

// Helper to get proxy agent (duplicated to avoid circular deps if extracting to utils)
// Ideally should be in a shared utils file
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

export interface ProxyToken {
  accountId: string;
  accessToken: string;
  refreshToken: string;
  expiresIn: number;
  expiryTimestamp: number;
  email: string;
  projectId?: string;
  sessionId: string;
}

// Token storage (in-memory cache)
const tokens: Map<string, ProxyToken> = new Map();
let currentIndex = 0;

/**
 * Generate a session ID (matching Rust implementation)
 */
function generateSessionId(): string {
  const min = 1_000_000_000_000_000_000n;
  const max = 9_000_000_000_000_000_000n;
  const range = max - min;
  const randomBigInt = min + BigInt(Math.floor(Math.random() * Number(range)));
  return (-randomBigInt).toString();
}

/**
 * Load accounts from SQLite database
 */
export async function loadAccounts(): Promise<number> {
  tokens.clear();

  const rows = db.prepare("SELECT * FROM accounts").all() as any[];

  for (const row of rows) {
    if (!row.refresh_token) continue;

    const token: ProxyToken = {
      accountId: row.id,
      accessToken: row.access_token || "",
      refreshToken: row.refresh_token,
      expiresIn: row.expires_in || 3600,
      expiryTimestamp: row.expiry_timestamp || 0,
      email: row.email,
      projectId: undefined, // Will be fetched on demand
      sessionId: generateSessionId(),
    };

    tokens.set(row.id, token);
  }

  console.log(`[TokenManager] Loaded ${tokens.size} accounts`);
  return tokens.size;
}

/**
 * Fetch project ID from Google API
 */
async function fetchProjectId(accessToken: string): Promise<string | null> {
  const LOAD_PROJECT_API_URL =
    "https://cloudcode-pa.googleapis.com/v1internal:loadCodeAssist";
  const USER_AGENT = "antigravity/1.11.3 Darwin/arm64";

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
      const data = (await response.json()) as any;
      return data.cloudaicompanionProject || null;
    }
  } catch (error) {
    console.error("[TokenManager] Failed to fetch project ID:", error);
  }
  return null;
}

/**
 * Generate a mock project ID (fallback)
 */
function generateMockProjectId(): string {
  const chars = "abcdefghijklmnopqrstuvwxyz";
  let suffix = "";
  for (let i = 0; i < 5; i++) {
    suffix += chars.charAt(Math.floor(Math.random() * chars.length));
  }
  return `mock-project-${suffix}`;
}

/**
 * Get next available token (round-robin)
 * Automatically refreshes expired tokens and fetches project IDs
 */
export async function getToken(): Promise<ProxyToken | null> {
  if (tokens.size === 0) {
    return null;
  }

  const tokenArray = Array.from(tokens.values());
  const idx = currentIndex % tokenArray.length;
  currentIndex++;

  let token = tokenArray[idx];
  const now = Date.now();

  // Check if token is expired (refresh 5 minutes early)
  if (now >= token.expiryTimestamp - 300000) {
    console.log(
      `[TokenManager] Token for ${token.email} is expiring, refreshing...`
    );

    try {
      const tokenRes = await refreshAccessToken(token.refreshToken);
      token.accessToken = tokenRes.access_token;
      token.expiresIn = tokenRes.expires_in;
      token.expiryTimestamp = now + tokenRes.expires_in * 1000;

      // Update in database
      db.prepare(
        "UPDATE accounts SET access_token = ?, expires_in = ?, expiry_timestamp = ? WHERE id = ?"
      ).run(
        token.accessToken,
        token.expiresIn,
        token.expiryTimestamp,
        token.accountId
      );

      // Update in memory
      tokens.set(token.accountId, token);

      console.log(`[TokenManager] Token refreshed successfully`);
    } catch (error) {
      console.error(`[TokenManager] Failed to refresh token:`, error);
      // Continue with possibly expired token
    }
  }

  // Fetch project ID if missing
  if (!token.projectId) {
    console.log(`[TokenManager] Fetching project ID for ${token.email}...`);

    const projectId = await fetchProjectId(token.accessToken);
    if (projectId) {
      token.projectId = projectId;
      console.log(`[TokenManager] Got project ID: ${projectId}`);
    } else {
      token.projectId = generateMockProjectId();
      console.log(`[TokenManager] Using mock project ID: ${token.projectId}`);
    }

    // Update in memory
    tokens.set(token.accountId, token);
  }

  return token;
}

/**
 * Get count of loaded tokens
 */
export function getTokenCount(): number {
  return tokens.size;
}

/**
 * Reload accounts from database
 */
export async function reloadAccounts(): Promise<number> {
  return loadAccounts();
}
