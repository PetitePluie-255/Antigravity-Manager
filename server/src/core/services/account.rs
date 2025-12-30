//! 账户服务
//! 账户 CRUD 操作

use std::fs;
use std::sync::Mutex;
use once_cell::sync::Lazy;

use crate::core::models::{Account, AccountIndex, AccountSummary, TokenData, QuotaData};
use crate::core::traits::{StorageConfig, EventEmitter};

// 账户索引文件锁，防止并发写入
static ACCOUNT_INDEX_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

/// 账户服务
pub struct AccountService;

impl AccountService {
    /// 加载账户索引
    pub fn load_index<S: StorageConfig>(storage: &S) -> Result<AccountIndex, String> {
        let index_path = storage.accounts_index_path();
        
        if !index_path.exists() {
            return Ok(AccountIndex::new());
        }
        
        let content = fs::read_to_string(&index_path)
            .map_err(|e| format!("读取账户索引失败: {}", e))?;
        
        let index: AccountIndex = serde_json::from_str(&content)
            .map_err(|e| format!("解析账户索引失败: {}", e))?;
        
        Ok(index)
    }
    
    /// 保存账户索引 (原子化写入)
    pub fn save_index<S: StorageConfig>(storage: &S, index: &AccountIndex) -> Result<(), String> {
        let _lock = ACCOUNT_INDEX_LOCK.lock().map_err(|e| format!("获取锁失败: {}", e))?;
        
        let index_path = storage.accounts_index_path();
        
        let content = serde_json::to_string_pretty(index)
            .map_err(|e| format!("序列化账户索引失败: {}", e))?;
        
        // 原子写入
        let temp_path = index_path.with_extension("json.tmp");
        fs::write(&temp_path, &content)
            .map_err(|e| format!("写入临时文件失败: {}", e))?;
        
        fs::rename(&temp_path, &index_path)
            .map_err(|e| format!("重命名文件失败: {}", e))?;
        
        Ok(())
    }
    
    /// 加载单个账户
    pub fn load_account<S: StorageConfig>(storage: &S, account_id: &str) -> Result<Account, String> {
        let account_path = storage.accounts_dir().join(format!("{}.json", account_id));
        
        if !account_path.exists() {
            return Err(format!("账户不存在: {}", account_id));
        }
        
        let content = fs::read_to_string(&account_path)
            .map_err(|e| format!("读取账户文件失败: {}", e))?;
        
        let account: Account = serde_json::from_str(&content)
            .map_err(|e| format!("解析账户文件失败: {}", e))?;
        
        Ok(account)
    }
    
    /// 保存单个账户
    pub fn save_account<S: StorageConfig>(storage: &S, account: &Account) -> Result<(), String> {
        let accounts_dir = storage.accounts_dir();
        fs::create_dir_all(&accounts_dir)
            .map_err(|e| format!("创建账户目录失败: {}", e))?;
        
        let account_path = accounts_dir.join(format!("{}.json", account.id));
        
        let content = serde_json::to_string_pretty(account)
            .map_err(|e| format!("序列化账户失败: {}", e))?;
        
        fs::write(&account_path, &content)
            .map_err(|e| format!("写入账户文件失败: {}", e))?;
        
        Ok(())
    }
    
    /// 列出所有账户
    pub fn list_accounts<S: StorageConfig>(storage: &S) -> Result<Vec<Account>, String> {
        let index = Self::load_index(storage)?;
        
        let mut accounts = Vec::new();
        for summary in index.accounts {
            match Self::load_account(storage, &summary.id) {
                Ok(account) => accounts.push(account),
                Err(e) => {
                    tracing::warn!("加载账户 {} 失败: {}", summary.id, e);
                }
            }
        }
        
        Ok(accounts)
    }
    
    /// 添加账户
    pub fn add_account<S: StorageConfig, E: EventEmitter>(
        storage: &S,
        emitter: &E,
        email: String,
        name: Option<String>,
        token: TokenData,
    ) -> Result<Account, String> {
        let mut index = Self::load_index(storage)?;
        
        // 检查是否已存在相同邮箱的账户
        if index.accounts.iter().any(|a| a.email == email) {
            return Err(format!("账户已存在: {}", email));
        }
        
        // 创建新账户
        let id = uuid::Uuid::new_v4().to_string();
        let mut account = Account::new(id.clone(), email.clone(), token);
        account.name = name.clone();
        
        // 保存账户文件
        Self::save_account(storage, &account)?;
        
        // 更新索引
        index.accounts.push(AccountSummary {
            id: id.clone(),
            email: email.clone(),
            name,
            created_at: account.created_at,
            last_used: account.last_used,
        });
        
        // 如果是第一个账户，设为当前账户
        if index.current_account_id.is_none() {
            index.current_account_id = Some(id.clone());
        }
        
        Self::save_index(storage, &index)?;
        
        // 发送事件
        emitter.emit("account-added", &account);
        
        Ok(account)
    }
    
