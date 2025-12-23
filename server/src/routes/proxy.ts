import { Router, Request, Response } from "express";
import {
  getToken,
  loadAccounts,
  getTokenCount,
} from "../services/tokenManager.js";
import { db } from "../db/sqlite.js";
import {
  OpenAIChatRequest,
  AnthropicChatRequest,
  convertOpenAIToGemini,
  convertAnthropicToGemini,
  convertGeminiToOpenAI,
  convertGeminiToAnthropic,
} from "../services/converter.js";
import {
  streamRequest,
  nonStreamRequest,
  parseSSEStream,
} from "../services/geminiClient.js";

export const proxyRoutes = Router();

// Helper to get config
function getProxyConfig() {
  try {
    const row = db
      .prepare("SELECT value FROM config WHERE key = 'app_config'")
      .get() as { value: string } | undefined;
    if (row) {
      const config = JSON.parse(row.value);
      return config.proxy || {};
    }
  } catch (e) {
    console.error("Error reading config:", e);
  }
  return {};
}

// Authentication Middleware
const authMiddleware = (req: Request, res: Response, next: Function) => {
  const config = getProxyConfig();

  // If no API key set in config, allow all (or default behavior)
  if (!config.api_key) {
    return next();
  }

  // Check Authorization header or x-api-key
  const authHeader = req.headers.authorization;
  const apiKeyHeader = req.headers["x-api-key"];

  let token = "";
  if (authHeader && authHeader.startsWith("Bearer ")) {
    token = authHeader.slice(7);
  } else if (typeof apiKeyHeader === "string") {
    token = apiKeyHeader;
  }

  if (token === config.api_key) {
    return next();
  }

  // Also allow if token matches one of the internal keys or if in dev mode?
  // For now strict check if key is configured
  res.status(401).json({ error: { message: "Invalid API Key" } });
};

// Apply auth middleware to all proxy routes
proxyRoutes.use(authMiddleware);

// Initialize token manager
let initialized = false;

async function ensureInitialized() {
  if (!initialized) {
    await loadAccounts();
    initialized = true;
  }
}

// Helper to map model
function mapModel(inputModel: string): string {
  const config = getProxyConfig();
  const mapping = config.anthropic_mapping || {};

  // 1. Check dynamic mapping first
  if (mapping[inputModel]) {
    return mapping[inputModel];
  }

  // 2. Default fallback mapping (hardcoded as backup)
  const defaults: Record<string, string> = {
    "claude-3-5-sonnet-20241022": "claude-sonnet-4-5",
    "claude-3-5-sonnet": "claude-sonnet-4-5",
    "claude-3-opus": "claude-opus-4-5-thinking",
    "claude-sonnet-4-5": "claude-sonnet-4-5",
    "claude-sonnet-4-5-thinking": "claude-sonnet-4-5-thinking",
    "gpt-4": "gemini-2.5-pro",
    "gpt-4o": "gemini-2.5-pro",
    "gpt-4-turbo": "gemini-2.5-pro",
    "gpt-3.5-turbo": "gemini-2.5-flash",
  };

  return defaults[inputModel] || inputModel;
}

// ===== OpenAI Compatible Endpoints =====

// GET /v1/models
proxyRoutes.get("/v1/models", async (req: Request, res: Response) => {
  const models = [
    "gemini-2.5-pro",
    "gemini-2.5-flash",
    "gemini-3-pro-high",
    "gemini-3-pro-low",
    "gemini-3-flash",
    "claude-sonnet-4-5",
    "claude-sonnet-4-5-thinking",
    "claude-opus-4-5-thinking",
  ];

  res.json({
    object: "list",
    data: models.map((id) => ({
      id,
      object: "model",
      created: Math.floor(Date.now() / 1000),
      owned_by: "antigravity",
    })),
  });
});

// POST /v1/chat/completions
proxyRoutes.post(
  "/v1/chat/completions",
  async (req: Request, res: Response) => {
    try {
      await ensureInitialized();

      const request = req.body as OpenAIChatRequest;
      const isStream = request.stream === true;

      // Get token with rotation
      const token = await getToken();
      if (!token) {
        return res
          .status(503)
          .json({ error: { message: "No available accounts" } });
      }

      // Map model name
      const model = mapModel(request.model);

      // Convert to Gemini format
      const contents = convertOpenAIToGemini(request.messages);

      // Extract system message
      const systemMsg = request.messages.find((m) => m.role === "system");
      const systemInstruction = systemMsg
        ? typeof systemMsg.content === "string"
          ? systemMsg.content
          : ""
        : undefined;

      console.log(
        `[Proxy] OpenAI request: model=${model}, stream=${isStream}, account=${token.email}`
      );

      if (isStream) {
        // Streaming response
        res.setHeader("Content-Type", "text/event-stream");
        res.setHeader("Cache-Control", "no-cache");
        res.setHeader("Connection", "keep-alive");

        try {
          const stream = await streamRequest({
            accessToken: token.accessToken,
            projectId: token.projectId || "unknown",
            sessionId: token.sessionId,
            model,
            contents,
            systemInstruction,
            temperature: request.temperature,
            topP: request.top_p,
            maxOutputTokens: request.max_tokens,
            stream: true,
          });

          for await (const chunk of parseSSEStream(stream)) {
            const openaiChunk = convertGeminiToOpenAI(chunk, model, true);
            res.write(`data: ${JSON.stringify(openaiChunk)}\n\n`);
          }

          res.write("data: [DONE]\n\n");
          res.end();
        } catch (error: any) {
          console.error("[Proxy] Stream error:", error);
          res.write(
            `data: ${JSON.stringify({ error: { message: error.message } })}\n\n`
          );
          res.end();
        }
      } else {
        // Non-streaming response
        const geminiResponse = await nonStreamRequest({
          accessToken: token.accessToken,
          projectId: token.projectId || "unknown",
          sessionId: token.sessionId,
          model,
          contents,
          systemInstruction,
          temperature: request.temperature,
          topP: request.top_p,
          maxOutputTokens: request.max_tokens,
          stream: false,
        });

        const openaiResponse = convertGeminiToOpenAI(
          geminiResponse,
          model,
          false
        );
        res.json(openaiResponse);
      }
    } catch (error: any) {
      console.error("[Proxy] Error:", error);
      res.status(500).json({ error: { message: error.message } });
    }
  }
);

