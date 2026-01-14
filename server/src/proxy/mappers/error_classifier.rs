// 错误分类模块 - 将底层错误转换为用户友好的消息
use reqwest::Error;

/// 分类流式响应错误并返回错误类型、英文消息和 i18n key
///
/// 返回值: (错误类型, 英文错误消息, i18n_key)
/// - 错误类型: 用于日志和错误码
/// - 英文消息: fallback 消息,供非浏览器客户端使用
/// - i18n_key: 前端翻译键,供浏览器客户端本地化
pub fn classify_stream_error(error: &Error) -> (&'static str, &'static str, &'static str) {
    if error.is_timeout() {
        (
            "timeout_error",
            "Request timeout, please check your network connection",
            "errors.stream.timeout_error",
        )
    } else if error.is_connect() {
        (
            "connection_error",
            "Connection failed, please check your network or proxy settings",
            "errors.stream.connection_error",
        )
    } else if error.is_decode() {
        (
            "decode_error",
            "Network unstable, data transmission interrupted. Try: 1) Check network 2) Switch proxy 3) Retry",
            "errors.stream.decode_error"
        )
    } else if error.is_body() {
        (
            "stream_error",
            "Stream transmission error, please retry later",
            "errors.stream.stream_error",
        )
    } else {
        (
            "unknown_error",
            "Unknown error occurred",
            "errors.stream.unknown_error",
        )
    }
}
