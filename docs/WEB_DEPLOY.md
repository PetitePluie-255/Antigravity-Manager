# Antigravity Manager Web 部署指南

## 快速开始

### 方式一：Docker 部署（推荐）

```bash
# 克隆代码
git clone https://github.com/your-repo/Antigravity-Manager.git
cd Antigravity-Manager

# 构建镜像
docker build -t antigravity-manager:latest .

# 运行容器
docker run -d \
  --name antigravity \
  -p 3000:3000 \
  -v /path/to/data:/data \
  --restart unless-stopped \
  antigravity-manager:latest
```

### 方式二：Docker Compose

```bash
# 使用 docker-compose
docker-compose up -d

# 查看日志
docker-compose logs -f
```

---

## 构建细节

### 1. 前端构建

```bash
# 安装依赖
pnpm install

# 构建
pnpm run build
# 输出: dist/
```

### 2. 后端构建

```bash
cd src-tauri

# 安装 Rust (如需)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 构建 Web Server
cargo build --release --bin antigravity-server \
  --no-default-features --features web-server

# 输出: target/release/antigravity-server
```

### 3. 手动部署

```bash
# 复制二进制
cp src-tauri/target/release/antigravity-server /opt/antigravity/

# 复制前端文件
cp -r dist /opt/antigravity/

# 创建数据目录
mkdir -p /opt/antigravity/data

# 运行
cd /opt/antigravity
./antigravity-server --port 3000 --data-dir ./data
```

---

## 环境变量

| 变量          | 默认值 | 说明         |
| ------------- | ------ | ------------ |
| `PORT`        | 3000   | 服务端口     |
| `DATA_DIR`    | ./data | 数据目录     |
| `STATIC_PATH` | ./dist | 前端文件路径 |

---

## Systemd 服务

```ini
# /etc/systemd/system/antigravity.service
[Unit]
Description=Antigravity Manager
After=network.target

[Service]
Type=simple
User=antigravity
WorkingDirectory=/opt/antigravity
ExecStart=/opt/antigravity/antigravity-server --port 3000 --data-dir /opt/antigravity/data
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
```

```bash
# 启用服务
sudo systemctl enable antigravity
sudo systemctl start antigravity
```

---

## API 端点

| 端点                        | 说明       |
| --------------------------- | ---------- |
| `GET /healthz`              | 健康检查   |
| `GET /api/accounts`         | 账户列表   |
| `GET /api/proxy/logs`       | 代理日志   |
| `POST /api/import/database` | 数据库导入 |

---

## 注意事项

1. **数据持久化**: 确保 `/data` 目录挂载到宿主机
2. **端口映射**: 默认端口 3000
3. **健康检查**: `/healthz` 返回 200 表示正常
