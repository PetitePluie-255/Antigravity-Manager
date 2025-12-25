use dashmap::DashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct ProxyToken {
    pub account_id: String,
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: i64,
    pub timestamp: i64,
    pub email: String,
    pub account_path: PathBuf,  // 账号文件路径，用于更新
    pub project_id: Option<String>,
    pub session_id: String,  // sessionId
}

pub struct TokenManager {
    tokens: Arc<DashMap<String, ProxyToken>>,  // account_id -> ProxyToken
    current_index: Arc<AtomicUsize>,
    data_dir: PathBuf,
}

impl TokenManager {
    /// 创建新的 TokenManager
    pub fn new(data_dir: PathBuf) -> Self {
        Self {
            tokens: Arc::new(DashMap::new()),
            current_index: Arc::new(AtomicUsize::new(0)),
            data_dir,
        }
    }
    
    /// 从主应用账号目录加载所有账号
    pub async fn load_accounts(&self) -> Result<usize, String> {
        let accounts_dir = self.data_dir.join("accounts");
        
        if !accounts_dir.exists() {
            return Err(format!("账号目录不存在: {:?}", accounts_dir));
        }
        
        let entries = std::fs::read_dir(&accounts_dir)
            .map_err(|e| format!("读取账号目录失败: {}", e))?;
        
        let mut count = 0;
        
        for entry in entries {
            let entry = entry.map_err(|e| format!("读取目录项失败: {}", e))?;
            let path = entry.path();
            
            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }
            
            // 尝试加载账号
            match self.load_single_account(&path).await {
                Ok(Some(token)) => {
                    let account_id = token.account_id.clone();
                    self.tokens.insert(account_id, token);
                    count += 1;
                },
                Ok(None) => {
                    // 跳过无效账号
                },
                Err(e) => {
                    tracing::warn!("加载账号失败 {:?}: {}", path, e);
                }
            }
        }
        
