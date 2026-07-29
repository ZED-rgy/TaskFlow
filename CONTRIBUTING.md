# 参与 TaskFlow 开发

感谢你愿意改进 TaskFlow。当前项目主要在 Windows 上开发和验证。

## 开发流程

1. 从 `main` 创建一个描述明确的分支。
2. 使用 `npm ci` 安装锁定版本的前端依赖。
3. 运行 `npm.cmd run dev` 启动桌面开发环境。
4. 尽量把纯业务规则放在 `src/runtime/`，并为规则补充脚本测试。
5. Rust 数据规则需要补充或更新 `src-tauri/src/main.rs` 中的单元测试。
6. 提交前运行 `npm.cmd run verify`。

## 代码约定

- 文本文件使用 UTF-8 和 LF 换行。
- 前端使用 Vue 3 Composition API。
- 前端通过 `src/runtime/api.js` 访问 Tauri 命令，不在界面模块中直接拼接命令调用。
- 不提交 `node_modules/`、`dist/`、`release/` 或 `src-tauri/target/`。
- 不提交真实任务数据、备份、日志、密钥或本机路径。

## 问题报告

请提供：

- 可复现的操作步骤
- 预期结果和实际结果
- Windows 与 WebView2 版本
- 是否能在最新 `main` 上复现
- 必要的诊断日志片段

日志或导出数据可能包含任务标题、备注与本机路径，公开前请先脱敏。
