# TmuxGo v2 — Windows 桌面客户端开发计划

## 产品定位

**TmuxGo v2** 是一款 Windows 桌面端 SSH 客户端，核心能力是通过 SSH 连接远程 Linux 服务器，管理 tmux 会话。定位类似 Tabby/Warp，但专注于 tmux 管理场景。

```
┌─────────────────────────────────────────────────────┐
│  TmuxGo v2 (Tauri + React, Windows 桌面应用)          │
│                                                       │
│  ┌──────────┐  ┌──────────┐  ┌────────────────────┐  │
│  │ 连接管理  │  │ 会话面板  │  │ 终端/文件/编辑器    │  │
│  └────┬─────┘  └────┬─────┘  └────────┬───────────┘  │
│       │              │                 │               │
│  ┌────┴──────────────┴─────────────────┴───────────┐  │
│  │              SSH 连接层 (ssh2)                    │  │
│  └──────────────────────┬──────────────────────────┘  │
└─────────────────────────┼─────────────────────────────┘
                          │ SSH
              ┌───────────┼───────────┐
              ▼           ▼           ▼
          Server A    Server B    Server C
          (tmux)      (tmux)      (tmux)
```

## 与 v1 的架构对比

| 维度 | v1 (部署端) | v2 (使用端) |
|------|------------|------------|
| 运行位置 | 服务器 | 用户 Windows 电脑 |
| 连接方向 | Agent → Gateway | App → SSH → 服务器 |
| 用户入口 | 浏览器访问 web | 桌面应用 |
| 命令执行 | 本地 node-pty | SSH remote exec |
| 文件操作 | 本地 fs | SFTP |
| 终端流 | WebSocket ↔ PTY | SSH shell channel |
| 多用户 | 支持 | 单用户 |

## 可复用资产（来自 v1）

| 模块 | 复用度 | 说明 |
|------|--------|------|
| `components/*` | **90%** | TerminalPane、PaneGrid、SessionPanel、FilePanel、EditorWorkbench 等 UI 组件几乎全复用 |
| `stores/useConsoleStore.ts` | **85%** | 状态管理复用，需扩展连接相关状态 |
| `hooks/useApi.ts` | **60%** | React Query hooks 复用，但底层 API 调用需替换 |
| `hooks/useWebSocket.ts` | **0%** | 需重写为 SSH channel 版本 |
| `lib/api.ts` | **0%** | 需重写为 SSH 命令执行层 |
| `lib/runtime-endpoints.ts` | **0%** | 不再需要 gateway 地址 |
| `i18n/` | **100%** | 直接复用 |
| `styles/` | **100%** | 直接复用 |
| Gateway 全部代码 | **0%** | 不需要 Gateway 层 |
| Agent 全部代码 | **0%** | 不需要 Agent |

## 技术栈选型

| 层 | 技术 | 理由 |
|----|------|------|
| 桌面壳 | **Tauri 2.0** | 比 Electron 轻 10x，Rust 后端适合做 SSH |
| 前端 | **React 18 + Next.js**（去 SSR） | 复用 v1 组件 |
| 终端 | **xterm.js** | 复用 v1 |
| SSH | **ssh2** (npm) + Tauri Rust 侧 `thrussh` | 双层：简单命令用 ssh2，高性能流用 Rust |
| 状态管理 | **Zustand** | 复用 v1 |
| 数据获取 | **TanStack Query** | 复用 v1 |
| 构建 | **Vite**（Tauri 默认） | 替换 Next.js 构建 |

## 项目结构

