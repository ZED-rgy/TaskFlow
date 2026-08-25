# 安全审计说明

## RustSec 例外

CI 仅忽略以下两个 `quick-xml` 公告：

| 公告 | 锁定依赖路径 | 适用性判断 |
| --- | --- | --- |
| `RUSTSEC-2026-0194` | `tauri-plugin-notification -> notify-rust -> tauri-winrt-notification 0.7.x -> quick-xml 0.37.x` | 不可达；公告影响属性解析，当前依赖只调用 `quick_xml::escape::escape` 生成 Windows 通知 XML |
| `RUSTSEC-2026-0195` | 同上 | 不可达；公告影响 `NsReader` 命名空间解析，当前依赖不使用 Reader 或 `NsReader` |

RustSec 按包版本报告，无法识别具体 API 是否可达，因此对这两个公告使用了有范围的 CI 例外。`quick-xml` 的另一条传递依赖已通过 `plist 1.10` 升级到修复后的 `0.41`。

删除例外的触发条件：`notify-rust` 改用 `tauri-winrt-notification 0.8+`，或后者在兼容版本中把 `quick-xml` 升级到 `0.41+`。除此之外的漏洞公告不会被忽略。

RustSec 还会报告 Tauri 在 Linux 目标使用的 GTK3 未维护警告。TaskFlow 当前仅面向 Windows，这些包不进入 Windows 可执行文件；警告保留在审计结果中，便于未来扩展平台时重新评估。
