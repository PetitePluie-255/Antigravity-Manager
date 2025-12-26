# Web 端功能适配状态

> 最后更新: 2025-12-25

## ✅ 已适配功能

### 账户管理

| 功能         | Tauri 命令            | Web API                      | 状态 |
| ------------ | --------------------- | ---------------------------- | ---- |
| 列出账号     | `list_accounts`       | `GET /api/accounts`          | ✅   |
| 添加账号     | `add_account`         | `POST /api/accounts`         | ✅   |
| 删除账号     | `delete_account`      | `DELETE /api/accounts/:id`   | ✅   |
| 批量删除     | `delete_accounts`     | `DELETE /api/accounts/batch` | ✅   |
| 切换账号     | `switch_account`      | `PUT /api/accounts/current`  | ✅   |
| 获取当前账号 | `get_current_account` | `GET /api/accounts/current`  | ✅   |
| 导出账号     | -                     | `GET /api/accounts/export`   | ✅   |

### 配额管理

| 功能         | Tauri 命令            | Web API                            | 状态 |
| ------------ | --------------------- | ---------------------------------- | ---- |
| 刷新单个配额 | `fetch_account_quota` | `POST /api/accounts/:id/quota`     | ✅   |
| 批量刷新配额 | `refresh_all_quotas`  | `POST /api/accounts/quota/refresh` | ✅   |

### 配置管理

| 功能     | Tauri 命令    | Web API           | 状态 |
| -------- | ------------- | ----------------- | ---- |
| 加载配置 | `load_config` | `GET /api/config` | ✅   |
| 保存配置 | `save_config` | `PUT /api/config` | ✅   |

### 代理服务

| 功能         | Tauri 命令             | Web API                        | 状态 |
| ------------ | ---------------------- | ------------------------------ | ---- |
| 启动代理     | `start_proxy`          | `POST /api/proxy/start`        | ✅   |
| 停止代理     | `stop_proxy`           | `POST /api/proxy/stop`         | ✅   |
| 代理状态     | `get_proxy_status`     | `GET /api/proxy/status`        | ✅   |
| 生成 API Key | `generate_api_key`     | `POST /api/proxy/key/generate` | ✅   |
| 更新模型映射 | `update_model_mapping` | `PUT /api/proxy/mapping`       | ✅   |

### OAuth 认证

| 功能           | Tauri 命令           | Web API                   | 状态 |
| -------------- | -------------------- | ------------------------- | ---- |
| 开始 OAuth     | `start_oauth_login`  | `POST /api/oauth/start`   | ✅   |
| 取消 OAuth     | `cancel_oauth_login` | 前端处理                  | ✅   |
| OAuth 回调     | -                    | `GET /api/oauth/callback` | ✅   |
| OAuth 状态     | -                    | `GET /api/oauth/status`   | ✅   |
| Token 自动刷新 | (内嵌)               | `core::services::oauth`   | ✅   |

### 文件导入

| 功能      | Tauri 命令 | Web API                 | 状态 |
| --------- | ---------- | ----------------------- | ---- |
| JSON 导入 | -          | `POST /api/import/json` | ✅   |
| 文件导入  | -          | `POST /api/import/file` | ✅   |

**支持格式:**

- JSON: `[{email, refresh_token}]` 或 `{accounts: [...]}`

### 数据库导入

| 功能            | Tauri 命令 | Web API                     | 状态 |
| --------------- | ---------- | --------------------------- | ---- |
| PostgreSQL 导入 | -          | `POST /api/import/database` | ✅   |
| SQLite 导入     | -          | `POST /api/import/database` | ✅   |

**API 用法:**

```json
POST /api/import/database
{
  "url": "postgresql://user:pass@host:5432/db",
  "table": "accounts",
  "email_column": "email",
  "token_column": "refresh_token"
}
```

---

## ❌ 未适配功能

### 数据库同步

| 功能            | Tauri 命令             | 适配方案         | 优先级 |
| --------------- | ---------------------- | ---------------- | ------ |
| 同步 IDE 数据库 | `sync_account_from_db` | WebSocket 或轮询 | P3     |

