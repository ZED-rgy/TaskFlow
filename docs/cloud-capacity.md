# 免费云端容量估算（2026-09-05）

注册账号数、月活用户数、同时在线设备数和实际任务容量是不同指标。当前没有多用户负载测试，下面是预算模型，不是承载人数保证。

## 官方额度

Supabase Free 当前包含每项目 500 MB 数据库、50,000 月活认证用户、5 GB 普通出站流量/月、200 个 Realtime 峰值连接、200 万条 Realtime 消息/月。月活额度不是总注册人数限制，也不是应用能承载 5 万活跃用户的保证；电脑和手机同时在线通常各占一个连接。[官方额度](https://supabase.com/docs/guides/platform/billing-on-supabase)

默认邮件服务仅向项目团队的邮箱发信，当前限额 2 封/小时。面向其他用户开放邮箱验证注册，需要配置自有 SMTP。本次未读取当前 SMTP 配置，不判断其是否已经配置。[邮件服务说明](https://supabase.com/docs/guides/auth/auth-smtp)

## 当前数据与应用行为

本轮只读统计：数据库约 12.4 MB，应用表及索引约 0.72 MB；目前仅一个账号，因此不能把这份样本的表空间简单线性外推。保留了 20 份同步快照，平均每份 JSON 约 13.9 KB；任务历史 34 条，平均 body 存储约 277 字节，不含行和索引开销。

- 每个个人空间最多保留 20 份同步快照；任务更多、备注更长会增大快照。
- 任务状态历史独立长期保留，随着修改次数累积。例：每天 20 次版本变化、每条 body 300 字节，一年仅 body 就约 2.19 MB/人，还需预留索引、元数据与快照空间。
- 个人同步每 5 秒轮询，返回游标之后的数据；没有变化时不会每次下载完整快照。
- 小组页每 15 秒重新查询公开任务。例：假设每次返回 20 KB、每天查看 1 小时，30 天约 144 MB/人，仅 30 个这样的日常查看者就约 4.32 GB/月，还未计入个人同步和认证流量。实际应以账单统计为准。

## 建议的试用规模

先以 50–100 名活跃用户作为内测目标，并保持同时在线设备数明显低于 200。轻度个人任务同步可进一步评估 100–300 人，但要观察历史增长；频繁查看小组的几十人就可能先触及出站流量预算。这些是基于上述假设的规划范围，不能视作压测结果。

扩量前重点记录每日数据库增量、月出站流量、峰值 Realtime 连接与接口延迟。长期优化方向是小组页只在内容变化时更新，以及有明确保留规则的历史归档；不要为节省空间直接删除用户已承诺保留的历史。

## 当前安全提醒

本轮收紧了 `rls_auto_enable` 的客户端执行权限，并补充了三处外键索引；其原有 DDL 事件触发器保留。[辅助函数授权说明](https://supabase.com/docs/guides/database/database-linter?lint=0028_anon_security_definer_function_executable) · [外键索引说明](https://supabase.com/docs/guides/database/database-linter?lint=0001_unindexed_foreign_keys)

小组私有表启用 RLS 且没有客户端读写策略，这是默认拒绝、通过校验 RPC 访问的设计，不应为了消除提示而放开策略。[RLS 提示说明](https://supabase.com/docs/guides/database/database-linter?lint=0008_rls_enabled_no_policy)

“泄露密码检测未启用”提醒仍存在，该功能需要 Pro 或以上套餐；本次没有升级套餐。[官方密码安全说明](https://supabase.com/docs/guides/auth/password-security#password-strength-and-leaked-password-protection)
