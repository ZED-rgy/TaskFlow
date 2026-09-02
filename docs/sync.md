# 云同步基础（P0）

当前版本完成的是本地优先同步基础，不会自动连接云端，也不会上传用户数据。桌面端所有成功的工作区变更都会在应用数据目录中维护一个 `sync-state.json`，为后续云端适配器提供稳定边界。

## 本地契约

- `deviceId`：首次创建的设备 UUID，跨重启保持不变。
- `cursor`：最近一次云端确认的同步游标；尚未同步时为空。
- `outbox`：待确认操作列表。P0 使用 `workspace/snapshot` 操作，保存最新完整工作区快照，并合并掉更早的未发送快照，避免离线连续编辑造成无限增长。
- 每个操作都有 `operationId`、`createdAt` 和 `baseCursor`，云端适配器可据此实现幂等提交与版本检查。

Tauri 命令已暴露为：

- `get_sync_status`：读取设备 ID、游标和待同步数量。
- `get_sync_outbox`：读取待同步操作和游标，供同步 worker 使用。
- `acknowledge_sync`：云端确认操作后移除对应操作并推进游标。

前端对应方法位于 `src/runtime/api.js`。本地数据写入失败不会阻断原有桌面操作，但会写入应用日志以便诊断。

## 后续阶段边界

P1 再接入认证与云端数据库（工作区成员、实体版本、软删除标记和 RLS）；P2 才启用实时订阅、增量拉取与冲突处理。届时应把快照 outbox 逐步升级为项目/任务级操作，同时保留操作 ID 幂等约束。

## Supabase 配置

复制 `.env.example` 为 `.env`，填入 Supabase 项目 URL 和公开 anon key：

```text
VITE_SUPABASE_URL=https://your-project.supabase.co
VITE_SUPABASE_ANON_KEY=your-public-anon-key
```

前端只允许使用公开 anon key；`service_role` key 不能进入桌面包、浏览器或 Git。未配置变量时，`syncRepository.enabled` 为 `false`，所有云端方法会给出明确错误而不是静默伪同步。
