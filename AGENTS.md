# TaskFlow project rules

## 定位

TaskFlow（小光任务）是一个本地优先的 Windows 任务管理器，支持可选的 Supabase 邮箱登录和多设备同步。

## 技术栈

- Vue 3 + Vite：主窗口、设置页和桌面组件界面
- Tauri 2 + Rust：Windows 窗口、托盘、本地持久化和系统能力
- SortableJS：任务拖拽排序
- Supabase JS：可选认证、工作区和实时同步

## 常用命令

- `npm.cmd run dev`：启动 Tauri 开发模式
- `npm.cmd run dev:web`：仅启动 Vite 前端
- `npm.cmd run build:portable`：构建并生成 `release/小光任务.exe`
- `npm.cmd run build:installer`：生成 NSIS 安装包
- `npm.cmd run verify`：前端构建、规则测试和 Rust 检查/测试

## 目录约定

- `src/`：Vue 界面、运行时适配和同步编排
- `src-tauri/`：Rust 命令、本地数据与窗口/托盘能力
- `supabase/`：云端表结构、策略和迁移
- `docs/`：架构、开发、同步和安全说明
- `release/`、`dist/`、`src-tauri/target/`：可重新生成的构建产物，不提交

## 当前状态

- 桌面端任务拖拽使用 Tauri 兼容的 fallback；临时预览副本通过 `task-fallback` 隐藏
- 登录后自动准备个人工作区并启动 Realtime + 轮询同步；未登录时保持本地模式
- 用户数据位于 `%APPDATA%\\com.personal.taskflow-lite`，修改前须保留本地数据和备份
- 公开发布前不得提交 `.env` 或 Supabase `service_role` key

## 变更要求

修改后至少运行相关的 `npm.cmd run test:unit` / `npm.cmd run build:web`；桌面发布还需验证 `release/小光任务.exe` 能启动。不要把 `release/`、`dist/` 或本地数据当作源码清理。
