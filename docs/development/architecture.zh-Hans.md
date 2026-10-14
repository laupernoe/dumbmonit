# 架构

DumbMonit 以单个容器的形式发布。容器内有一个 Rust 进程,负责提供 Web 界面和 API,
另有一个子进程(VictoriaMetrics),由该 Rust 进程启动并监管。配置和状态保存在
SQLite 中。

## 组件

| 组件 | 位置 | 作用 |
|---|---|---|
| 服务器 | `crates/server` | 唯一的 Rust 二进制文件:HTTP API(axum)、身份验证、调度器、告警引擎、通知渠道、报告。 |
| Web 界面 | `web/` | SvelteKit 单页应用(Svelte 5、Tailwind 4、uPlot)。构建为静态文件,并通过 `rust_embed` 嵌入服务器二进制文件。 |
| 时间序列 | `crates/server/src/tsdb/` | VictoriaMetrics 的批量写入器和查询代理。`embedded.rs` 启动并监管内置的 VictoriaMetrics 子进程(数据位于 `/data/vm`);也可以让 `DUMBMONIT_VM_URL` 指向外部实例。 |
| 配置和状态 | `crates/server/src/db/` | 带编号迁移的 SQLite:设备、凭据、用户、规则、告警状态、静默、渠道。 |
| 采集器 | `crates/collectors`、`crates/server/src/collectors/` | 每种设备或服务对应一个 `Collector` trait 的实现,并注册到 `Registry` 中。调度器只认识这个 trait。 |
| 共享类型 | `crates/proto` | `Sample`、`Target`、`Credential`、`Collector` trait 和 `ProbeError`,以及代理协议。 |
| 代理 | `crates/agent` | 推送主机指标的 Linux、macOS、FreeBSD 和 Windows 代理;它还可以充当中继,为远程站点运行采集器。 |
| 告警 | `crates/server/src/alerting/` | 规则、告警状态机、按父设备抑制、静默、学习得到的基线。 |
| 通知 | `crates/server/src/notify/` | 渠道(聊天、推送、值班、电子邮件、webhook),由 `catalog.rs` 向界面描述。 |
| 密钥 | `crates/server/src/crypto.rs` | 使用 AES-256-GCM 加密存放在 SQLite 中的凭据和令牌;密钥来自 `/data/secret.key` 或 `DUMBMONIT_SECRET`。 |

## 数据流

```text
 devices / services ──(SNMP, HTTP APIs, probes)──► collectors ┐
                                                              │ samples
 agents ──(push, per-machine token)──► /api/ingest ───────────┤
 relay agent (collectors run at a remote site) ───────────────┤
                                                              ▼
                                           tsdb writer (batched) ──► VictoriaMetrics
                                                                           │
 browser / scripts / assistants ◄─ /api/* ◄─ query proxy ◄─────────────────┘
                                   │
                                   └─► alerting engine ──► notify channels

 SQLite (config, users, alert state, silences) is read and written by the
 API, the scheduler and the alerting engine.
```

1. 调度器按各自的间隔,通过采集器注册表运行每个已启用的设备。代理和中继使用
   每台机器独立的令牌,将测量数据推送到 `/api/ingest`。
2. 采集器返回名为 `dumbmonit_<kind>_*`、带有 `target` 标签(设备 id)的样本。
   写入器将它们批量写入 VictoriaMetrics。
3. 告警引擎根据已存储的序列评估规则。`phase` 是引擎的状态;`effective_phase`
   是用户看到的状态(父设备宕机时为 `suppressed`)。无法访问的设备会触发告警;
   配置错误会显示出来,但不会发送通知。
4. 通知通过通知策略中配置的渠道发出。
5. 界面和任何脚本都通过 `/api/*` 读取所有内容;API 没有私有的另一半。

## 扩展点

- 新增一种设备,就是一个 `Collector` 实现加一次 `Registry::register` 调用。
  界面表单由 `GET /api/collectors` 生成。
- 新增通知渠道,就是一个发送器加上 `notify/catalog.rs` 中的一个条目;界面根据
  `GET /api/notify/kinds` 渲染表单。
- 只需要配置文件的 SNMP 设备无需编写 Rust:在 `profiles/` 下放一个 YAML 文件即可。

构建说明请参阅[开发](../development.md),信任边界请参阅[安全模型](security-model.md)。