    /// 添加或更新账户 (upsert)
    pub fn upsert_account<S: StorageConfig, E: EventEmitter>(
        storage: &S,
        emitter: &E,
        email: String,
        name: Option<String>,
        token: TokenData,
    ) -> Result<Account, String> {
        let index = Self::load_index(storage)?;
        
        // 检查是否已存在
        if let Some(existing) = index.accounts.iter().find(|a| a.email == email) {
            // 更新现有账户
            let mut account = Self::load_account(storage, &existing.id)?;
            account.token = token;
            account.name = name;
            account.update_last_used();
            
            Self::save_account(storage, &account)?;
            emitter.emit("account-updated", &account);
            
            return Ok(account);
        }
        
        // 添加新账户
        Self::add_account(storage, emitter, email, name, token)
    }
    
    /// 删除账户
    pub fn delete_account<S: StorageConfig, E: EventEmitter>(
        storage: &S,
        emitter: &E,
        account_id: &str,
    ) -> Result<(), String> {
        let mut index = Self::load_index(storage)?;
        
        // 从索引中移除
        let original_len = index.accounts.len();
        index.accounts.retain(|a| a.id != account_id);
        
        if index.accounts.len() == original_len {
            return Err(format!("账户不存在: {}", account_id));
        }
        
        // 如果删除的是当前账户，切换到其他账户
        if index.current_account_id.as_deref() == Some(account_id) {
            index.current_account_id = index.accounts.first().map(|a| a.id.clone());
        }
        
        Self::save_index(storage, &index)?;
        
        // 删除账户文件
        let account_path = storage.accounts_dir().join(format!("{}.json", account_id));
        if account_path.exists() {
            fs::remove_file(&account_path)
                .map_err(|e| format!("删除账户文件失败: {}", e))?;
        }
        
        emitter.emit("account-deleted", account_id);
        
        Ok(())
    }
    
    /// 批量删除账户
    pub fn delete_accounts<S: StorageConfig, E: EventEmitter>(
        storage: &S,
        emitter: &E,
        account_ids: &[String],
    ) -> Result<(), String> {
        let mut index = Self::load_index(storage)?;
        
        // 批量从索引中移除
        index.accounts.retain(|a| !account_ids.contains(&a.id));
        
        // 如果当前账户被删除，切换到其他账户
        if let Some(current_id) = &index.current_account_id {
            if account_ids.contains(current_id) {
                index.current_account_id = index.accounts.first().map(|a| a.id.clone());
            }
        }
        
        Self::save_index(storage, &index)?;
        
        // 删除账户文件
        for account_id in account_ids {
            let account_path = storage.accounts_dir().join(format!("{}.json", account_id));
            if account_path.exists() {
                let _ = fs::remove_file(&account_path);
            }
        }
        
        emitter.emit("accounts-deleted", account_ids);
        
        Ok(())
    }
    
    /// 切换当前账户
    pub fn switch_account<S: StorageConfig, E: EventEmitter>(
        storage: &S,
        emitter: &E,
        account_id: &str,
    ) -> Result<(), String> {
        let mut index = Self::load_index(storage)?;
        
        // 验证账户存在
        if !index.accounts.iter().any(|a| a.id == account_id) {
            return Err(format!("账户不存在: {}", account_id));
        }
        
        // 更新账户的 last_used
        if let Ok(mut account) = Self::load_account(storage, account_id) {
            account.update_last_used();
            Self::save_account(storage, &account)?;
            
            // 更新索引中的 last_used
            if let Some(summary) = index.accounts.iter_mut().find(|a| a.id == account_id) {
                summary.last_used = account.last_used;
            }
        }
        
        index.current_account_id = Some(account_id.to_string());
        Self::save_index(storage, &index)?;
        
        emitter.emit("account-switched", account_id);
        
        Ok(())
    }
    
    /// 获取当前账户
    pub fn get_current_account<S: StorageConfig>(storage: &S) -> Result<Option<Account>, String> {
        let index = Self::load_index(storage)?;
        
        match index.current_account_id {
            Some(id) => {
                let account = Self::load_account(storage, &id)?;
                Ok(Some(account))
            }
            None => Ok(None),
        }
    }
    
    /// 获取当前账户 ID
    pub fn get_current_account_id<S: StorageConfig>(storage: &S) -> Result<Option<String>, String> {
        let index = Self::load_index(storage)?;
        Ok(index.current_account_id)
    }
    
    /// 更新账户配额
    pub fn update_account_quota<S: StorageConfig>(
        storage: &S,
        account_id: &str,
        quota: QuotaData,
    ) -> Result<(), String> {
        let mut account = Self::load_account(storage, account_id)?;
        account.update_quota(quota);
        Self::save_account(storage, &account)?;
        Ok(())
    }
    
    /// 导出所有账户的 refresh_token
    pub fn export_accounts<S: StorageConfig>(storage: &S) -> Result<Vec<(String, String)>, String> {
        let accounts = Self::list_accounts(storage)?;
        
        let tokens: Vec<(String, String)> = accounts
            .into_iter()
            .map(|a| (a.email, a.token.refresh_token))
            .collect();
        
        Ok(tokens)
    }
}