```
TmuxGo_v2/
├── src-tauri/                    # Tauri Rust 后端
│   ├── src/
│   │   ├── main.rs               # Tauri 入口
│   │   ├── ssh/
│   │   │   ├── mod.rs            # SSH 连接管理
│   │   │   ├── connection.rs     # 单个 SSH 连接封装
│   │   │   ├── pool.rs           # 连接池
│   │   │   ├── shell.rs          # Shell channel (终端流)
│   │   │   ├── exec.rs           # 远程命令执行
│   │   │   └── sftp.rs           # SFTP 文件操作
│   │   ├── commands/
│   │   │   ├── hosts.rs          # 主机管理 (增删改查)
│   │   │   ├── sessions.rs       # tmux 会话操作
│   │   │   ├── windows.rs        # tmux 窗口操作
│   │   │   ├── panes.rs          # tmux 面板操作
│   │   │   ├── files.rs          # 文件操作 (via SFTP)
│   │   │   └── system.rs         # 远程系统信息
│   │   └── store/
│   │       ├── connections.rs    # 连接配置持久化
│   │       └── preferences.rs   # 用户偏好持久化
│   ├── Cargo.toml
│   └── tauri.conf.json
│
├── src/                          # React 前端 (复用/改造 v1)
│   ├── app/
│   │   ├── page.tsx              # 主页面
│   │   └── layout.tsx
│   ├── components/
│   │   ├── ConnectionManager.tsx  # [新增] 连接管理界面
│   │   ├── HostList.tsx          # [新增] 主机列表
│   │   ├── SshKeyManager.tsx     # [新增] SSH 密钥管理
│   │   ├── ConsoleLayout.tsx     # [改造] 适配 SSH 连接
│   │   ├── TerminalPane.tsx      # [改造] 数据源改为 SSH channel
│   │   ├── SessionPanel.tsx      # [复用] 小改
│   │   ├── PaneGrid.tsx          # [复用]
│   │   ├── FilePanel.tsx         # [改造] 底层改为 SFTP
│   │   ├── EditorWorkbench.tsx   # [改造] 底层改为 SFTP
│   │   └── ...                   # 其余组件复用
│   ├── hooks/
│   │   ├── useSshConnection.ts   # [新增] SSH 连接 hook
│   │   ├── useRemoteExec.ts      # [新增] 远程命令执行 hook
│   │   ├── useSftp.ts            # [新增] SFTP hook
│   │   ├── useApi.ts             # [改造] 底层调用替换
│   │   └── ...
│   ├── stores/
│   │   ├── useConsoleStore.ts    # [改造] 扩展连接状态
│   │   └── useConnectionStore.ts # [新增] 连接配置 store
│   ├── lib/
│   │   ├── tauri-commands.ts     # [新增] Tauri invoke 封装
│   │   ├── tmux-remote.ts        # [新增] 远程 tmux 命令封装
│   │   └── ...
│   └── types/
│       ├── connection.ts         # [新增] 连接相关类型
│       └── ...
│
├── package.json
├── vite.config.ts
├── tsconfig.json
└── DEVELOPMENT_PLAN.md
```

## 开发阶段

### Phase 1: 基础框架搭建（Week 1-2）

**目标：** Tauri 项目初始化 + SSH 连接能通

- [ ] 初始化 Tauri 2.0 项目（Windows target）
- [ ] 迁移 v1 前端代码到 Tauri 的 `src/` 目录
- [ ] 替换 Next.js 构建为 Vite
- [ ] 实现 Rust 侧 SSH 连接管理（`ssh2-rs` crate）
  - [ ] 连接建立（密码 / SSH key）
  - [ ] 连接池（多主机并发）
  - [ ] 心跳 + 断线检测
  - [ ] 自动重连
- [ ] 实现 Tauri Commands 桥接（前端 invoke → Rust SSH）
- [ ] 基础连接管理 UI（添加/编辑/删除主机）

**交付物：** 能通过 SSH 连上远程服务器，看到连接状态

### Phase 2: 终端核心（Week 3-4）

**目标：** 能在远程服务器上操作终端

- [ ] Rust 侧 Shell Channel 实现
  - [ ] SSH shell channel 开启
  - [ ] 输入/输出双向流
  - [ ] resize 支持
- [ ] 前端 TerminalPane 改造
  - [ ] xterm.onData → Tauri invoke → SSH channel stdin
  - [ ] SSH channel stdout → xterm.write
  - [ ] resize 事件同步
- [ ] 实现 `useSshConnection` hook
- [ ] 实现 `useRemoteExec` hook（远程命令执行）

**交付物：** 能在远程服务器上敲命令，基本终端可用

### Phase 3: tmux 管理（Week 5-7）

**目标：** 完整的 tmux 会话/窗口/面板管理

- [ ] 远程 tmux 命令封装（`tmux-remote.ts`）
  - [ ] 通过 SSH exec 执行 tmux 命令
  - [ ] 复用 v1 的 tmux 命令逻辑（sessions/windows/panes 路由）
- [ ] tmux attach 方案实现
  - [ ] 方案 A: SSH shell + `tmux attach`（推荐，复用 v1 流模式）
  - [ ] 方案 B: tmux 控制模式（`tmux -C`，备选）
- [ ] 前端 SessionPanel / WindowTabs / PaneGrid 适配
- [ ] 会话模板功能迁移
- [ ] 多主机会话切换

**交付物：** 完整的 tmux 管理体验，等同 v1

