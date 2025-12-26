//! Antigravity Web 服务器入口
//! 
//! 独立运行的 HTTP API 服务，提供与 Tauri 客户端相同的功能
//! 
//! 用法:
//!   antigravity-server [OPTIONS]
//! 
//! Options:
//!   -p, --port <PORT>     监听端口 (默认: 3000)
//!   -d, --data-dir <DIR>  数据目录 (默认: ~/.antigravity_tools/)
//!   -h, --help            显示帮助信息

use std::env;
use std::path::PathBuf;

#[tokio::main]
async fn main() {
    // 初始化日志
    tracing_subscriber::fmt::init();
    
    // 解析命令行参数
    let args: Vec<String> = env::args().collect();
    let mut port: u16 = 3000;
    let mut data_dir: Option<PathBuf> = None;
    
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-p" | "--port" => {
                if i + 1 < args.len() {
                    port = args[i + 1].parse().expect("无效的端口号");
                    i += 2;
                } else {
                    eprintln!("错误: --port 需要一个参数");
                    std::process::exit(1);
                }
            }
            "-d" | "--data-dir" => {
                if i + 1 < args.len() {
                    data_dir = Some(PathBuf::from(&args[i + 1]));
                    i += 2;
                } else {
                    eprintln!("错误: --data-dir 需要一个参数");
                    std::process::exit(1);
                }
            }
            "-h" | "--help" => {
                println!("Antigravity Web 服务器");
                println!();
                println!("用法: antigravity-server [OPTIONS]");
                println!();
                println!("Options:");
                println!("  -p, --port <PORT>     监听端口 (默认: 3000)");
                println!("  -d, --data-dir <DIR>  数据目录 (默认: ~/.antigravity_tools/)");
                println!("  -h, --help            显示帮助信息");
                std::process::exit(0);
            }
            _ => {
                eprintln!("未知参数: {}", args[i]);
                std::process::exit(1);
            }
        }
    }
    
    // 创建并启动服务器
    let server = if let Some(dir) = data_dir {
        antigravity_tools_lib::web::WebServer::with_data_dir(port, dir)
    } else {
        antigravity_tools_lib::web::WebServer::new(port)
    };
    
    match server {
        Ok(s) => {
            if let Err(e) = s.run().await {
                eprintln!("服务器错误: {}", e);
                std::process::exit(1);
            }
        }
        Err(e) => {
            eprintln!("初始化失败: {}", e);
            std::process::exit(1);
        }
    }
}