// ===== Anthropic Compatible Endpoints =====

// POST /v1/messages (Anthropic format)
proxyRoutes.post("/v1/messages", async (req: Request, res: Response) => {
  try {
    await ensureInitialized();

    const request = req.body as AnthropicChatRequest;
    const isStream = request.stream === true;

    // Get token with rotation
    const token = await getToken();
    if (!token) {
      return res
        .status(503)
        .json({ error: { message: "No available accounts" } });
    }

    // Map model name
    const model = mapModel(request.model);

    // Convert to Gemini format
    const contents = convertAnthropicToGemini(request);

    console.log(
      `[Proxy] Anthropic request: model=${model}, stream=${isStream}, account=${token.email}`
    );

    if (isStream) {
      // Streaming response
      res.setHeader("Content-Type", "text/event-stream");
      res.setHeader("Cache-Control", "no-cache");
      res.setHeader("Connection", "keep-alive");

      // Send message_start event
      res.write(
        `event: message_start\ndata: ${JSON.stringify({
          type: "message_start",
          message: {
            id: `msg_${Date.now()}`,
            type: "message",
            role: "assistant",
            content: [],
            model,
          },
        })}\n\n`
      );

      // Send content_block_start
      res.write(
        `event: content_block_start\ndata: ${JSON.stringify({
          type: "content_block_start",
          index: 0,
          content_block: { type: "text", text: "" },
        })}\n\n`
      );

      try {
        const stream = await streamRequest({
          accessToken: token.accessToken,
          projectId: token.projectId || "unknown",
          sessionId: token.sessionId,
          model,
          contents,
          systemInstruction: request.system,
          temperature: request.temperature,
          topP: request.top_p,
          maxOutputTokens: request.max_tokens,
          stream: true,
        });

        for await (const chunk of parseSSEStream(stream)) {
          const anthropicChunk = convertGeminiToAnthropic(chunk, model, true);
          if (anthropicChunk.type === "content_block_delta") {
            res.write(
              `event: content_block_delta\ndata: ${JSON.stringify(
                anthropicChunk
              )}\n\n`
            );
          }
        }

        // Send content_block_stop
        res.write(
          `event: content_block_stop\ndata: ${JSON.stringify({
            type: "content_block_stop",
            index: 0,
          })}\n\n`
        );

        // Send message_stop
        res.write(
          `event: message_stop\ndata: ${JSON.stringify({
            type: "message_stop",
          })}\n\n`
        );

        res.end();
      } catch (error: any) {
        console.error("[Proxy] Stream error:", error);
        res.write(
          `event: error\ndata: ${JSON.stringify({
            error: { message: error.message },
          })}\n\n`
        );
        res.end();
      }
    } else {
      // Non-streaming response
      const geminiResponse = await nonStreamRequest({
        accessToken: token.accessToken,
        projectId: token.projectId || "unknown",
        sessionId: token.sessionId,
        model,
        contents,
        systemInstruction: request.system,
        temperature: request.temperature,
        topP: request.top_p,
        maxOutputTokens: request.max_tokens,
        stream: false,
      });

      const anthropicResponse = convertGeminiToAnthropic(
        geminiResponse,
        model,
        false
      );
      res.json(anthropicResponse);
    }
  } catch (error: any) {
    console.error("[Proxy] Error:", error);
    res.status(500).json({ error: { message: error.message } });
  }
});

// ===== Status Endpoints =====

// GET /health
proxyRoutes.get("/health", (req: Request, res: Response) => {
  res.json({ status: "ok", accounts: getTokenCount() });
});

// POST /reload (reload accounts)
proxyRoutes.post("/reload", async (req: Request, res: Response) => {
  try {
    const count = await loadAccounts();
    res.json({ success: true, accounts: count });
  } catch (error: any) {
    res.status(500).json({ error: error.message });
  }
});
