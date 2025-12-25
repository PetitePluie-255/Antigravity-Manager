// Gemini v1internal 包装/解包
use serde_json::{json, Value};

/// 包装请求体为 v1internal 格式
pub fn wrap_request(body: &Value, project_id: &str, model_name: &str) -> Value {
    // 优先使用传入的 model_name (通常来自 URL)，其次尝试从 body 获取
    let final_model = if !model_name.is_empty() {
        model_name
    } else {
        body.get("model").and_then(|v| v.as_str()).unwrap_or("gemini-2.5-pro")
    };

    // 复制 body 以便修改
    let mut inner_request = body.clone();

    // 强制设置 Gemini v1internal 的最大输出 token 数
    if let Some(obj) = inner_request.as_object_mut() {
        let gen_config = obj.entry("generationConfig").or_insert_with(|| json!({}));
        if let Some(gen_obj) = gen_config.as_object_mut() {
            gen_obj.insert("maxOutputTokens".to_string(), json!(65535));
        }
    }

    // Use shared grounding logic
    let config = crate::proxy::mappers::common_utils::resolve_request_config(final_model, final_model);
    
    tracing::info!("[Debug] Gemini Wrap: original='{}', final='{}', type='{}', has_image_config={}", 
        final_model, config.final_model, config.request_type, config.image_config.is_some());
    
    // Inject googleSearch tool if needed
    if config.inject_google_search {
        crate::proxy::mappers::common_utils::inject_google_search_tool(&mut inner_request);
    }

    // Inject imageConfig if present (for image generation models)
    if let Some(image_config) = config.image_config {
         if let Some(obj) = inner_request.as_object_mut() {
             // 1. Remove tools (image generation does not support tools)
             obj.remove("tools");
             
             // 2. Remove systemInstruction (image generation does not support system prompts)
             obj.remove("systemInstruction");

             // 3. Clean generationConfig (remove thinkingConfig, responseMimeType, responseModalities etc.)
             let gen_config = obj.entry("generationConfig").or_insert_with(|| json!({}));
             if let Some(gen_obj) = gen_config.as_object_mut() {
                 gen_obj.remove("thinkingConfig");
                 gen_obj.remove("responseMimeType"); 
                 gen_obj.remove("responseModalities"); // Cherry Studio sends this, might conflict
                 gen_obj.insert("imageConfig".to_string(), image_config);
             }
         }
    }

    let final_request = json!({
        "project": project_id,
        "requestId": format!("agent-{}", uuid::Uuid::new_v4()), // 修正为 agent- 前缀
        "request": inner_request,
        "model": config.final_model,
        "userAgent": "antigravity",
        "requestType": config.request_type
    });

    final_request
}

/// 解包响应（提取 response 字段）
pub fn unwrap_response(response: &Value) -> Value {
    response.get("response").unwrap_or(response).clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wrap_request() {
        let body = json!({
            "model": "gemini-2.5-flash",
            "contents": [{"role": "user", "parts": [{"text": "Hi"}]}]
        });

        let result = wrap_request(&body, "test-project", "gemini-2.5-flash");
        assert_eq!(result["project"], "test-project");
        assert_eq!(result["model"], "gemini-2.5-flash");
        assert!(result["requestId"].as_str().unwrap().starts_with("agent-"));
    }

    #[test]
    fn test_unwrap_response() {
        let wrapped = json!({
            "response": {
                "candidates": [{"content": {"parts": [{"text": "Hello"}]}}]
            }
        });

        let result = unwrap_response(&wrapped);
        assert!(result.get("candidates").is_some());
        assert!(result.get("response").is_none());
    }
}
