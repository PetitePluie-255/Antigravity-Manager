// Claude 辅助函数
// JSON Schema 清理、签名处理等

use serde_json::Value;

/// 清理 JSON Schema (移除不支持的属性)
pub fn clean_json_schema(schema: Value) -> Value {
    let mut cleaned = schema.clone();

    // 移除 additionalProperties (Gemini 不支持)
    if let Some(obj) = cleaned.as_object_mut() {
        obj.remove("additionalProperties");

        // 递归处理 properties
        if let Some(props) = obj.get_mut("properties") {
            if let Some(props_obj) = props.as_object_mut() {
                for (_key, value) in props_obj.iter_mut() {
                    if value.is_object() {
                        *value = clean_json_schema(value.clone());
                    }
                }
            }
        }

        // 递归处理 items
        if let Some(items) = obj.get_mut("items") {
            if items.is_object() {
                *items = clean_json_schema(items.clone());
            }
        }
    }

    cleaned
}

/// 将 JSON Schema 中的类型名称转为大写 (Gemini 要求)
/// 例如: "string" -> "STRING", "integer" -> "INTEGER"
pub fn uppercase_schema_types(schema: Value) -> Value {
    let mut result = schema.clone();

    if let Some(obj) = result.as_object_mut() {
        // 处理 type 字段
        if let Some(type_val) = obj.get_mut("type") {
            if let Some(type_str) = type_val.as_str() {
                *type_val = Value::String(type_str.to_uppercase());
            }
        }

        // 递归处理 properties
        if let Some(props) = obj.get_mut("properties") {
            if let Some(props_obj) = props.as_object_mut() {
                for (_key, value) in props_obj.iter_mut() {
                    *value = uppercase_schema_types(value.clone());
                }
            }
        }

        // 递归处理 items
        if let Some(items) = obj.get_mut("items") {
            *items = uppercase_schema_types(items.clone());
        }

        // 处理 anyOf, oneOf, allOf
        for field in &["anyOf", "oneOf", "allOf"] {
            if let Some(arr) = obj.get_mut(*field) {
                if let Some(arr_val) = arr.as_array_mut() {
                    for item in arr_val.iter_mut() {
                        *item = uppercase_schema_types(item.clone());
                    }
                }
            }
        }
    }

    result
}

/// 从 Gemini UsageMetadata 转换为 Claude Usage
pub fn to_claude_usage(usage_metadata: &super::models::UsageMetadata) -> super::models::Usage {
    super::models::Usage {
        input_tokens: usage_metadata.prompt_token_count.unwrap_or(0),
        output_tokens: usage_metadata.candidates_token_count.unwrap_or(0),
    }
}

/// 提取 thoughtSignature
pub fn extract_thought_signature(part: &Value) -> Option<String> {
    part.get("thoughtSignature")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_clean_json_schema() {
        let schema = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "integer"}
            },
            "additionalProperties": false
        });

        let cleaned = clean_json_schema(schema);
        assert!(cleaned.get("additionalProperties").is_none());
        assert!(cleaned.get("properties").is_some());
    }

    #[test]
    fn test_uppercase_schema_types() {
        let schema = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "integer"},
                "tags": {
                    "type": "array",
                    "items": {"type": "string"}
                }
            }
        });

        let result = uppercase_schema_types(schema);
        assert_eq!(result["type"], "OBJECT");
        assert_eq!(result["properties"]["name"]["type"], "STRING");
        assert_eq!(result["properties"]["age"]["type"], "INTEGER");
        assert_eq!(result["properties"]["tags"]["type"], "ARRAY");
        assert_eq!(result["properties"]["tags"]["items"]["type"], "STRING");
    }

    #[test]
    fn test_to_claude_usage() {
        use super::super::models::UsageMetadata;

        let usage = UsageMetadata {
            prompt_token_count: Some(100),
            candidates_token_count: Some(50),
            total_token_count: Some(150),
        };

        let claude_usage = to_claude_usage(&usage);
        assert_eq!(claude_usage.input_tokens, 100);
        assert_eq!(claude_usage.output_tokens, 50);
    }
}
