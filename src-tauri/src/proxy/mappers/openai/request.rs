// OpenAI → Gemini 请求转换
use super::models::*;
use serde_json::{json, Value};

pub fn transform_openai_request(request: &OpenAIRequest, project_id: &str, mapped_model: &str) -> Value {
    // Resolve grounding config
    let config = crate::proxy::mappers::common_utils::resolve_request_config(&request.model, mapped_model);

    tracing::info!("[Debug] OpenAI Request: original='{}', mapped='{}', type='{}', has_image_config={}", 
        request.model, mapped_model, config.request_type, config.image_config.is_some());
    
    // 构建 Gemini contents
    let contents: Vec<Value> = request
        .messages
        .iter()
        .map(|msg| {
            let role = if msg.role == "assistant" { "model" } else { &msg.role };
            json!({
                "role": role,
                "parts": [{"text": msg.content}]
            })
        })
        .collect();

    // 构建请求体
    let mut inner_request = json!({
        "contents": contents,
        "generationConfig": {
            "maxOutputTokens": request.max_tokens.unwrap_or(65535),
            "temperature": request.temperature.unwrap_or(1.0),
            "topP": request.top_p.unwrap_or(1.0), 
        },
        "safetySettings": [
            { "category": "HARM_CATEGORY_HARASSMENT", "threshold": "OFF" },
            { "category": "HARM_CATEGORY_HATE_SPEECH", "threshold": "OFF" },
            { "category": "HARM_CATEGORY_SEXUALLY_EXPLICIT", "threshold": "OFF" },
            { "category": "HARM_CATEGORY_DANGEROUS_CONTENT", "threshold": "OFF" },
        ]
    });
    
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
                 gen_obj.remove("responseModalities");
                 gen_obj.insert("imageConfig".to_string(), image_config);
             }
         }
    }

    json!({
        "project": project_id,
        "requestId": format!("openai-{}", uuid::Uuid::new_v4()),
        "request": inner_request,
        "model": config.final_model,
        "userAgent": "antigravity-openai", 
        "requestType": config.request_type
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[test]
    fn test_transform_openai_request() {
        let req = OpenAIRequest {
            model: "gpt-4".to_string(),
            messages: vec![OpenAIMessage {
                role: "user".to_string(),
                content: "Hello".to_string(),
            }],
            stream: false,
            max_tokens: None,
            temperature: None,
            top_p: None,
        };

        let result = transform_openai_request(&req, "test-project", "gemini-1.5-pro-latest");
        assert_eq!(result["project"], "test-project");
        assert!(result["requestId"].as_str().unwrap().starts_with("openai-"));
    }
}
