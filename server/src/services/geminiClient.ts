import { GeminiContent } from "./converter.js";
import { v4 as uuidv4 } from "uuid";
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
    // Ignore config read errors
  }
  return undefined;
}

const GEMINI_API_URL =
  "https://daily-cloudcode-pa.sandbox.googleapis.com/v1internal:streamGenerateContent?alt=sse";
const GEMINI_API_URL_NON_STREAM =
  "https://daily-cloudcode-pa.sandbox.googleapis.com/v1internal:generateContent";

export interface GeminiRequestOptions {
  accessToken: string;
  projectId: string;
  sessionId: string;
  model: string;
  contents: GeminiContent[];
  systemInstruction?: string;
  temperature?: number;
  topP?: number;
  maxOutputTokens?: number;
  stream?: boolean;
}

/**
 * Build Antigravity internal API request body
 * 注意：这是 Antigravity 特殊的内部 API 格式，不是标准 Gemini API
 */
function buildRequestBody(options: GeminiRequestOptions): any {
  // Generation config
  const generationConfig: any = {
    temperature: options.temperature ?? 1.0,
    topP: options.topP ?? 0.95,
    maxOutputTokens: options.maxOutputTokens ?? 8096,
    candidateCount: 1,
  };

  // System instruction
  const systemInstruction = {
    role: "user",
    parts: [{ text: options.systemInstruction || "" }],
  };

  // Antigravity internal API 格式
  const body = {
    project: options.projectId,
    requestId: uuidv4(),
    model: options.model,
    userAgent: "antigravity",
    request: {
      contents: options.contents,
      systemInstruction,
      generationConfig,
      toolConfig: {
        functionCallingConfig: {
          mode: "VALIDATED",
        },
      },
      sessionId: options.sessionId,
    },
  };

  return body;
}

/**
 * Make a streaming request to Antigravity internal API
 */
export async function streamRequest(
  options: GeminiRequestOptions
): Promise<ReadableStream<Uint8Array>> {
  const body = buildRequestBody(options);

  console.log(
    `[GeminiClient] Stream request: model=${options.model}, project=${options.projectId}`
  );

  const response = await fetch(GEMINI_API_URL, {
    method: "POST",
    headers: {
      Authorization: `Bearer ${options.accessToken}`,
      Host: "daily-cloudcode-pa.sandbox.googleapis.com",
      "User-Agent": "antigravity/1.11.3 windows/amd64",
      "Content-Type": "application/json",
    },
    body: JSON.stringify(body),
    agent: getProxyAgent(),
  } as any);

  if (!response.ok) {
    const errorText = await response.text();
    throw new Error(`Gemini API error: ${response.status} - ${errorText}`);
  }

  if (!response.body) {
    throw new Error("No response body");
  }

  return response.body;
}

/**
 * Make a non-streaming request to Antigravity internal API
 */
export async function nonStreamRequest(
  options: GeminiRequestOptions
): Promise<any> {
  const body = buildRequestBody(options);

  console.log(
    `[GeminiClient] Non-stream request: model=${options.model}, project=${options.projectId}`
  );

  const response = await fetch(GEMINI_API_URL_NON_STREAM, {
    method: "POST",
    headers: {
      Authorization: `Bearer ${options.accessToken}`,
      Host: "daily-cloudcode-pa.sandbox.googleapis.com",
      "User-Agent": "antigravity/1.11.3 windows/amd64",
      "Content-Type": "application/json",
    },
    body: JSON.stringify(body),
    agent: getProxyAgent(),
  } as any);

  if (!response.ok) {
    const errorText = await response.text();
    throw new Error(`Gemini API error: ${response.status} - ${errorText}`);
  }

  return response.json();
}

/**
 * Parse SSE stream and yield JSON objects
 */
export async function* parseSSEStream(
  stream: ReadableStream<Uint8Array>
): AsyncGenerator<any> {
  const reader = stream.getReader();
  const decoder = new TextDecoder();
  let buffer = "";

  try {
    while (true) {
      const { done, value } = await reader.read();
      if (done) break;

      buffer += decoder.decode(value, { stream: true });

      // Process complete SSE events
      const lines = buffer.split("\n");
      buffer = lines.pop() || ""; // Keep incomplete line in buffer

      for (const line of lines) {
        if (line.startsWith("data: ")) {
          const data = line.slice(6);
          if (data === "[DONE]") {
            return;
          }
          try {
            const json = JSON.parse(data);
            // Antigravity API 可能返回 response 包装
            if (json.response) {
              yield json.response;
            } else {
              yield json;
            }
          } catch (e) {
            // Skip invalid JSON
          }
        }
      }
    }
  } finally {
    reader.releaseLock();
  }
}
