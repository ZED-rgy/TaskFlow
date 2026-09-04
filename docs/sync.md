# 云同步基础（P1）

当前版本完成的是本地优先同步基础，并在配置 Supabase、登录后自动准备个人同步空间并连接云端。桌面端所有成功的工作区变更都会在应用数据目录中维护一个 `sync-state.json`；同步 worker 会将待确认快照上传，并接收其他设备的远端更新。

## 本地契约

- `deviceId`：首次创建的设备 UUID，跨重启保持不变。
- `cursor`：最近一次云端确认的同步游标；尚未同步时为空。
- `outbox`：待确认操作列表。P0 使用 `workspace/snapshot` 操作，保存最新完整工作区快照，并合并掉更早的未发送快照，避免离线连续编辑造成无限增长。
- 每个操作都有 `operationId`、`createdAt` 和 `baseCursor`，云端适配器可据此实现幂等提交与版本检查。
- 快照生成时机：本地编辑递增内存中的变更版本，随 500ms 防抖落盘一起把当前状态写成一份快照进 outbox。连续输入不会每次按键都全量 fsync `sync-state.json`；只有对应版本成功进入 outbox 后才会推进确认版本，因此写入失败会自动重试，旧快照也不会清掉较新的修改。
- `sync-state.json` 使用同目录临时文件原子替换，并保留最近一个可解析的 `sync-state.json.prev`；主文件缺失或损坏时会自动恢复，而不是创建空队列覆盖现场。

## 云端事件保留

`sync_events` 是追加表，但每条 `workspace/snapshot` 都是完整状态，旧快照对任何客户端都没有增量价值。迁移 `20260903000100_sync_events_retention.sql` 增加了 `after insert` 触发器：每次写入新快照后，只保留该工作区最近 20 条快照，更早的行由 `security definer` 函数删除，客户端本身仍然没有 delete 权限。`seq` 是 identity 列，删除不会复用编号，游标停在已删行上的客户端下次拉 `seq > cursor` 会直接拿到更新的快照。Realtime 只订阅 INSERT，删除不会打扰客户端。

Tauri 命令已暴露为：

- `get_sync_status`：读取设备 ID、游标和待同步数量。
- `get_sync_outbox`：读取待同步操作和游标，供同步 worker 使用。
- `set_sync_workspace`：绑定或解绑当前云端工作区。首次绑定允许携带本地待同步队列；从一个已绑定工作区切换到另一个前必须清空队列，切换后游标会重置。
- `acknowledge_sync`：云端确认操作后移除对应操作并推进游标。
- `enqueue_local_snapshot`：不经编辑直接把当前本地工作区放入 outbox。首次绑定选择"使用本机 / 合并"时用它主动上传。
- `backup_local_data`：以指定原因名生成一份本地数据备份。首次绑定可能覆盖本地数据前调用。

前端对应方法位于 `src/runtime/api.js`。本地数据写入失败不会阻断原有桌面操作，但会写入应用日志以便诊断。

## 当前阶段边界

P1 已提供认证、工作区创建、云端表结构、RLS、同步事件写入、增量拉取和 Realtime 订阅适配器。主窗口优先保持当前已绑定工作区；只有未绑定或原工作区不可访问时，才选择已有个人空间或创建“我的任务”。Realtime 事件用于即时触发，5 秒轮询用于断线恢复；远端快照经过本地运行时校验后写入，不会再次进入 outbox。Supabase SDK 按需加载，未配置或未使用云同步时不会进入首屏主包。

迁移 `20260904000100_sync_compare_and_swap.sql` 将客户端写入收口到 `push_sync_event` RPC：函数按工作区加事务锁，并检查 `baseCursor` 之后是否出现其他设备的事件。这样可覆盖“本机拉取完成后，另一台设备恰好抢先上传”的竞态；服务端会以 `40001` 拒绝旧快照，客户端重新拉取并进入显式冲突处理。应用新版本前必须先执行这条迁移，否则新版客户端没有对应 RPC，上传会失败而不会静默降级为不安全直写。

