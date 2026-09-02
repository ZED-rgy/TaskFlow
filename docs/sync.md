# 云同步基础（P1）

当前版本完成的是本地优先同步基础，并在配置 Supabase、登录且绑定工作区后自动连接云端。桌面端所有成功的工作区变更都会在应用数据目录中维护一个 `sync-state.json`；同步 worker 会将待确认快照上传，并接收其他设备的远端更新。

## 本地契约

- `deviceId`：首次创建的设备 UUID，跨重启保持不变。
- `cursor`：最近一次云端确认的同步游标；尚未同步时为空。
- `outbox`：待确认操作列表。P0 使用 `workspace/snapshot` 操作，保存最新完整工作区快照，并合并掉更早的未发送快照，避免离线连续编辑造成无限增长。
- 每个操作都有 `operationId`、`createdAt` 和 `baseCursor`，云端适配器可据此实现幂等提交与版本检查。

Tauri 命令已暴露为：

- `get_sync_status`：读取设备 ID、游标和待同步数量。
- `get_sync_outbox`：读取待同步操作和游标，供同步 worker 使用。
- `set_sync_workspace`：绑定或解绑当前云端工作区。首次绑定允许携带本地待同步队列；从一个已绑定工作区切换到另一个前必须清空队列，切换后游标会重置。
- `acknowledge_sync`：云端确认操作后移除对应操作并推进游标。

前端对应方法位于 `src/runtime/api.js`。本地数据写入失败不会阻断原有桌面操作，但会写入应用日志以便诊断。

## 当前阶段边界

P1 已提供认证、工作区创建、云端表结构、RLS、同步事件写入、增量拉取和 Realtime 订阅适配器。主窗口在登录并绑定工作区后会自动启动同步 worker：Realtime 事件用于即时触发，5 秒轮询用于断线恢复；远端快照经过本地运行时校验后写入，不会再次进入 outbox。当前仍采用完整快照的最后写入覆盖策略，下一阶段再升级为项目/任务级操作、冲突提示和更细粒度合并。

## Supabase 配置

复制 `.env.example` 为 `.env`，填入 Supabase 项目 URL 和公开 anon key：

```text
VITE_SUPABASE_URL=https://your-project.supabase.co
VITE_SUPABASE_ANON_KEY=your-public-anon-key
```

前端只允许使用公开 anon key；`service_role` key 不能进入桌面包、浏览器或 Git。未配置变量时，`syncRepository.enabled` 为 `false`，所有云端方法会给出明确错误而不是静默伪同步。

`createSyncEngine` 负责一次同步编排：先按游标拉取远端事件，再按批次幂等推送本地 outbox。它不会自动确认远端游标；调用方必须成功应用 `remoteEvents` 后，再调用 `commitRemoteCursor`，这样冲突或校验失败时不会跳过远端数据。`App.vue` 负责登录态检查、worker 生命周期、Realtime 订阅和远端快照应用；未登录或未绑定工作区时保持停止状态。
