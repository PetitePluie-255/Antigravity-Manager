import { Router } from "express";
import { db, DATA_DIR } from "../db/sqlite.js";
import crypto from "crypto";
import { getTokenCount } from "../services/tokenManager.js";

export const configRoutes: Router = Router();

// Default config
const defaultConfig = {
  language: "zh",
  theme: "system",
  auto_refresh: false,
  refresh_interval: 15,
  auto_sync: false,
  sync_interval: 5,
  proxy: {
    enabled: false,
    port: 8045,
    api_key: "",
    auto_start: false,
    request_timeout: 120,
    upstream_proxy: {
      enabled: false,
      url: "",
    },
  },
};

// load_config
configRoutes.post("/load_config", (req, res) => {
  try {
    const row = db
      .prepare("SELECT value FROM config WHERE key = 'app_config'")
      .get() as { value: string } | undefined;
    if (row) {
      const config = JSON.parse(row.value);
      res.json({ ...defaultConfig, ...config });
    } else {
      res.json(defaultConfig);
    }
  } catch (error) {
    console.error("load_config error:", error);
    res.json(defaultConfig);
  }
});

// save_config
configRoutes.post("/save_config", (req, res) => {
  try {
    const { config } = req.body;
    const value = JSON.stringify(config);

    db.prepare(
      `
            INSERT INTO config (key, value) VALUES ('app_config', ?)
            ON CONFLICT(key) DO UPDATE SET value = excluded.value
        `
    ).run(value);

    res.json({ success: true });
  } catch (error) {
    console.error("save_config error:", error);
    res.status(500).json({ error: String(error) });
  }
});

// save_text_file - For web, we return the content as download
configRoutes.post("/save_text_file", (req, res) => {
  // In web mode, file saving is handled client-side via blob download
  // This endpoint is mainly for Tauri compatibility
  res.json({ success: true, message: "File saved client-side in web mode" });
});

// get_data_dir_path - Expose server-side data directory
configRoutes.post("/get_data_dir_path", (req, res) => {
  res.json(DATA_DIR);
});

// Proxy-related endpoints
configRoutes.post("/get_proxy_status", (req, res) => {
  // In Node.js version, proxy is always part of the main server
  const port = parseInt(process.env.PORT || "3000");
  res.json({
    running: true,
    port: port,
    base_url: `http://localhost:${port}/proxy`,
    active_accounts: getTokenCount(),
  });
});

// Since proxy runs with the main server, start/stop are no-ops or just return current state
configRoutes.post("/start_proxy_service", (req, res) => {
  res.json({
    success: true,
    message: "Proxy service is managed by the main server",
  });
});

configRoutes.post("/stop_proxy_service", (req, res) => {
  res
    .status(400)
    .json({ error: "Cannot stop proxy service separately in web mode" });
});

configRoutes.post("/generate_api_key", (req, res) => {
  try {
    const newKey = "sk-antigravity-" + crypto.randomUUID().replace(/-/g, "");

    // Update config
    const row = db
      .prepare("SELECT value FROM config WHERE key = 'app_config'")
      .get() as { value: string } | undefined;
    let config = row ? JSON.parse(row.value) : {};

    config.proxy = { ...config.proxy, api_key: newKey };

    db.prepare(
      "INSERT INTO config (key, value) VALUES ('app_config', ?) ON CONFLICT(key) DO UPDATE SET value = excluded.value"
    ).run(JSON.stringify(config));

    res.json(newKey);
  } catch (error) {
    console.error("generate_api_key error:", error);
    res.status(500).json({ error: String(error) });
  }
});

configRoutes.post("/update_model_mapping", (req, res) => {
  try {
    const { mapping } = req.body;

    // Update config
    const row = db
      .prepare("SELECT value FROM config WHERE key = 'app_config'")
      .get() as { value: string } | undefined;
    let config = row ? JSON.parse(row.value) : {};

    config.proxy = { ...config.proxy, anthropic_mapping: mapping };

    db.prepare(
      "INSERT INTO config (key, value) VALUES ('app_config', ?) ON CONFLICT(key) DO UPDATE SET value = excluded.value"
    ).run(JSON.stringify(config));

    // Note: The proxy service needs to reload this mapping or read from DB on each request
    // For now we just save it

    res.json({ success: true });
  } catch (error) {
    console.error("update_model_mapping error:", error);
    res.status(500).json({ error: String(error) });
  }
});
