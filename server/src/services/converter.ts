// ===== OpenAI Format Types =====

export interface OpenAIMessage {
  role: string;
  content: string | ContentPart[];
  name?: string;
}

export interface ContentPart {
  type: "text" | "image_url";
  text?: string;
  image_url?: { url: string };
}

export interface OpenAIChatRequest {
  model: string;
  messages: OpenAIMessage[];
  temperature?: number;
  top_p?: number;
  max_tokens?: number;
  stream?: boolean;
  size?: string; // For image generation
  quality?: string;
}

// ===== Anthropic Format Types =====

export interface AnthropicContent {
  type: "text" | "thinking" | "image";
  text?: string;
  thinking?: string;
  signature?: string;
  source?: {
    type: string;
    media_type: string;
    data: string;
  };
}

export interface AnthropicMessage {
  role: string;
  content: string | AnthropicContent[];
}

export interface AnthropicChatRequest {
  model: string;
  messages: AnthropicMessage[];
  system?: string;
  max_tokens?: number;
  stream?: boolean;
  temperature?: number;
  top_p?: number;
}

// ===== Gemini Format Types =====

export interface GeminiInlineData {
  mimeType: string;
  data: string;
}

export interface GeminiPart {
  text?: string;
  inlineData?: GeminiInlineData;
  thoughtSignature?: string;
}

export interface GeminiContent {
  role: string;
  parts: GeminiPart[];
}

// ===== Converters =====

/**
 * Extract text content from OpenAI message
 */
function getMessageText(content: string | ContentPart[]): string {
  if (typeof content === "string") {
    return content;
  }
  return content
    .filter((p) => p.type === "text")
    .map((p) => p.text || "")
    .join("");
}

/**
 * Convert OpenAI messages to Gemini contents
 */
export function convertOpenAIToGemini(
  messages: OpenAIMessage[]
): GeminiContent[] {
  const contents: GeminiContent[] = [];

  // Regex for extracting base64 images from markdown
  const markdownImageRegex =
    /!\[.*?\]\(data:\s*(image\/[a-zA-Z+-]+)\s*;\s*base64\s*,\s*([a-zA-Z0-9+/=\s]+)\)/g;
  const dataUrlRegex =
    /data:\s*(image\/[a-zA-Z+-]+)\s*;\s*base64\s*,\s*([a-zA-Z0-9+/=\s]+)/;

  for (const msg of messages) {
    // Role mapping
    const role =
      msg.role === "assistant"
        ? "model"
        : msg.role === "system"
        ? "user"
        : msg.role;

    const parts: GeminiPart[] = [];

    if (typeof msg.content === "string") {
      // Parse markdown images from string content
      let lastIndex = 0;
      let match;

      markdownImageRegex.lastIndex = 0;
      while ((match = markdownImageRegex.exec(msg.content)) !== null) {
        // Add text before the image
        if (match.index > lastIndex) {
          const textPart = msg.content.slice(lastIndex, match.index);
          if (textPart.trim()) {
            parts.push({ text: textPart });
          }
        }

        // Add image
        const mimeType = match[1];
        const data = match[2].replace(/\s/g, "");
        parts.push({
          inlineData: { mimeType, data },
        });

        lastIndex = match.index + match[0].length;
      }

      // Add remaining text
      if (lastIndex < msg.content.length) {
        const textPart = msg.content.slice(lastIndex);
        if (textPart.trim()) {
          parts.push({ text: textPart });
        }
      }

      // If no parts were added, add empty text
      if (parts.length === 0) {
        parts.push({ text: msg.content });
      }
    } else {
      // Array content (multimodal)
      for (const part of msg.content) {
        if (part.type === "text") {
          parts.push({ text: part.text || "" });
        } else if (part.type === "image_url" && part.image_url) {
          const match = dataUrlRegex.exec(part.image_url.url);
          if (match) {
            parts.push({
              inlineData: {
                mimeType: match[1],
                data: match[2].replace(/\s/g, ""),
              },
            });
          }
        }
      }
    }

    if (parts.length === 0) {
      parts.push({ text: "" });
    }

    contents.push({ role, parts });
  }

  // Merge consecutive user messages (Gemini requirement)
  let i = 1;
  while (i < contents.length) {
    if (contents[i].role === "user" && contents[i - 1].role === "user") {
      contents[i - 1].parts.push({ text: "\n\n" });
      contents[i - 1].parts.push(...contents[i].parts);
      contents.splice(i, 1);
    } else {
      i++;
    }
  }

  return contents;
}

