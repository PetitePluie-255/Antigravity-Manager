import { Router } from "express";
import { db } from "../db/sqlite.js";
import crypto from "crypto";
import {
  refreshAccessToken,
  getUserInfo,
  fetchQuota,
} from "../services/google.js";

export const accountRoutes: Router = Router();

// Types
interface Account {
  id: string;
  email: string;
  name?: string;
  token: {
    access_token: string;
    refresh_token: string;
    expires_in: number;
    expiry_timestamp: number;
    token_type: string;
    email?: string;
  };
  quota?: {
    models: Array<{ name: string; percentage: number; reset_time: string }>;
    last_updated: number;
    is_forbidden?: boolean;
  };
  created_at: number;
  last_used: number;
}

function rowToAccount(row: any): Account {
  return {
    id: row.id,
    email: row.email,
    name: row.name,
    token: {
      access_token: row.access_token || "",
      refresh_token: row.refresh_token,
      expires_in: row.expires_in || 3600,
      expiry_timestamp: row.expiry_timestamp || 0,
      token_type: row.token_type || "Bearer",
      email: row.email,
    },
    quota: row.quota_data ? JSON.parse(row.quota_data) : undefined,
    created_at: row.created_at,
    last_used: row.last_used,
  };
}

// list_accounts
accountRoutes.post("/list_accounts", (req, res) => {
  try {
    const rows = db
      .prepare("SELECT * FROM accounts ORDER BY last_used DESC")
      .all();
    const accounts = rows.map(rowToAccount);
    res.json(accounts);
  } catch (error) {
    console.error("list_accounts error:", error);
    res.status(500).json({ error: String(error) });
  }
});

// get_current_account
accountRoutes.post("/get_current_account", (req, res) => {
  try {
    const current = db
      .prepare("SELECT account_id FROM current_account WHERE id = 1")
      .get() as { account_id: string } | undefined;
    if (!current?.account_id) {
      return res.json(null);
    }
    const row = db
      .prepare("SELECT * FROM accounts WHERE id = ?")
      .get(current.account_id);
    if (!row) {
      return res.json(null);
    }
    res.json(rowToAccount(row));
  } catch (error) {
    console.error("get_current_account error:", error);
    res.status(500).json({ error: String(error) });
  }
});

// add_account - 使用 Google API 验证 token
accountRoutes.post("/add_account", async (req, res) => {
  try {
    const { email: inputEmail, refreshToken } = req.body;

    if (!refreshToken) {
      return res.status(400).json({ error: "缺少 refreshToken" });
    }

    // Check for duplicate
    const existing = db
      .prepare("SELECT id FROM accounts WHERE refresh_token = ?")
      .get(refreshToken);
    if (existing) {
      return res.status(400).json({ error: "账户已存在" });
    }

    // 1. 使用 refresh_token 获取 access_token
    console.log("[add_account] 验证 refresh_token...");
    const tokenRes = await refreshAccessToken(refreshToken);

    // 2. 获取用户信息
    console.log("[add_account] 获取用户信息...");
    const userInfo = await getUserInfo(tokenRes.access_token);

    const id = crypto.randomUUID();
    const nowMs = Date.now();
    const nowSec = Math.floor(nowMs / 1000); // 使用秒级时间戳
    const expiryTimestamp = nowMs + tokenRes.expires_in * 1000; // token 过期仍用毫秒
    const email =
      userInfo.email ||
      inputEmail ||
      `user_${id.slice(0, 8)}@antigravity.local`;
    const name = userInfo.name || userInfo.given_name || null;

    console.log(`[add_account] 添加账户: ${email} (${name || "no name"})`);

    db.prepare(
      `
            INSERT INTO accounts (id, email, name, refresh_token, access_token, expires_in, expiry_timestamp, created_at, last_used)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        `
    ).run(
      id,
      email,
      name,
      refreshToken,
      tokenRes.access_token,
      tokenRes.expires_in,
      expiryTimestamp,
      nowSec,
      nowSec
    );

    const row = db.prepare("SELECT * FROM accounts WHERE id = ?").get(id);
    res.json(rowToAccount(row));
  } catch (error) {
    console.error("add_account error:", error);
    res.status(500).json({ error: String(error) });
  }
});

// delete_account
accountRoutes.post("/delete_account", (req, res) => {
  try {
    const { accountId } = req.body;
    db.prepare("DELETE FROM accounts WHERE id = ?").run(accountId);

    // Also remove from current if this was current
    const current = db
      .prepare("SELECT account_id FROM current_account WHERE id = 1")
      .get() as { account_id: string } | undefined;
    if (current?.account_id === accountId) {
      db.prepare("DELETE FROM current_account WHERE id = 1").run();
    }

    res.json({ success: true });
  } catch (error) {
    console.error("delete_account error:", error);
    res.status(500).json({ error: String(error) });
  }
});

