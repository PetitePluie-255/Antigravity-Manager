// 模型名称映射
use once_cell::sync::Lazy;
use std::collections::HashMap;

static CLAUDE_TO_GEMINI: Lazy<HashMap<&'static str, &'static str>> = Lazy::new(|| {
    let mut m = HashMap::new();

    // 直接支持的模型
    m.insert("claude-opus-4-5-thinking", "claude-opus-4-5-thinking");
    m.insert("claude-sonnet-4-5", "claude-sonnet-4-5");
    m.insert("claude-sonnet-4-5-thinking", "claude-sonnet-4-5-thinking");

    // 别名映射
    m.insert("claude-sonnet-4-5-20250929", "claude-sonnet-4-5-thinking");
    m.insert("claude-3-5-sonnet-20241022", "claude-sonnet-4-5");
    m.insert("claude-3-5-sonnet-20240620", "claude-sonnet-4-5");
    m.insert("claude-opus-4", "claude-opus-4-5-thinking");
    m.insert("claude-opus-4-5-20251101", "claude-opus-4-5-thinking");
    m.insert("claude-haiku-4", "claude-sonnet-4-5");
    m.insert("claude-3-haiku-20240307", "claude-sonnet-4-5");
    m.insert("claude-haiku-4-5-20251001", "claude-sonnet-4-5");
    // OpenAI 协议映射表
    m.insert("gpt-4", "gemini-2.5-pro");
    m.insert("gpt-4-turbo", "gemini-2.5-pro");
    m.insert("gpt-4-turbo-preview", "gemini-2.5-pro");
    m.insert("gpt-4-0125-preview", "gemini-2.5-pro");
    m.insert("gpt-4-1106-preview", "gemini-2.5-pro");
    m.insert("gpt-4-0613", "gemini-2.5-pro");

    m.insert("gpt-4o", "gemini-2.5-pro");
    m.insert("gpt-4o-2024-05-13", "gemini-2.5-pro");
    m.insert("gpt-4o-2024-08-06", "gemini-2.5-pro");

    m.insert("gpt-4o-mini", "gemini-2.5-flash");
    m.insert("gpt-4o-mini-2024-07-18", "gemini-2.5-flash");

    m.insert("gpt-3.5-turbo", "gemini-2.5-flash");
    m.insert("gpt-3.5-turbo-16k", "gemini-2.5-flash");
    m.insert("gpt-3.5-turbo-0125", "gemini-2.5-flash");
    m.insert("gpt-3.5-turbo-1106", "gemini-2.5-flash");
    m.insert("gpt-3.5-turbo-0613", "gemini-2.5-flash");

    // Gemini 协议映射表
    m.insert("gemini-2.5-flash-lite", "gemini-2.5-flash-lite");
    m.insert("gemini-2.5-flash-thinking", "gemini-2.5-flash-thinking");
    m.insert("gemini-3-pro-low", "gemini-3-pro-low");
    m.insert("gemini-3-pro-high", "gemini-3-pro-high");
    m.insert("gemini-3-pro-preview", "gemini-3-pro-preview");
    m.insert("gemini-2.5-flash", "gemini-2.5-flash");
    m.insert("gemini-3-flash", "gemini-3-flash");
    m.insert("gemini-3-pro-image", "gemini-3-pro-image");

    m
});

pub fn map_claude_model_to_gemini(input: &str) -> String {
    // 1. Check exact match in map
    if let Some(mapped) = CLAUDE_TO_GEMINI.get(input) {
        return mapped.to_string();
    }

    // 2. Pass-through known prefixes (gemini-, -thinking) to support dynamic suffixes
    if input.starts_with("gemini-") || input.contains("thinking") {
        return input.to_string();
    }

    // 3. Fallback to default
    "claude-sonnet-4-5".to_string()
}