### 代理日志

| 功能           | Web API                                  | 状态      |
| -------------- | ---------------------------------------- | --------- |
| 获取代理日志   | `GET /api/proxy/logs?limit=100&offset=0` | ✅        |
| 清除代理日志   | `POST /api/proxy/logs/clear`             | ✅        |
| 日志记录中间件 | (代理 handler 调用 log_store.record)     | ⏳ 待实现 |

### 系统功能（需评估）

| 功能                  | Tauri 命令             | 适配方案                  | 优先级 |
| --------------------- | ---------------------- | ------------------------- | ------ |
| 保存文件              | `save_text_file`       | 不适用 (浏览器下载)       | -      |
| 清除日志              | `clear_log_cache`      | 服务端日志管理            | P4     |
| 打开数据目录          | `open_data_folder`     | 不适用 (系统功能)         | -      |
| 获取数据路径          | `get_data_dir_path`    | `GET /api/system/info`    | P4     |
| 显示窗口              | `show_main_window`     | 不适用 (桌面功能)         | -      |
| 获取 Antigravity 路径 | `get_antigravity_path` | 不适用                    | -      |
| 检查更新              | `check_for_updates`    | `GET /api/system/updates` | P4     |

### 🐛 已知问题 (待修复)

| 问题           | 描述                                                  | 状态      |
| -------------- | ----------------------------------------------------- | --------- |
| 保存配置 422   | `PUT /api/config` 返回 `missing field language` 错误  | ⏳ 待修复 |
| 日志记录中间件 | 代理 handler 未调用 `log_store.record()` 记录请求日志 | ⏳ 待实现 |

---

## 📊 统计

| 类别     | 已适配 | 未适配 | 适配率    |
| -------- | ------ | ------ | --------- |
| 核心功能 | 20     | 0      | 100%      |
| 导入功能 | 4      | 1      | 80%       |
| 日志功能 | 2      | 0      | 100%      |
| 系统功能 | 0      | 7      | 0%        |
| **总计** | **26** | **8**  | **76.5%** |

---

## 🗺️ 后续规划

### Phase 1: 文件导入 (P1) ✅ 已完成

- [x] 添加文件上传 API
- [x] 支持 JSON/CSV 格式
- [x] 前端适配

### Phase 2: PostgreSQL 集成 (P2) ✅ 已完成

- [x] 添加 sqlx 依赖
- [x] 实现 PostgreSQL 导入服务
- [x] 实现 SQLite 导入服务
- [x] 添加 API 端点

### Phase 3: 代理日志 (P3) ✅ 已完成

- [x] 创建 `LogStore` 日志存储模块
- [x] 实现日志查询 API (`GET /api/proxy/logs`)
- [x] 实现日志清除 API (`POST /api/proxy/logs/clear`)
- [x] 创建前端日志页面 (`Logs.tsx`)

### Phase 4: 系统信息 (P4)

- [ ] 添加系统信息 API
- [ ] 添加版本检查 API

### Phase 5: 配额智能切换 (P2) ✅ 已完成

> 调用模型时检查配额，不足时自动切换到配额充足的账户

- [x] 在 `TokenManager` 中缓存配额数据
- [x] 实现配额感知的账号选择逻辑 (`get_token_with_quota`)
- [x] 配额耗尽时返回友好提示
- [x] 定时后台刷新配额缓存 (`start_quota_refresh_task`)

**已实现功能:**

- 配额阈值: 10%
- 按配额排序，优先使用高配额账号
- 支持 gemini/claude 模型类型识别

### Phase 6: 架构统一重构 (P3)

> 统一 `modules/` 和 `core/` 两套架构，重构为业务模块模式

**当前架构 (混合模式):**

```
src-tauri/src/
├── core/           # 技术分层 (models/services/storage)
├── modules/        # 业务模块 (Tauri 专用)
├── proxy/          # 按功能分层
└── web/            # 技术分层
```