// switch_account
accountRoutes.post("/switch_account", (req, res) => {
  try {
    const { accountId } = req.body;

    // Verify account exists
    const account = db
      .prepare("SELECT id FROM accounts WHERE id = ?")
      .get(accountId);
    if (!account) {
      return res.status(404).json({ error: "账户不存在" });
    }

    // Upsert current account
    db.prepare(
      `
            INSERT INTO current_account (id, account_id) VALUES (1, ?)
            ON CONFLICT(id) DO UPDATE SET account_id = excluded.account_id
        `
    ).run(accountId);

    // Update last_used (使用秒级时间戳)
    db.prepare("UPDATE accounts SET last_used = ? WHERE id = ?").run(
      Math.floor(Date.now() / 1000),
      accountId
    );

    res.json({ success: true });
  } catch (error) {
    console.error("switch_account error:", error);
    res.status(500).json({ error: String(error) });
  }
});

// fetch_account_quota - 使用真实 Google API
accountRoutes.post("/fetch_account_quota", async (req, res) => {
  try {
    const { accountId } = req.body;

    // 获取账户信息
    const row = db
      .prepare("SELECT * FROM accounts WHERE id = ?")
      .get(accountId) as any;
    if (!row) {
      return res.status(404).json({ error: "账户不存在" });
    }

    // 检查 token 是否过期，如需要则刷新
    let accessToken = row.access_token;
    const now = Date.now();

    if (!accessToken || row.expiry_timestamp < now + 300000) {
      // 5分钟缓冲
      console.log(`[fetch_quota] Token 已过期，刷新中...`);
      try {
        const tokenRes = await refreshAccessToken(row.refresh_token);
        accessToken = tokenRes.access_token;
        const expiryTimestamp = now + tokenRes.expires_in * 1000;

        // 更新数据库
        db.prepare(
          "UPDATE accounts SET access_token = ?, expires_in = ?, expiry_timestamp = ? WHERE id = ?"
        ).run(accessToken, tokenRes.expires_in, expiryTimestamp, accountId);
      } catch (error) {
        console.error("[fetch_quota] Token 刷新失败:", error);
        return res.status(401).json({ error: "Token 刷新失败: " + error });
      }
    }

    // 获取配额
    const quotaData = await fetchQuota(accessToken);

    // 保存到数据库
    db.prepare("UPDATE accounts SET quota_data = ? WHERE id = ?").run(
      JSON.stringify(quotaData),
      accountId
    );

    res.json(quotaData);
  } catch (error) {
    console.error("fetch_account_quota error:", error);
    res.status(500).json({ error: String(error) });
  }
});

// refresh_all_quotas
accountRoutes.post("/refresh_all_quotas", async (req, res) => {
  try {
    const rows = db.prepare("SELECT id FROM accounts").all() as {
      id: string;
    }[];

    let success = 0;
    let failed = 0;
    const details: string[] = [];

    for (const row of rows) {
      try {
        // 这里简化处理，直接调用 fetch_account_quota 的逻辑
        const account = db
          .prepare("SELECT * FROM accounts WHERE id = ?")
          .get(row.id) as any;
        if (!account) continue;

        let accessToken = account.access_token;
        const now = Date.now();

        if (!accessToken || account.expiry_timestamp < now + 300000) {
          const tokenRes = await refreshAccessToken(account.refresh_token);
          accessToken = tokenRes.access_token;
          const expiryTimestamp = now + tokenRes.expires_in * 1000;
          db.prepare(
            "UPDATE accounts SET access_token = ?, expires_in = ?, expiry_timestamp = ? WHERE id = ?"
          ).run(accessToken, tokenRes.expires_in, expiryTimestamp, row.id);
        }

        const quotaData = await fetchQuota(accessToken);
        db.prepare("UPDATE accounts SET quota_data = ? WHERE id = ?").run(
          JSON.stringify(quotaData),
          row.id
        );
        success++;
      } catch (error) {
        failed++;
        details.push(`${row.id}: ${error}`);
      }
    }

    res.json({
      total: rows.length,
      success,
      failed,
      details,
    });
  } catch (error) {
    console.error("refresh_all_quotas error:", error);
    res.status(500).json({ error: String(error) });
  }
});

// check_for_updates
accountRoutes.post("/check_for_updates", async (req, res) => {
  try {
    // Fetch latest release from GitHub
    const response = await fetch(
      "https://api.github.com/repos/lbjlaq/Antigravity-Manager/releases/latest"
    );
    const data = (await response.json()) as any;

    const currentVersion = "3.1.1";
    const latestVersion = data.tag_name?.replace("v", "") || currentVersion;
    const hasUpdate = latestVersion !== currentVersion;

    res.json({
      has_update: hasUpdate,
      latest_version: latestVersion,
      current_version: currentVersion,
      download_url:
        data.html_url ||
        "https://github.com/lbjlaq/Antigravity-Manager/releases",
    });
  } catch (error) {
    console.error("check_for_updates error:", error);
    res.json({
      has_update: false,
      latest_version: "3.1.1",
      current_version: "3.1.1",
      download_url: "https://github.com/lbjlaq/Antigravity-Manager/releases",
    });
  }
});
