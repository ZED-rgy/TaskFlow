# TaskFlow Lite 迁移说明

## 目标

TaskFlow Lite 使用 Tauri 替代 Electron，目标是把空闲状态内存从 Electron 版约 480-530 MB 降到 80-150 MB 区间。

## 当前状态

已完成：

- 保留现有 Electron 版，不破坏当前可用 release 流程。
- 新增共享数据核心：`src/shared/taskflow-core.cjs`。
- Electron `electron/store.js` 已改为文件适配层，任务/项目业务规则调用共享核心。
- 新增前端运行时适配层：`src/runtime/api.js`。
- `App.vue` 已从直接调用 `window.api` 改为调用统一 `api`。
- 新增 Tauri 工程骨架：`src-tauri/`。
- 新增脚本：
  - `npm.cmd run lite:dev`
  - `npm.cmd run lite:exe`
  - `npm.cmd run lite:build`
  - `npm.cmd run lite:check`
- 已安装并接入 Rust/Cargo 工具链。
- `npm.cmd run lite:check` 已通过。
- `npm.cmd run lite:exe` 已通过，生成免安装可执行文件：
  `release/TaskFlow-Lite.exe`。
- 当前实测 Lite 版启动后工作集约 33.7 MB，私有内存约 11.2 MB。
- 2026-06-09 修复 Lite 验收问题：窗口标题栏可拖动、任务列表改为 WebView2 更稳定的 fallback 拖拽、release exe 改为 Windows GUI 子系统避免弹出黑色控制台窗口。
- 修复后再次实测：工作集约 21.9 MB，私有内存约 4.7 MB。
- 2026-06-09 新增轻量桌面组件和 Windows 原生托盘：主窗口关闭后隐藏到托盘，托盘菜单支持显示主窗口、显示/隐藏桌面组件、退出；桌面组件支持选择项目、置顶、紧凑模式、透明度、显示数量、拖动位置和调整大小。
- 新版启动实测：工作集约 33.7 MB，私有内存约 11.3 MB。
- 2026-06-09 修复显示桌面组件后主界面卡住的问题：组件窗口改为异步延迟创建，主界面按钮不再等待第二个 WebView 初始化；启动时恢复组件也改为延迟执行。修复后在 `visible=true` 配置下启动 12 秒，进程保持响应，工作集约 23.1 MB，私有内存约 5.7 MB。

当前限制：

- `lite:build` 生成安装包时需要下载 NSIS 打包器；当前网络访问 GitHub 可能超时。
- Tauri Lite 已补齐数据导出、数据导入、日志导出文件对话框。

## 环境准备

1. 安装 Rust：

   https://www.rust-lang.org/tools/install

2. 安装 Windows WebView2 Runtime（多数 Windows 11 已自带）：

   https://developer.microsoft.com/microsoft-edge/webview2/

3. 首次运行 Tauri CLI：

   ```bash
   npm.cmd run lite:dev
   ```

   如果 `npx` 需要下载 `@tauri-apps/cli`，需要允许网络访问。

## 验证命令

当前 Electron 版回归：

```bash
npm.cmd run verify
```

Tauri Rust 侧检查（安装 Rust 后）：

```bash
npm.cmd run lite:check
```

Tauri 开发模式：

```bash
npm.cmd run lite:dev
```

Tauri 免安装 exe 构建：

```bash
npm.cmd run lite:exe
```

Tauri 安装包构建：

```bash
npm.cmd run lite:build
```

## 下一步迁移任务

1. 跑 `npm.cmd run lite:dev`，验证 Vue 前端能通过 `src/runtime/api.js` 调用 Tauri commands。
2. 补齐 Tauri 端系统字体读取。
3. 做 Electron 与 Tauri 的数据路径迁移或导入流程。
4. 做长时间运行测试，重点观察拖拽是否稳定、内存是否持续增长。
5. 如需安装包分发，解决 NSIS 下载或改为本地预置打包器。
