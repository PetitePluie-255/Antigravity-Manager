// Claude 协议处理器

use axum::{
    body::Body,
    extract::{Json, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response, Sse},
};
use bytes::Bytes;
use futures::StreamExt;
use serde_json::{json, Value};
use std::sync::Arc;
use tracing::{debug, error};

use crate::proxy::mappers::claude::{
    transform_claude_request_in, transform_response, create_claude_sse_stream, ClaudeRequest,
};
use crate::proxy::upstream::client::UpstreamClient;
use crate::proxy::token_manager::TokenManager;
use crate::proxy::server::AppState;

/// 处理 Claude messages 请求
/// 
/// 处理 Chat 消息请求流程
pub async fn handle_messages(
    State(state): State<AppState>,
    Json(request): Json<ClaudeRequest>,
) -> Response {
    tracing::info!("Received Claude request for model: {}", request.model);

    // 1. 获取 UpstreamClient
    let upstream = state.upstream.clone();
    
    // 2. 准备闭包
    // 克隆 request 供闭包使用
    let request_for_body = request.clone();
    let token_manager = state.token_manager;
    
    // 确定方法和查询字符串
    let method = if request.stream {
        "streamGenerateContent"
    } else {
        "generateContent"
    };
    let query_string = if request.stream { Some("alt=sse") } else { None };

    let max_attempts = token_manager.len().max(1);

    // 准备闭包：获取凭证
    // 注意：这个闭包不能是异步的，所以我们需要在外层准备好 token
    // 实际上，我们应该在外层循环中处理重试
    
    // 简化方案：直接在这里处理重试逻辑
    let mut last_error = String::new();
    
    for attempt in 0..max_attempts {
        // 4. 获取 Token
        let model_group = crate::proxy::common::utils::infer_quota_group(&request_for_body.model);
        let (access_token, project_id) = match token_manager.get_token(&model_group).await {
            Ok(t) => t,
            Err(e) => {
                 return (
                    StatusCode::SERVICE_UNAVAILABLE,
                    Json(json!({
                        "type": "error",
                        "error": {
                            "type": "overloaded_error",
                            "message": format!("No available accounts: {}", e)
                        }
                    }))
                ).into_response();
            }
        };
        
        // 构建请求体
        let mapped_model = crate::proxy::common::model_mapping::resolve_model_route(
            &request_for_body.model,
            &*state.custom_mapping.read().await,
            &*state.openai_mapping.read().await,
            &*state.anthropic_mapping.read().await,
        );
        
        // 传递映射后的模型名
        let mut request_with_mapped = request_for_body.clone();
        request_with_mapped.model = mapped_model;

        let gemini_body = match transform_claude_request_in(&request_with_mapped, &project_id) {
            Ok(b) => b,
            Err(e) => {
                 return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({
                        "type": "error",
                        "error": {
                            "type": "api_error",
                            "message": format!("Transform error: {}", e)
                        }
                    }))
                ).into_response();
            }
        };
        
    // 4. 上游调用
    let is_stream = request.stream;
    let method = if is_stream { "streamGenerateContent" } else { "generateContent" };
    let query = if is_stream { Some("alt=sse") } else { None };

    let response = match upstream.call_v1_internal(
        method,
        &access_token,
        gemini_body,
        query
    ).await {
            Ok(r) => r,
            Err(e) => {
                last_error = e.clone();
                tracing::warn!("Request failed on attempt {}/{}: {}", attempt + 1, max_attempts, e);
                continue;
            }
        };
        
        let status = response.status();
        
        // 成功
        if status.is_success() {
            // 处理流式响应
            if request.stream {
                let stream = response.bytes_stream();
                let gemini_stream = Box::pin(stream);
                let claude_stream = create_claude_sse_stream(gemini_stream);

                // 转换为 Bytes stream
                let sse_stream = claude_stream.map(|result| -> Result<Bytes, std::io::Error> {
                    match result {
                        Ok(bytes) => Ok(bytes),
                        Err(e) => Ok(Bytes::from(format!("data: {{\"error\":\"{}\"}}\n\n", e))),
                    }
                });

                return Response::builder()
                    .status(StatusCode::OK)
                    .header(header::CONTENT_TYPE, "text/event-stream")
                    .header(header::CACHE_CONTROL, "no-cache")
                    .header(header::CONNECTION, "keep-alive")
                    .body(Body::from_stream(sse_stream))
                    .unwrap();
            } else {
                // 处理非流式响应
                let bytes = match response.bytes().await {
                    Ok(b) => b,
                    Err(e) => return (StatusCode::BAD_GATEWAY, format!("Failed to read body: {}", e)).into_response(),
                };
                
                // Debug print
                if let Ok(text) = String::from_utf8(bytes.to_vec()) {
                    debug!("Upstream Response for Claude request: {}", text);
                }

                let gemini_resp: Value = match serde_json::from_slice(&bytes) {
                    Ok(v) => v,
                    Err(e) => return (StatusCode::BAD_GATEWAY, format!("Parse error: {}", e)).into_response(),
                };

                // 解包 response 字段（v1internal 格式）
                let raw = gemini_resp.get("response").unwrap_or(&gemini_resp);

                // 转换为 Gemini Response 结构
                let gemini_response: crate::proxy::mappers::claude::models::GeminiResponse = match serde_json::from_value(raw.clone()) {
                    Ok(r) => r,
                    Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("Convert error: {}", e)).into_response(),
                };
                
                // 转换
                let claude_response = match transform_response(&gemini_response) {
                    Ok(r) => r,
                    Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("Transform error: {}", e)).into_response(),
                };

                return Json(claude_response).into_response();
            }
        }
        
        // 处理错误
        let error_text = response.text().await.unwrap_or_else(|_| format!("HTTP {}", status));
        last_error = format!("HTTP {}: {}", status, error_text);
        
        // 429/404/403/401 - 轮换账号重试
        if status.as_u16() == 429 || status.as_u16() == 404 || status.as_u16() == 403 || status.as_u16() == 401 {
            tracing::warn!("HTTP {} on attempt {}/{}, rotating account", status, attempt + 1, max_attempts);
            continue;
        }
        
        // 其他错误直接返回
        return (status, error_text).into_response();
    }
    
    (StatusCode::TOO_MANY_REQUESTS, Json(json!({
        "type": "error",
        "error": {
            "type": "overloaded_error",
            "message": format!("All {} attempts failed. Last error: {}", max_attempts, last_error)
        }
    }))).into_response()
}

/// 列出可用模型
pub async fn handle_list_models() -> impl IntoResponse {
    Json(json!({
        "object": "list",
        "data": [
            {
                "id": "claude-sonnet-4-5",
                "object": "model",
                "created": 1706745600,
                "owned_by": "anthropic"
            },
            {
                "id": "claude-opus-4-5-thinking",
                "object": "model",
                "created": 1706745600,
                "owned_by": "anthropic"
            },
            {
                "id": "claude-3-5-sonnet-20241022",
                "object": "model",
                "created": 1706745600,
                "owned_by": "anthropic"
            }
        ]
    }))
}

/// 计算 tokens (占位符)
pub async fn handle_count_tokens(Json(_body): Json<Value>) -> impl IntoResponse {
    Json(json!({
        "input_tokens": 0,
        "output_tokens": 0
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_handle_list_models() {
        let response = handle_list_models().await.into_response();
        assert_eq!(response.status(), StatusCode::OK);
    }
}