/// 核心模型路由解析引擎
/// 优先级：Custom Mapping (精确) > Group Mapping (家族) > System Mapping (内置插件)
pub fn resolve_model_route(
    original_model: &str,
    custom_mapping: &std::collections::HashMap<String, String>,
    openai_mapping: &std::collections::HashMap<String, String>,
    anthropic_mapping: &std::collections::HashMap<String, String>,
) -> String {
    // 1. 检查自定义精确映射 (优先级最高)
    if let Some(target) = custom_mapping.get(original_model) {
        tracing::info!(
            "[Router] 使用自定义精确映射: {} -> {}",
            original_model,
            target
        );
        return target.clone();
    }

    let lower_model = original_model.to_lowercase();

    // 2. 检查家族分组映射 (OpenAI 系)
    // GPT-4 系列 (含 GPT-4 经典, o1, o3 等, 排除 4o/mini/turbo)
    if (lower_model.starts_with("gpt-4")
        && !lower_model.contains("o")
        && !lower_model.contains("mini")
        && !lower_model.contains("turbo"))
        || lower_model.starts_with("o1-")
        || lower_model.starts_with("o3-")
        || lower_model == "gpt-4"
    {
        if let Some(target) = openai_mapping.get("gpt-4-series") {
            tracing::info!(
                "[Router] 使用 GPT-4 系列映射: {} -> {}",
                original_model,
                target
            );
            return target.clone();
        }
    }

    // GPT-4o / 3.5 系列 (均衡与轻量, 含 4o, mini, turbo)
    if lower_model.contains("4o")
        || lower_model.starts_with("gpt-3.5")
        || (lower_model.contains("mini") && !lower_model.contains("gemini"))
        || lower_model.contains("turbo")
    {
        if let Some(target) = openai_mapping.get("gpt-4o-series") {
            tracing::info!(
                "[Router] 使用 GPT-4o/3.5 系列映射: {} -> {}",
                original_model,
                target
            );
            return target.clone();
        }
    }

    // GPT-5 系列 (gpt-5, gpt-5.1, gpt-5.2 等)
    if lower_model.starts_with("gpt-5") {
        // 优先使用 gpt-5-series 映射，如果没有则使用 gpt-4-series
        if let Some(target) = openai_mapping.get("gpt-5-series") {
            tracing::info!(
                "[Router] 使用 GPT-5 系列映射: {} -> {}",
                original_model,
                target
            );
            return target.clone();
        }
        if let Some(target) = openai_mapping.get("gpt-4-series") {
            tracing::info!(
                "[Router] 使用 GPT-4 系列映射 (GPT-5 fallback): {} -> {}",
                original_model,
                target
            );
            return target.clone();
        }
    }

    // 3. 检查家族分组映射 (Anthropic 系)
    if lower_model.starts_with("claude-") {
        let family_key = if lower_model.contains("4-5") || lower_model.contains("4.5") {
            "claude-4.5-series"
        } else if lower_model.contains("3-5") || lower_model.contains("3.5") {
            "claude-3.5-series"
        } else {
            "claude-default"
        };

        if let Some(target) = anthropic_mapping.get(family_key) {
            tracing::warn!(
                "[Router] 使用 Anthropic 系列映射: {} -> {}",
                original_model,
                target
            );
            return target.clone();
        }

        // 兜底兼容旧版精确映射
        if let Some(target) = anthropic_mapping.get(original_model) {
            return target.clone();
        }
    }

    // 4. 下沉到系统默认映射逻辑
    map_claude_model_to_gemini(original_model)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_mapping() {
        assert_eq!(
            map_claude_model_to_gemini("claude-3-5-sonnet-20241022"),
            "claude-sonnet-4-5"
        );
        assert_eq!(
            map_claude_model_to_gemini("claude-opus-4"),
            "claude-opus-4-5-thinking"
        );
        // Test gemini pass-through (should not be caught by "mini" rule)
        assert_eq!(
            map_claude_model_to_gemini("gemini-2.5-flash-mini-test"),
            "gemini-2.5-flash-mini-test"
        );
        assert_eq!(
            map_claude_model_to_gemini("unknown-model"),
            "claude-sonnet-4-5"
        );
    }
}