## Supabase 配置

复制 `.env.example` 为 `.env`，填入 Supabase 项目 URL 和公开 anon key：

```text
VITE_SUPABASE_URL=https://your-project.supabase.co
VITE_SUPABASE_ANON_KEY=your-public-anon-key
```

前端只允许使用公开 anon key；`service_role` key 不能进入桌面包、浏览器或 Git。未配置变量时，`syncRepository.enabled` 为 `false`，所有云端方法会给出明确错误而不是静默伪同步。

`src-tauri/tauri.conf.json` 保留兼容可配置项目的基础 CSP；Vite 构建时还会从 `VITE_SUPABASE_URL` 注入更严格的页面级 CSP，两层策略取交集后只允许连接实际配置的 HTTPS/WSS 项目域名。HTTP 远端地址会直接导致构建失败。若使用 Supabase 自定义域名，仍只需把其 HTTPS URL 写入 `.env`。

`createSyncEngine` 负责一次同步编排：先按游标拉取远端事件，再按批次幂等推送本地 outbox。本地和外部远端事件同时存在时不会上传；调用方必须先完成“合并 / 使用云端 / 使用本机”选择。普通拉取也不会自动确认游标，只有远端快照成功应用后才调用 `commitRemoteCursor`。`App.vue` 负责登录态检查、工作区选择、worker 生命周期和冲突交互；Realtime 只有收到 `SUBSCRIBED` 状态后才显示连接成功。

## 首次绑定的数据冲突处理

由于当前是整库快照覆盖，首次把本机绑定到某个云端工作区时，`App.vue` 会先比较两边是否都含有用户数据（判定规则见 `src/runtime/sync-merge.mjs` 的 `hasMeaningfulData`，安装时的示例项目和示例任务不算）：

- 云端为空或只有示例：本机数据作为初始快照上传。
- 本机为空或只有示例：直接采用云端最新快照。
- 两边都有数据：弹窗让用户在"合并两边数据 / 使用云端数据 / 使用本机数据 / 取消"中选择。合并按 id 取并集并把纯示例的同名项目折叠（规则与测试见 `mergeWorkspaces` 和 `scripts/test-sync-merge.mjs`）。选择前会先写一份 `before-cloud-bind` 备份；取消则不绑定、不开启同步。

无论哪种选择，绑定后都会把本地游标推进到云端当前最新事件，避免常规同步把历史快照重新拉下来覆盖刚刚保留的数据。设置页不再直接绑定工作区，统一交给主窗口走这条流程。

绑定后的多设备冲突采用相同选择界面。选择前写入 `before-sync-conflict` 备份；合并时如果同一个实体 ID 的内容不同，远端条目保持原 ID，本机条目以“本机冲突副本”新 ID 保留，避免表面合并实际丢字段。取消会暂停在该游标，不会反复弹窗或推进游标。

## 邮箱验证回跳

桌面安装包注册了 `taskflow://auth/callback` 自定义协议。请在 Supabase Dashboard 的 Authentication → URL Configuration 中，将 `taskflow://auth/callback` 加入 Redirect URLs；生产环境应将 Site URL 改为真实可访问的 HTTPS 地址，`http://localhost:3000` 仅保留给本地 Web 开发。认证使用 PKCE 流程：回跳链接只携带一次性 `code`，应用用本机保存的 verifier 调用 `exchangeCodeForSession` 换取会话；不是本机发起的登录、或直接携带 `access_token` 的旧式链接都会被拒绝，第三方无法通过构造 `taskflow://` 链接把用户登录到别人的账户。点击验证邮件后，Windows 会唤起小光任务，应用会自动完成换取并刷新登录状态。若应用未运行，首次启动也会读取该回调；若系统阻止协议唤起，仍可回到应用手动登录。