### Phase 4: 文件管理（Week 8-9）

**目标：** 远程文件浏览、编辑、搜索

- [ ] Rust 侧 SFTP 封装
  - [ ] 目录列表
  - [ ] 文件读写
  - [ ] 文件上传/下载
  - [ ] 文件搜索（远程 `find` + `grep`/`rg`）
- [ ] 前端 FilePanel / EditorWorkbench 适配
  - [ ] 底层 API 从 HTTP fetch → Tauri invoke → SFTP
  - [ ] 文件预览 / 编辑 / 保存
- [ ] 本地 ↔ 远程文件拖拽传输

**交付物：** 完整的远程文件管理体验

### Phase 5: 主机管理与连接配置（Week 10）

**目标：** 完善的主机管理体验

- [ ] 主机配置持久化（本地 JSON/SQLite）
  - [ ] 主机名、地址、端口
  - [ ] 认证方式（密码 / SSH key / agent forwarding）
  - [ ] 分组 / 标签
- [ ] SSH 密钥管理 UI
  - [ ] 导入现有密钥
  - [ ] 生成新密钥对
  - [ ] 密钥密码管理
- [ ] 连接历史 + 快速连接
- [ ] 批量操作（批量执行命令）

**交付物：** 完善的多主机管理体验

### Phase 6: 体验打磨（Week 11-12）

**目标：** 生产级体验

- [ ] 系统托盘 + 全局快捷键唤起
- [ ] Windows 通知集成（远程命令完成通知）
- [ ] 剪贴板双向同步
- [ ] 性能优化（大输出缓冲、延迟渲染）
- [ ] 错误处理与重连策略完善
- [ ] 主题 / 字体 / 快捷键自定义
- [ ] 打包分发（Windows installer / portable）

**交付物：** 可分发的 v2.0 正式版

## 关键技术决策

### 1. 终端流方案

**推荐：SSH shell channel + tmux attach**

```
xterm.js ←→ Tauri IPC ←→ Rust SSH shell ←→ tmux attach
```

- 与 v1 的 WebSocket ↔ PTY 模式对齐，前端改动最小
- tmux 负责多路复用，客户端只做流转发
- resize 通过 SSH channel 的 `set_window_size` 实现

### 2. tmux 命令执行

```
前端 → Tauri invoke("exec_remote", {hostId, cmd}) → Rust ssh.exec() → 结果返回
```

- 复用 v1 的 tmux 命令参数（list-sessions, new-session 等）
- 每次命令开新 channel 执行，不复用 shell channel

### 3. 文件操作

```
前端 → Tauri invoke("sftp_*", {hostId, path}) → Rust SFTP → 结果返回
```

- 替代 v1 的 HTTP REST 文件 API
- 上传/下载通过 SFTP stream 实现

### 4. 数据持久化

| 数据 | 存储位置 | 方式 |
|------|----------|------|
| 主机配置 | `%APPDATA%/tmuxgo-v2/hosts.json` | JSON |
| SSH 密钥 | `%APPDATA%/tmuxgo-v2/keys/` | 文件 |
| 用户偏好 | `%APPDATA%/tmuxgo-v2/preferences.json` | JSON |
| 连接历史 | `%APPDATA%/tmuxgo-v2/history.json` | JSON |

## 风险与应对

| 风险 | 影响 | 应对 |
|------|------|------|
| ssh2 性能不如原生 SSH | 终端卡顿 | Rust 侧用 `thrussh` 处理高频流 |
| tmux attach 断线恢复 | 用户体验 | 自动重连 + tmux session 保持 |
| Windows SSH key 格式兼容 | 连接失败 | 支持 OpenSSH / PuTTY / PEM 格式 |
| 大文件传输 | 内存/速度 | SFTP streaming + 进度回调 |
| Tauri IPC 开销 | 高频终端输出 | 二进制传输 + 批量 flush |

## 里程碑

| 里程碑 | 时间 | 验收标准 |
|--------|------|----------|
| M1: 能连上 | Week 2 | SSH 连接成功，显示远程 shell |
| M2: 能用终端 | Week 4 | 完整终端体验，支持 tmux attach |
| M3: 能管 tmux | Week 7 | 会话/窗口/面板 CRUD 完整 |
| M4: 能管文件 | Week 9 | 文件浏览/编辑/搜索/传输 |
| M5: 能管理主机 | Week 10 | 多主机配置/分组/快速连接 |
| M6: 能分发 | Week 12 | Windows installer，生产可用 |
