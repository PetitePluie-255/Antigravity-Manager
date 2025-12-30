// CORS 中间件
use tower_http::cors::{CorsLayer, Any};

/// 创建 CORS layer
pub fn cors_layer() -> CorsLayer {
    CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cors_layer_creation() {
        let _layer = cors_layer();
        // Layer 创建成功
        assert!(true);
    }
}