/**
 * Convert Anthropic messages to Gemini contents
 */
export function convertAnthropicToGemini(
  request: AnthropicChatRequest
): GeminiContent[] {
  const contents: GeminiContent[] = [];

  for (const msg of request.messages) {
    const role = msg.role === "assistant" ? "model" : "user";
    const parts: GeminiPart[] = [];

    // Handle string or array content
    const contentArray =
      typeof msg.content === "string"
        ? [{ type: "text" as const, text: msg.content }]
        : msg.content;

    for (const content of contentArray) {
      if (content.type === "text") {
        parts.push({ text: content.text || "" });
      } else if (content.type === "image" && content.source) {
        if (content.source.type === "base64") {
          parts.push({
            inlineData: {
              mimeType: content.source.media_type,
              data: content.source.data,
            },
          });
        }
      }
      // Skip 'thinking' blocks
    }

    if (parts.length === 0) {
      parts.push({ text: "" });
    }

    contents.push({ role, parts });
  }

  // Merge consecutive same-role messages
  let i = 1;
  while (i < contents.length) {
    if (contents[i].role === contents[i - 1].role) {
      contents[i - 1].parts.push(...contents[i].parts);
      contents.splice(i, 1);
    } else {
      i++;
    }
  }

  return contents;
}

/**
 * Convert Gemini response to OpenAI format
 */
export function convertGeminiToOpenAI(
  geminiResponse: any,
  model: string,
  isStream: boolean = false
): any {
  const id = `chatcmpl-${Date.now()}`;
  const created = Math.floor(Date.now() / 1000);

  if (isStream) {
    // SSE chunk format
    const candidates =
      geminiResponse.candidates || geminiResponse.response?.candidates;
    const text = candidates?.[0]?.content?.parts?.[0]?.text || "";
    const finishReason = candidates?.[0]?.finishReason;

    return {
      id,
      object: "chat.completion.chunk",
      created,
      model,
      choices: [
        {
          index: 0,
          delta: finishReason ? {} : { content: text },
          finish_reason: finishReason ? "stop" : null,
        },
      ],
    };
  } else {
    // Non-stream format
    const candidates =
      geminiResponse.candidates || geminiResponse.response?.candidates;
    const text = candidates?.[0]?.content?.parts?.[0]?.text || "";

    return {
      id,
      object: "chat.completion",
      created,
      model,
      choices: [
        {
          index: 0,
          message: {
            role: "assistant",
            content: text,
          },
          finish_reason: "stop",
        },
      ],
      usage: {
        prompt_tokens: 0,
        completion_tokens: 0,
        total_tokens: 0,
      },
    };
  }
}

/**
 * Convert Gemini response to Anthropic format
 */
export function convertGeminiToAnthropic(
  geminiResponse: any,
  model: string,
  isStream: boolean = false
): any {
  const id = `msg_${Date.now()}`;

  if (isStream) {
    const candidates =
      geminiResponse.candidates || geminiResponse.response?.candidates;
    const text = candidates?.[0]?.content?.parts?.[0]?.text || "";
    const finishReason = candidates?.[0]?.finishReason;

    if (finishReason) {
      return {
        type: "message_stop",
      };
    }

    return {
      type: "content_block_delta",
      index: 0,
      delta: {
        type: "text_delta",
        text,
      },
    };
  } else {
    const candidates =
      geminiResponse.candidates || geminiResponse.response?.candidates;
    const text = candidates?.[0]?.content?.parts?.[0]?.text || "";

    return {
      id,
      type: "message",
      role: "assistant",
      content: [
        {
          type: "text",
          text,
        },
      ],
      model,
      stop_reason: "end_turn",
      usage: {
        input_tokens: 0,
        output_tokens: 0,
      },
    };
  }
}
