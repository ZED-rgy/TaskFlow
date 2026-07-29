<div align="center">
  <img src="assets/icon.svg" width="96" alt="小光任务图标" />
  <h1>TaskFlow · 小光任务</h1>
  <p>一个本地优先、轻量的 Windows 桌面任务管理器。</p>
</div>

## 功能

- 项目、任务、子任务、优先级、标签、备注与重复任务
- 今天、近 7 天、已完成等智能视图
- 自然语言快速添加与全局快捷键
- 可拖拽排序的任务列表和桌面组件
- 系统托盘、到期提醒、数据导入导出与自动备份
- 多主题与系统字体设置
- 单实例运行，关闭主窗口后可继续驻留托盘

所有任务数据默认保存在本机，不依赖在线账号或云服务。

## 技术栈

- Vue 3 + Vite：界面与交互
- Tauri 1 + Rust：桌面窗口、本地存储、系统托盘和通知
- SortableJS：任务拖拽排序

当前主要面向 Windows。Rust 后端使用了 Windows 注册表读取能力，其他平台尚未完成适配与验证。

## 快速开始

### 环境要求

- Node.js 18+
- Rust stable 与 Cargo
- Windows WebView2 和 Tauri 1 所需的系统构建环境

```powershell
git clone https://github.com/ZED-rgy/TaskFlow.git
cd TaskFlow
npm ci
npm.cmd run dev
```

## 常用命令

| 命令 | 用途 |
| --- | --- |
| `npm.cmd run dev` | 启动 Tauri 开发模式 |
| `npm.cmd run dev:web` | 仅启动 Vite 前端 |
| `npm.cmd run verify` | 运行规则测试、前端构建、Rust 检查和单元测试 |
| `npm.cmd run build` | 生成本地便携版 exe |
| `npm.cmd run build:installer` | 生成 NSIS 安装包 |
| `npm.cmd run generate:icon` | 重新生成 Windows 图标与网页 favicon |

更完整的开发、构建和桌面验收步骤见 [docs/development.md](docs/development.md)。

## 项目结构

```text
TaskFlow/
├─ assets/                 应用图标源文件与 Windows 图标
├─ docs/                   开发、架构和历史资料
├─ public/                 Vite 静态资源
├─ scripts/                图标、构建复制与规则测试脚本
├─ src/
│  ├─ components/          主窗口界面模块
│  ├─ runtime/             Tauri 调用适配与纯业务规则
│  ├─ App.vue              主窗口入口
│  ├─ QuickAdd.vue         全局快速添加窗口
│  └─ Widget.vue           桌面组件窗口
└─ src-tauri/
   ├─ src/main.rs          Rust 后端、数据层与系统能力
   └─ tauri.conf.json      Tauri 构建和权限配置
```

模块关系和数据流见 [docs/architecture.md](docs/architecture.md)。

## 数据与隐私

应用数据位于：

```text
%APPDATA%\com.personal.taskflow-lite\taskflow-data.json
```

启动和导入前会自动创建备份，默认保留最近 30 份。应用不包含遥测、在线登录或第三方云同步。

## 质量检查

当前回归链路包括：

- 智能视图和桌面组件排序规则测试
- Vite 生产构建
- `cargo check`
- 21 个 Rust 数据规范化、导入、重复任务和任务树单元测试

提交改动前请运行：

```powershell
npm.cmd run verify
```

## 参与开发

请先阅读 [CONTRIBUTING.md](CONTRIBUTING.md)。发现问题时，建议附上复现步骤、系统版本和诊断日志中的相关信息，并在分享前移除个人任务内容。

## 开源许可

本项目基于 [MIT License](LICENSE) 开源。
