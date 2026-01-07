export interface UpstreamProxyConfig {
  enabled: boolean;
  url: string;
}

export interface ZaiMcpConfig {
  enabled?: boolean;
  web_search_enabled?: boolean;
  web_reader_enabled?: boolean;
  vision_enabled?: boolean;
}

export interface ZaiConfig {
  enabled?: boolean;
  api_key?: string;
  dispatch_mode?: string; // 'off' | 'exclusive' | 'pooled' | 'fallback'
  mcp?: ZaiMcpConfig;
}

export interface SchedulingConfig {
  mode?: string; // 'cache_first' | 'balance' | 'performance_first'
  max_wait_seconds?: number;
}

export interface ProxyConfig {
  enabled: boolean;
  allow_lan_access?: boolean;
  port: number;
  api_key: string;
  auto_start: boolean;
  anthropic_mapping?: Record<string, string>;
  openai_mapping?: Record<string, string>;
  custom_mapping?: Record<string, string>;
  request_timeout: number;
  upstream_proxy: UpstreamProxyConfig;
  zai?: ZaiConfig;
  scheduling?: SchedulingConfig;
  auth_mode?: string; // 'off' | 'strict' | 'all_except_health' | 'auto'
}

export interface AppConfig {
  language: string;
  theme: string;
  auto_refresh: boolean;
  refresh_interval: number;
  auto_sync: boolean;
  sync_interval: number;
  default_export_path?: string;
  antigravity_executable?: string; // [NEW] 手动指定的反重力程序路径
  auto_launch?: boolean; // 开机自动启动
  accounts_page_size?: number; // 账号列表每页显示数量,默认 0 表示自动计算
  proxy: ProxyConfig;
}
