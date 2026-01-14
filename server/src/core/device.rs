use crate::core::models::DeviceProfile;
use rand::{distributions::Alphanumeric, Rng};
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;

const GLOBAL_BASELINE: &str = "device_original.json";

/// 获取数据目录（用于存放全局指纹等）
pub fn get_data_dir() -> Result<PathBuf, String> {
    let home = dirs::home_dir().ok_or_else(|| "无法获取用户目录".to_string())?;
    let dir = home.join(".gemini-antigravity");
    if !dir.exists() {
        fs::create_dir_all(&dir).map_err(|e| format!("创建数据目录失败: {}", e))?;
    }
    Ok(dir)
}

/// 全局原始指纹（所有账号共享）的存取
pub fn load_global_original() -> Option<DeviceProfile> {
    if let Ok(dir) = get_data_dir() {
        let path = dir.join(GLOBAL_BASELINE);
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(profile) = serde_json::from_str::<DeviceProfile>(&content) {
                    return Some(profile);
                }
            }
        }
    }
    None
}

pub fn save_global_original(profile: &DeviceProfile) -> Result<(), String> {
    let dir = get_data_dir()?;
    let path = dir.join(GLOBAL_BASELINE);
    if path.exists() {
        return Ok(()); // 已存在则不覆盖
    }
    let content =
        serde_json::to_string_pretty(profile).map_err(|e| format!("序列化原始指纹失败: {}", e))?;
    fs::write(&path, content).map_err(|e| format!("写入原始指纹失败: {}", e))
}

/// 生成一组新的设备指纹（符合 Cursor/VSCode 风格）
pub fn generate_profile() -> DeviceProfile {
    DeviceProfile {
        machine_id: format!("auth0|user_{}", random_hex(32)),
        mac_machine_id: new_standard_machine_id(),
        dev_device_id: Uuid::new_v4().to_string(),
        sqm_id: format!("{{{}}}", Uuid::new_v4().to_string().to_uppercase()),
    }
}

fn random_hex(length: usize) -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(length)
        .map(char::from)
        .collect::<String>()
        .to_lowercase()
}

fn new_standard_machine_id() -> String {
    // xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx (y in 8..b)
    let mut rng = rand::thread_rng();
    let mut id = String::with_capacity(36);
    for ch in "xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx".chars() {
        if ch == '-' || ch == '4' {
            id.push(ch);
        } else if ch == 'x' {
            id.push_str(&format!("{:x}", rng.gen_range(0..16)));
        } else if ch == 'y' {
            id.push_str(&format!("{:x}", rng.gen_range(8..12)));
        }
    }
    id
}
