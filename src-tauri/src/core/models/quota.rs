//! 配额数据模型

use serde::{Deserialize, Serialize};

/// 模型配额信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelQuota {
    pub name: String,
    pub percentage: i32,  // 剩余百分比 0-100
    pub reset_time: String,
}

/// 配额数据结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuotaData {
    pub models: Vec<ModelQuota>,
    pub last_updated: i64,
    #[serde(default)]
    pub is_forbidden: bool,
}

impl QuotaData {
    pub fn new() -> Self {
        Self {
            models: Vec::new(),
            last_updated: chrono::Utc::now().timestamp(),
            is_forbidden: false,
        }
    }

    pub fn add_model(&mut self, name: String, percentage: i32, reset_time: String) {
        self.models.push(ModelQuota {
            name,
            percentage,
            reset_time,
        });
    }
    
    /// 获取平均配额百分比
    pub fn average_percentage(&self) -> i32 {
        if self.models.is_empty() {
            return 0;
        }
        let sum: i32 = self.models.iter().map(|m| m.percentage).sum();
        sum / self.models.len() as i32
    }
}

impl Default for QuotaData {
    fn default() -> Self {
        Self::new()
    }
}
