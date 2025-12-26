// Handlers 模块 - API 端点处理器
// 核心端点处理器模块

pub mod claude;
pub mod gemini;
pub mod openai;

// 使用 glob 导出（允许歧义，在使用时显式指定模块）
#[allow(ambiguous_glob_reexports)]
pub use claude::*;
#[allow(ambiguous_glob_reexports)]
pub use gemini::*;
#[allow(ambiguous_glob_reexports)]
pub use openai::*;
