// Gemini → OpenAI 响应转换
use super::models::*;
use serde_json::Value;

pub fn transform_openai_response(gemini_response: &Value) -> OpenAIResponse {
    // 解包 response 字段
    let raw = gemini_response.get("response").unwrap_or(gemini_response);

    // 提取文本
    let text_part = raw
        .get("candidates")
        .and_then(|c| c.get(0))
        .and_then(|cand| cand.get("content"))
        .and_then(|content| content.get("parts"))
        .and_then(|parts| parts.get(0))
        .and_then(|part| part.get("text"))
        .and_then(|t| t.as_str())
        .unwrap_or("");

    // 提取图片 (inlineData)
    let image_part = raw
        .get("candidates")
        .and_then(|c| c.get(0))
        .and_then(|cand| cand.get("content"))
        .and_then(|content| content.get("parts"))
        .and_then(|parts| parts.get(0))
        .and_then(|part| part.get("inlineData"));

    let text = if let Some(img) = image_part {
        let mime_type = img.get("mimeType").and_then(|v| v.as_str()).unwrap_or("image/png");
        let data = img.get("data").and_then(|v| v.as_str()).unwrap_or("");
        if !data.is_empty() {
            format!("![image](data:{};base64,{})", mime_type, data)
        } else {
            text_part.to_string()
        }
    } else {
        text_part.to_string()
    };

    // 提取 finish_reason
    let finish_reason = raw
        .get("candidates")
        .and_then(|c| c.get(0))
        .and_then(|cand| cand.get("finishReason"))
        .and_then(|f| f.as_str())
        .map(|f| {
            if f == "STOP" {
                "stop"
            } else if f == "MAX_TOKENS" {
                "length"
            } else {
                "stop"
            }
        })
        .unwrap_or("stop");

    OpenAIResponse {
        id: raw
            .get("responseId")
            .and_then(|v| v.as_str())
            .unwrap_or("resp_unknown")
            .to_string(),
        object: "chat.completion".to_string(),
        created: chrono::Utc::now().timestamp() as u64,
        model: raw
            .get("modelVersion")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string(),
        choices: vec![Choice {
            index: 0,
            message: OpenAIMessage {
                role: "assistant".to_string(),
                content: text.to_string(),
            },
            finish_reason: finish_reason.to_string(),
        }],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_transform_openai_response() {
        let gemini_resp = json!({
            "candidates": [{
                "content": {
                    "parts": [{"text": "Hello!"}]
                },
                "finishReason": "STOP"
            }],
            "modelVersion": "gemini-2.5-pro",
            "responseId": "resp_123"
        });

        let result = transform_openai_response(&gemini_resp);
        assert_eq!(result.object, "chat.completion");
        assert_eq!(result.choices[0].message.content, "Hello!");
        assert_eq!(result.choices[0].finish_reason, "stop");
    }
}
