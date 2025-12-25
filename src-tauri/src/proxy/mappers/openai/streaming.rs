// OpenAI 流式转换
use bytes::{Bytes, BytesMut};
use futures::{Stream, StreamExt};
use serde_json::{json, Value};
use std::pin::Pin;
use chrono::Utc;
use uuid::Uuid;
use tracing::{info, debug};

pub fn create_openai_sse_stream(
    mut gemini_stream: Pin<Box<dyn Stream<Item = Result<Bytes, reqwest::Error>> + Send>>,
    model: String,
) -> Pin<Box<dyn Stream<Item = Result<Bytes, String>> + Send>> {
    let mut buffer = BytesMut::new();
    
    let stream = async_stream::stream! {
        while let Some(item) = gemini_stream.next().await {
            match item {
                Ok(bytes) => {
                    // Verbose logging for debugging image fragmentation
                    debug!("[OpenAI-SSE] Received chunk: {} bytes", bytes.len());
                    buffer.extend_from_slice(&bytes);
                    
                    // Process complete lines from buffer
                    while let Some(pos) = buffer.iter().position(|&b| b == b'\n') {
                        let line_raw = buffer.split_to(pos + 1);
                        if let Ok(line_str) = std::str::from_utf8(&line_raw) {
                            let line = line_str.trim();
                            if line.is_empty() { continue; }

                            if line.starts_with("data: ") {
                                let json_part = line.trim_start_matches("data: ").trim();
                                if json_part == "[DONE]" {
                                    continue;
                                }

                                if let Ok(mut json) = serde_json::from_str::<Value>(json_part) {
                                    // Handle v1internal wrapper if present
                                    let actual_data = if let Some(inner) = json.get_mut("response").map(|v| v.take()) {
                                        inner
                                    } else {
                                        json
                                    };

                                    // Extract components
                                    let candidates = actual_data.get("candidates").and_then(|c| c.as_array());
                                    let candidate = candidates.and_then(|c| c.get(0));
                                    let parts = candidate.and_then(|c| c.get("content")).and_then(|c| c.get("parts")).and_then(|p| p.as_array());

                                    let mut content_out = String::new();
                                    
                                    if let Some(parts_list) = parts {
                                        for part in parts_list {
                                            if let Some(text) = part.get("text").and_then(|t| t.as_str()) {
                                                content_out.push_str(text);
                                            }
                                            if let Some(img) = part.get("inlineData") {
                                                let mime_type = img.get("mimeType").and_then(|v| v.as_str()).unwrap_or("image/png");
                                                let data = img.get("data").and_then(|v| v.as_str()).unwrap_or("");
                                                if !data.is_empty() {
                                                    info!("[OpenAI-SSE] Detected image data: {} chars (base64)", data.len());
                                                    content_out.push_str(&format!("![image](data:{};base64,{})", mime_type, data));
                                                }
                                            }
                                        }
                                    }

                                    if content_out.is_empty() {
                                        // Skip empty chunks if no text or image was found
                                        // Unless it has a finish reason
                                        if actual_data.get("candidates").and_then(|c| c.get(0)).and_then(|c| c.get("finishReason")).is_none() {
                                            continue;
                                        }
                                    }
                                        
                                    // Extract finish reason
                                    let finish_reason = candidate.and_then(|c| c.get("finishReason"))
                                        .and_then(|f| f.as_str())
                                        .map(|f| match f {
                                            "STOP" => "stop",
                                            "MAX_TOKENS" => "length",
                                            "SAFETY" => "content_filter",
                                            _ => f,
                                        });

                                    // Construct OpenAI SSE chunk
                                    let openai_chunk = json!({
                                        "id": format!("chatcmpl-{}", Uuid::new_v4()),
                                        "object": "chat.completion.chunk",
                                        "created": Utc::now().timestamp(),
                                        "model": model,
                                        "choices": [
                                            {
                                                "index": 0,
                                                "delta": {
                                                    "content": content_out
                                                },
                                                "finish_reason": finish_reason
                                            }
                                        ]
                                    });

                                    let sse_out = format!("data: {}\n\n", serde_json::to_string(&openai_chunk).unwrap_or_default());
                                    yield Ok::<Bytes, String>(Bytes::from(sse_out));
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    yield Err(format!("Upstream error: {}", e));
                }
            }
        }
        // End of stream signal for OpenAI
        yield Ok::<Bytes, String>(Bytes::from("data: [DONE]\n\n"));
    };

    Box::pin(stream)
}
