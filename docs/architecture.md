# TaskFlow 架构

## 总览

TaskFlow 是一个 Vue 3 + Tauri 1 桌面应用。前端负责界面状态和交互，Rust 后端负责持久化、备份、窗口生命周期、系统托盘、通知、全局快捷键与系统字体读取。

```text
App / Widget / QuickAdd
          │
          ▼
 src/runtime/api.js
          │  Tauri invoke
          ▼
src-tauri/src/main.rs
   ├─ 内存状态与防抖落盘
   ├─ JSON 规范化、迁移和备份
   ├─ 主窗口 / 桌面组件 / 快速添加窗口
   └─ 托盘、通知、快捷键和系统字体
```

## 前端

### 三个窗口入口

`src/main.js` 根据查询参数挂载不同根视图：

- `src/App.vue`：主窗口
- `src/Widget.vue`：桌面组件
- `src/QuickAdd.vue`：全局快速添加

### 界面模块

`src/components/` 保存主窗口的界面模块：

- `Sidebar.vue`：智能视图、项目和项目排序
- `TaskList.vue`：任务新增、筛选、拖拽和键盘操作
- `TaskItem.vue`：单条任务与子任务交互
- `TaskDetail.vue`：任务完整字段编辑
- `SettingsView.vue`：主题、字体、组件、备份和诊断
- `CommandPalette.vue`：跨项目搜索与命令导航

### 运行时模块

`src/runtime/api.js` 是前端访问 Rust 的唯一适配器。`taskviews.mjs`、`widget-order.mjs` 和 `quickparse.js` 保存可独立验证的业务规则；主题和字体规则分别位于 `themes.js` 与 `fonts.js`。

## Rust 后端

`src-tauri/src/main.rs` 当前包含以下职责：

- 数据模型、旧版本兼容和字段规范化
- 内存状态、原子落盘、损坏恢复与备份轮换
- 项目和任务命令
- 主窗口、桌面组件与快速添加窗口管理
- 系统托盘、通知、全局快捷键和单实例控制
- 日志、导入导出和系统字体读取

这个文件是当前最大的维护热点。后续重构时，适合按 `model`、`storage`、`commands`、`windows`、`settings` 拆成内部 Rust 模块，但应保持现有 Tauri 命令名称和 `src/runtime/api.js` 接口稳定，避免把复杂度扩散到三个前端入口。

## 数据流

1. 窗口通过 `src/runtime/api.js` 调用 Tauri 命令。
2. Rust 命令读取共享内存状态并完成规范化修改。
3. 数据变更向其他窗口广播，保持主窗口与桌面组件同步。
4. 写入经过防抖后保存到本地 JSON；退出等关键节点会强制刷新。
5. 启动和导入前创建备份，损坏数据会尝试从最近有效备份恢复。

## 测试接口

- `scripts/test-taskviews.mjs`：智能视图日期规则
- `scripts/test-widget-order.mjs`：桌面组件可见任务排序
- `src-tauri/src/main.rs` 的 `tests` 模块：数据规范化、导入、重复日期和任务树

完整验证入口是 `npm.cmd run verify`。
