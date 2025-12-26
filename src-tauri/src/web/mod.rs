//! Web 服务器模块
//! 提供独立运行的 HTTP API 服务

pub mod routes;
pub mod handlers;
pub mod server;

pub use server::WebServer;