**目标架构 (业务模块模式):**

```
src-tauri/src/
├── account/        # 账户管理模块
│   ├── models.rs
│   ├── service.rs
│   └── handlers.rs
├── oauth/          # OAuth 认证模块
│   ├── service.rs
│   └── handlers.rs
├── config/         # 配置管理模块
│   ├── models.rs
│   └── service.rs
├── quota/          # 配额管理模块
│   ├── models.rs
│   └── service.rs
├── proxy/          # 代理服务模块
│   ├── handlers/
│   ├── mappers/
│   └── token_manager.rs
├── web/            # Web 服务器
│   ├── routes.rs
│   └── server.rs
└── shared/         # 共享组件
    ├── storage/
    └── traits/
```

**重构步骤:**

- [ ] 创建业务模块目录结构
- [ ] 将 `core/models/` 拆分到各业务模块
- [ ] 将 `core/services/` 拆分到各业务模块
- [ ] 将 `modules/` 迁移到对应业务模块
- [ ] 更新 `commands/` 和 `web/handlers.rs` 引用
- [ ] 删除旧目录 (`models/`, `modules/`, `core/`)

---

## ⚠️ 必要功能 (后续规划)

| 功能           | 必要性  | 方案                  | 状态    |
| -------------- | ------- | --------------------- | ------- |
| OAuth 认证     | ⭐ 必备 | GitHub/Google OAuth   | 📅 后续 |
| Token 加密存储 | ⭐ 必备 | AES 加密              | 📅 后续 |
| 服务自动重启   | ⭐ 建议 | Docker restart policy | 📅 后续 |
| 使用统计限额   | ⭐ 建议 | 可视化图表            | 📅 后续 |

---

## 🔮 其他优化方向

### 性能优化

| 方向     | 说明                 | 优先级 |
| -------- | -------------------- | ------ |
| 连接池   | 复用 HTTP 客户端连接 | P2     |
| 请求缓存 | 缓存配额查询结果     | P2     |
| 并发限制 | 限制同时请求数       | P3     |
| 响应压缩 | gzip/brotli 压缩     | P4     |

### 安全增强 (公网部署必备)

| 方向           | 说明                       | 优先级 |
| -------------- | -------------------------- | ------ |
| API Key 认证   | 环境变量配置，保护所有 API | **P0** |
| HTTPS 支持     | TLS 证书配置               | **P1** |
| Token 加密存储 | AES 加密 refresh_token     | **P1** |
| 速率限制       | 防止 API 滥用              | P2     |
| CORS 白名单    | 限制跨域来源               | P2     |
| IP 白名单      | 可选的 IP 访问限制         | P3     |

**API Key 认证方案:**

```bash
# 服务器启动时配置
export ANTIGRAVITY_API_KEY="your-secret-key"
./antigravity-server --port 3000

# 客户端请求时携带
curl -H "Authorization: Bearer your-secret-key" http://localhost:3000/api/accounts
```

**Token 加密方案:**

```bash
# 环境变量设置加密密钥
export ANTIGRAVITY_ENCRYPT_KEY="your-encrypt-key"

# 账号文件自动加密存储
# ~/.antigravity_tools/accounts.json.enc
```

### 可观测性

| 方向       | 说明                | 优先级 |
| ---------- | ------------------- | ------ |
| 结构化日志 | tracing + JSON 格式 | P2     |
| 指标导出   | Prometheus metrics  | P3     |
| 健康检查   | 详细健康状态        | P4     |

### 质量保障

| 方向     | 说明             | 优先级 |
| -------- | ---------------- | ------ |
| 单元测试 | 核心服务测试覆盖 | P2     |
| 集成测试 | API 端到端测试   | P3     |
| CI/CD    | GitHub Actions   | P2     |

### 用户体验

| 方向      | 说明               | 优先级 |
| --------- | ------------------ | ------ |
| WebSocket | 实时状态推送       | P3     |
| 多语言    | API 错误信息国际化 | P4     |