        Ok(count)
    }
    
    /// 加载单个账号
    async fn load_single_account(&self, path: &PathBuf) -> Result<Option<ProxyToken>, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("读取文件失败: {}", e))?;
        
        let account: serde_json::Value = serde_json::from_str(&content)
            .map_err(|e| format!("解析 JSON 失败: {}", e))?;
        
        let account_id = account["id"].as_str()
            .ok_or("缺少 id 字段")?
            .to_string();
        
        let email = account["email"].as_str()
            .ok_or("缺少 email 字段")?
            .to_string();
        
        let token_obj = account["token"].as_object()
            .ok_or("缺少 token 字段")?;
        
        let access_token = token_obj["access_token"].as_str()
            .ok_or("缺少 access_token")?
            .to_string();
        
        let refresh_token = token_obj["refresh_token"].as_str()
            .ok_or("缺少 refresh_token")?
            .to_string();
        
        let expires_in = token_obj["expires_in"].as_i64()
            .ok_or("缺少 expires_in")?;
        
        let timestamp = token_obj["expiry_timestamp"].as_i64()
            .ok_or("缺少 expiry_timestamp")?;
        
        // project_id 和 session_id 是可选的
        let project_id = token_obj.get("project_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let session_id = token_obj.get("session_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| generate_session_id());
        
        Ok(Some(ProxyToken {
            account_id,
            access_token,
            refresh_token,
            expires_in,
            timestamp,
            email,
            account_path: path.clone(),
            project_id,
            session_id,
        }))
    }
    
    /// 获取当前可用的 Token（轮换机制）
    /// 参数 `_quota_group` 用于区分 "claude" vs "gemini" 组（暂未完全实现分组逻辑，目前全局轮换）
    /// 返回 (access_token, project_id)
    pub async fn get_token(&self, _quota_group: &str) -> Result<(String, String), String> {
        let total = self.tokens.len();
        if total == 0 {
            return Err("Token pool is empty".to_string());
        }
        
        // 简单轮换策略 (Round Robin)
        // TODO: 基于 quota_group 筛选 tokens
        let idx = self.current_index.fetch_add(1, Ordering::SeqCst) % total;
        
        // 获取 Token 对象 (Clone to avoid holding lock across await)
        let mut token = self.tokens.iter()
            .nth(idx)
            .map(|entry| entry.value().clone())
            .ok_or("Failed to retrieve token from pool")?;
        
        // 检查 token 是否过期（提前5分钟刷新）
        let now = chrono::Utc::now().timestamp();
        if now >= token.timestamp - 300 {
            tracing::info!("账号 {} (Group: {}) 的 token 即将过期，正在刷新...", token.email, _quota_group);
            
            // 调用 OAuth 刷新 token
            match crate::modules::oauth::refresh_access_token(&token.refresh_token).await {
                Ok(token_response) => {
                    tracing::info!("Token 刷新成功！有效期: {} 秒", token_response.expires_in);
                    
                    // 更新 token 信息
                    token.access_token = token_response.access_token.clone();
                    token.expires_in = token_response.expires_in;
                    token.timestamp = now + token_response.expires_in; // Use 'now' for timestamp
                    
                    // 更新 DashMap 中的值
                    if let Some(mut entry) = self.tokens.get_mut(&token.account_id) {
                        entry.access_token = token.access_token.clone();
                        entry.expires_in = token.expires_in;
                        entry.timestamp = token.timestamp;
                    }
                    // 同样更新本地文件 (省略, 假设只在内存更新, 只有 add_account 会写文件)
                    // TODO: 持久化刷新后的 Token 
                }
                Err(e) => {
                    tracing::error!("Token 刷新失败: {}，尝试下一个账号", e);
                    // 递归尝试下一个 (限制递归深度？或者这里简单报错由上层重试)
                    // 这里简单返回错误，让 client 重试逻辑处理
                    return Err(format!("Token refresh failed: {}", e));
                }
            }
        }

        // 确保有 project_id
        let project_id = if let Some(pid) = &token.project_id {
            pid.clone()
        } else {
             // 动态获取 project_id
            tracing::info!("账号 {} 缺少 project_id，尝试获取...", token.email);
            match crate::proxy::project_resolver::fetch_project_id(&token.access_token).await {
                Ok(pid) => {
                    tracing::info!("成功获取 project_id: {}", pid);
                    
                    // 更新内存
                    if let Some(mut entry) = self.tokens.get_mut(&token.account_id) {
                        entry.project_id = Some(pid.clone());
                    }
                    
                    // 尝试保存到文件 (Ignore error)
                    let _ = self.save_project_id(&token.account_id, &pid).await;
                    
                    pid
                }
                Err(e) => {
                    tracing::error!("Failed to fetch project_id for {}: {}", token.email, e);
                    return Err(format!("Failed to fetch project_id: {}", e));
                }
            }
        };

        Ok((token.access_token, project_id))
    }
    
    /// 保存 project_id 到账号文件
    async fn save_project_id(&self, account_id: &str, project_id: &str) -> Result<(), String> {
        let entry = self.tokens.get(account_id)
            .ok_or("账号不存在")?;
        
        let path = &entry.account_path;
        
        let mut content: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(path).map_err(|e| format!("读取文件失败: {}", e))?
        ).map_err(|e| format!("解析 JSON 失败: {}", e))?;
        
        content["token"]["project_id"] = serde_json::Value::String(project_id.to_string());
        
        std::fs::write(path, serde_json::to_string_pretty(&content).unwrap())
            .map_err(|e| format!("写入文件失败: {}", e))?;
        
        tracing::info!("已保存 project_id 到账号 {}", account_id);
        Ok(())
    }
    
    /// 保存刷新后的 token 到账号文件
    async fn save_refreshed_token(&self, account_id: &str, token_response: &crate::modules::oauth::TokenResponse) -> Result<(), String> {
        let entry = self.tokens.get(account_id)
            .ok_or("账号不存在")?;
        
        let path = &entry.account_path;
        
        let mut content: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(path).map_err(|e| format!("读取文件失败: {}", e))?
        ).map_err(|e| format!("解析 JSON 失败: {}", e))?;
        
        let now = chrono::Utc::now().timestamp();
        
        content["token"]["access_token"] = serde_json::Value::String(token_response.access_token.clone());
        content["token"]["expires_in"] = serde_json::Value::Number(token_response.expires_in.into());
        content["token"]["expiry_timestamp"] = serde_json::Value::Number((now + token_response.expires_in).into());
        
        std::fs::write(path, serde_json::to_string_pretty(&content).unwrap())
            .map_err(|e| format!("写入文件失败: {}", e))?;
        
        tracing::info!("已保存刷新后的 token 到账号 {}", account_id);
        Ok(())
    }
    
    /// 获取当前加载的账号数量
    pub fn len(&self) -> usize {
        self.tokens.len()
    }
    
    /// 强制使某个账号的 token 失效（用于 401 错误）
    pub fn invalidate_token(&self, account_id: &str) {
        if let Some(mut entry) = self.tokens.get_mut(account_id) {
            entry.timestamp = 0; // 设为 0 会触发下一次加载时的自动刷新
            tracing::info!("账号已标记为 token 失效，等待下次刷新: {}", account_id);
        }
    }
}

/// 生成 sessionId
/// 格式：负数大整数字符串
fn generate_session_id() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    // 生成 1e18 到 9e18 之间的负数
    let num: i64 = -rng.gen_range(1_000_000_000_000_000_000..9_000_000_000_000_000_000);
    num.to_string()
}
