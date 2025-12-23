import express from "express";
import cors from "cors";
import path from "path";
import { fileURLToPath } from "url";
import { accountRoutes } from "./routes/accounts.js";
import { configRoutes } from "./routes/config.js";
import { proxyRoutes } from "./routes/proxy.js";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const app = express();
const PORT = process.env.PORT || 3000;
const PROXY_PORT = process.env.PROXY_PORT || 8045;

// Middleware
app.use(cors());
app.use(express.json({ limit: "50mb" })); // Increase limit for image data

// API Routes - 模拟 Tauri invoke 命令
app.use("/api", accountRoutes);
app.use("/api", configRoutes);

// Serve static frontend in production
const staticPath =
  process.env.STATIC_PATH || path.join(__dirname, "../../dist");
app.use(express.static(staticPath));

// Proxy Routes - OpenAI/Anthropic 兼容接口
app.use("/proxy", proxyRoutes); // Keep for backward compatibility if any
app.use("/", proxyRoutes); // Support /v1/... directly

// SPA fallback
app.get("*", (req, res) => {
  if (!req.path.startsWith("/api") && !req.path.startsWith("/proxy")) {
    res.sendFile(path.join(staticPath, "index.html"));
  }
});

app.listen(PORT, () => {
  console.log(`🚀 Antigravity Server running at http://localhost:${PORT}`);
  console.log(`📁 Static files: ${staticPath}`);
  console.log(`🔌 Proxy endpoints:`);
  console.log(
    `   - OpenAI: http://localhost:${PORT}/proxy/v1/chat/completions`
  );
  console.log(`   - Anthropic: http://localhost:${PORT}/proxy/v1/messages`);
  console.log(`   - Models: http://localhost:${PORT}/proxy/v1/models`);
});
