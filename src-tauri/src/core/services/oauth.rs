//! OAuth 服务
//! 提供 Token 刷新和 OAuth 登录功能（独立于 Tauri）

use serde::{Deserialize, Serialize};

// Google OAuth 配置 - 从环境变量获取或使用默认值
fn get_client_id() -> String {
    std::env::var("GOOGLE_CLIENT_ID").unwrap_or_else(|_| {
        "1071006060591-tmhssin2h21lcre235vtolojh4g403ep.apps.googleusercontent.com".to_string()
    })
}

fn get_client_secret() -> String {
    std::env::var("GOOGLE_CLIENT_SECRET")
        .unwrap_or_else(|_| "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".to_string())
}
const TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const USERINFO_URL: &str = "https://www.googleapis.com/oauth2/v2/userinfo";
const AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";

/// Token 响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub expires_in: i64,
    #[serde(default)]
    pub token_type: String,
    #[serde(default)]
    pub refresh_token: Option<String>,
}

/// 用户信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub email: String,
    pub name: Option<String>,
    pub given_name: Option<String>,
    pub family_name: Option<String>,
    pub picture: Option<String>,
}

impl UserInfo {
    /// 获取最佳的显示名称
    pub fn get_display_name(&self) -> Option<String> {
        if let Some(name) = &self.name {
            if !name.trim().is_empty() {
                return Some(name.clone());
            }
        }
        match (&self.given_name, &self.family_name) {
            (Some(given), Some(family)) => Some(format!("{} {}", given, family)),
            (Some(given), None) => Some(given.clone()),
            (None, Some(family)) => Some(family.clone()),
            (None, None) => None,
        }
    }
}

/// 生成 OAuth 授权 URL
pub fn get_auth_url(redirect_uri: &str) -> String {
    let scopes = vec![
        "https://www.googleapis.com/auth/cloud-platform",
        "https://www.googleapis.com/auth/userinfo.email",
        "https://www.googleapis.com/auth/userinfo.profile",
        "https://www.googleapis.com/auth/cclog",
        "https://www.googleapis.com/auth/experimentsandconfigs",
    ]
    .join(" ");

    let client_id = get_client_id();
    let params: Vec<(&str, &str)> = vec![
        ("client_id", &client_id),
        ("redirect_uri", redirect_uri),
        ("response_type", "code"),
        ("scope", &scopes),
        ("access_type", "offline"),
        ("prompt", "consent"),
        ("include_granted_scopes", "true"),
    ];

    let url = url::Url::parse_with_params(AUTH_URL, &params).expect("无效的 Auth URL");
    url.to_string()
}

/// 使用 Authorization Code 交换 Token
pub async fn exchange_code(code: &str, redirect_uri: &str) -> Result<TokenResponse, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败: {}", e))?;

    let client_id = get_client_id();
    let client_secret = get_client_secret();
    let params = [
        ("client_id", client_id.as_str()),
        ("client_secret", client_secret.as_str()),
        ("code", code),
        ("redirect_uri", redirect_uri),
        ("grant_type", "authorization_code"),
    ];

    tracing::info!("正在交换 Authorization Code...");

    let response = client
        .post(TOKEN_URL)
        .form(&params)
        .send()
        .await
        .map_err(|e| format!("Token 交换请求失败: {}", e))?;

    if response.status().is_success() {
        let token_res = response
            .json::<TokenResponse>()
            .await
            .map_err(|e| format!("Token 解析失败: {}", e))?;

        tracing::info!(
            "Token 交换成功! access_token: {}..., refresh_token: {}",
            &token_res.access_token.chars().take(20).collect::<String>(),
            if token_res.refresh_token.is_some() {
                "✓"
            } else {
                "✗ 缺失"
            }
        );

        Ok(token_res)
    } else {
        let error_text = response.text().await.unwrap_or_default();
        Err(format!("Token 交换失败: {}", error_text))
    }
}

/// 使用 refresh_token 刷新 access_token
pub async fn refresh_access_token(refresh_token: &str) -> Result<TokenResponse, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败: {}", e))?;

    let client_id = get_client_id();
    let client_secret = get_client_secret();
    let params = [
        ("client_id", client_id.as_str()),
        ("client_secret", client_secret.as_str()),
        ("refresh_token", refresh_token),
        ("grant_type", "refresh_token"),
    ];

    tracing::info!("正在刷新 Token...");

    let response = client
        .post(TOKEN_URL)
        .form(&params)
        .send()
        .await
        .map_err(|e| format!("刷新请求失败: {}", e))?;

    if response.status().is_success() {
        let token_data = response
            .json::<TokenResponse>()
            .await
            .map_err(|e| format!("刷新数据解析失败: {}", e))?;

        tracing::info!("Token 刷新成功！有效期: {} 秒", token_data.expires_in);
        Ok(token_data)
    } else {
        let error_text = response.text().await.unwrap_or_default();
        Err(format!("刷新失败: {}", error_text))
    }
}

/// 获取用户信息
pub async fn get_user_info(access_token: &str) -> Result<UserInfo, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败: {}", e))?;

    let response = client
        .get(USERINFO_URL)
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|e| format!("用户信息请求失败: {}", e))?;

    if response.status().is_success() {
        response
            .json::<UserInfo>()
            .await
            .map_err(|e| format!("用户信息解析失败: {}", e))
    } else {
        let error_text = response.text().await.unwrap_or_default();
        Err(format!("获取用户信息失败: {}", error_text))
    }
}
