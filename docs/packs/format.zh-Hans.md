# Pack 格式参考 {#pack-format-reference}

一个 pack 是一个 YAML 文件，`pack.yaml`，格式版本为 1。未知键是错误，不是忽略的设置：打字错误如 `treshold` 会被报告。

```text
my-pack/
  pack.yaml          pack 本身：服务器唯一需要的文件
  fixtures/          每个源的捕获响应，按其 id 命名
    status.json
  expected.prom      提取必须从 fixtures 生成的内容
```

## Header {#header}

| Key | Required | Meaning |
|---|---|---|
| `schema` | yes | 始终为 `1`。 |
| `id` | yes | 小写字母、数字和单个破折号，以字母开头，最多 40 个字符：`shelly-plug`。设备类型变为 `pack.<id>`，度量前缀为 `dumbmonit_<id with underscores>_`。 |
| `version` | yes | [语义版本](https://semver.org) of the pack: `1.2.0`。 |
| `requires` | no | 该 pack 适用的 DumbMonit 版本，作为 semver 要求：`">=0.2"`。超出范围的服务器拒绝该 pack。 |
| `label` | yes | 设备类型列表中显示的名称。 |
| `summary` | no | 一句话：pack 监控什么。 |
| `examples` | no | 它适用的产品，显示为提示。 |
| `address_hint` | no | 地址字段的占位符：`192.168.1.50`。 |
| `default_port` | no | 当地址没有端口时使用的端口。 |
| `scheme` | no | `http`（默认）或 `https`；设备可以覆盖它。 |
| `setup` | no | 设备表单旁边显示的通知：`title`、`steps`（列表；步骤中换行符后的内容显示为可复制的命令）、`warning`、`doc_url`。 |
| `credentials` | no | 接受的凭证类型，按优先顺序：`none`、`api_token`、`username_password`。默认：`[none]`。一个条目可以是 `{kind, label, help}` 来重新表述表单。 |
| `options` | no | 设备表单中显示的设置（下面）。 |

## Options {#options}

Options 存储在设备上，由路径、标头和正文中的 `{{option.<key>}}` 读取，并由规则阈值读取。

```yaml
options:
  - key: switch_id          # 小写、数字、下划线
    label: Switch channel
    help: 0 for a plug; 0, 1, 2… for each channel of a relay.
    input: number           # text (default), number, boolean or select
    default: 0
    required: false
    placeholder: ""
    choices: []             # select 的值
```

每个也进行 polling 的 pack 都获得五个连接选项，它不能重新声明：`scheme`、`port`、`insecure_tls`、`request_timeout_seconds`（1 到 120，默认 10）和 `allow_private_targets`。

## Sources {#sources}

一个源是每次 poll 的一个请求，始终发送到设备自己的地址。

```yaml
sources:
  - id: status              # 小写、数字、下划线
    type: http              # http or prometheus
    path: /rpc/Switch.GetStatus?id={{option.switch_id}}
    method: GET             # GET (default) or POST
    format: json            # http only: json (default) or text
    headers:
      Accept: application/json
    body: '{"id": {{option.switch_id}}}'   # POST only
    auth: auto              # auto (default), none, bearer or basic
```

- `path` 必须以单个 `/` 开头。请求 URL 是设备地址后跟路径；替换后必须保持相同的 scheme、host 和 port，否则请求被拒绝。Option 值在路径中是百分比编码的，所以一个值不能添加段或更改主机。
- `auth: auto` 将 `api_token` 凭证作为 `Authorization: Bearer …` 发送，`username_password` 凭证作为基本认证。`none` 隐式不发送任何内容，所以标头可以改为携带凭证：`X-Api-Key: "{{credential.token}}"`。`{{credential.token}}`、`{{credential.username}}` 和 `{{credential.password}}` 在**仅在标头中**被接受：从不在路径或正文中，这些最终会出现在日志中。
- 重定向仅遵循相同的 scheme、host 和 port，最多三次。`401` 和 `403` 作为身份验证错误报告，其他非 2xx 状态作为协议错误。
- 一个 poll 最多进行 16 个请求，每个响应最多读取 4 MiB。

### Prometheus sources {#prometheus-sources}

```yaml
sources:
  - id: metrics
    type: prometheus
    path: /metrics
    keep: [node_load1, node_memory_MemAvailable_bytes]
    rename:
      node_memory_MemAvailable_bytes: memory_available_bytes
    drop:
      fstype: ^(tmpfs|overlay)$
```

- `keep` 列出导入的族，保持原样，包含所有标签。一个族名也涵盖其直方图和摘要系列（`_bucket`、`_sum`、`_count`）。Counters、直方图和摘要总和保持 counters；其他一切是 gauge。
- `rename` 给一个保留的族其 DumbMonit 名称。不是小写字母、数字和下划线的名称必须重命名。
- `drop` 丢弃标签匹配正则表达式的每个样本。
- DumbMonit 自身设置的标签（`target`、`host`、`instance`、`job`、`tag_*`…）保留为 `exported_<name>`。

## Metrics {#metrics}

每个度量的名称**没有**前缀：`shelly-plug` pack 中的 `power_watts` 写成 `dumbmonit_shelly_plug_power_watts`。名称是小写字母、数字和下划线，在 pack 中唯一。

```yaml
metrics:
  - name: power_watts
    help: Active power drawn by the load.
    source: status
    kind: gauge             # gauge (default) or counter
    json: $.apower          # exactly one of json, regex or prom
    scale: 1                # multiplier, e.g. 0.001 for ms → s
    map: {on: 1, off: 0}    # text value → number
    labels: {}
```

| Extractor | Source | Value | `labels` |
|---|---|---|---|
| `json: <JSONPath>` | `http`, format `json` | 路径匹配的每个节点（[RFC 9535](https://www.rfc-editor.org/rfc/rfc9535) JSONPath，包含通配符和过滤器）生成一个系列。 | label → JSONPath 在**包含**该值的对象上求值：`$.sensors[*].value` 使用其兄弟 `name` 标签化每个值。 |
| `regex: <expression>` | `http`, format `text` | 每个匹配生成一个系列；值是命名为 `value` 的组，否则是第一组。 | label → 捕获组的名称。 |
| `prom: <sample name>` | `prometheus` | 具有此确切名称的每个样本。 | label → 源标签。空：保留所有源标签。 |

值：数字按原样取用；`true`/`false` 变成 1/0；文本在 `map` 中查找，或在没有 `map` 时解析为数字。缺失、`null` 或超出 `map` 范围的值为此 poll 不产生任何内容。

## Discover {#discover}

`discover` 将列表的每个元素变成共享标签的一组系列：每个磁盘、传感器或队列一行。

```yaml
discover:
  - name: disks
    source: list
    rows: $.disks[*]        # JSONPath：每个匹配一行
    max_rows: 100           # default 100, at most 1,000
    labels:                 # JSONPath 在每行上求值
      disk: $.name
      model: $.info.model
    metrics:
      - name: disk_used_bytes
        json: $.used        # JSONPath 在每行上求值
      - name: disk_healthy
        json: $.state
        map: {ok: 1, degraded: 0}
```

行度量接受 `name`、`help`、`kind`、`json`、`scale` 和 `map`。超过 `max_rows` 的行被忽略。

## Rules {#rules}

规则随 pack 一起安装，作为常规警报规则，uid 为 `pack:<id>:<name>`。然后它们属于您：pack 的更新添加它没有的规则，从不重写您更改或删除的规则。

```yaml
rules:
  - name: overheating       # 小写、数字、下划线
    title: Shelly plug overheating
    description: The internal temperature stayed above the limit for five minutes.
    expr: dumbmonit_shelly_plug_temperature_celsius
    op: ">"                 # >, >=, < or <=
    threshold: "{{option.max_temperature}}"   # or a number
    for: 5m                 # 30s, 5m, 1h, 1d
    severity: warning       # info, warning (default) or critical
    unit: °C
```

- `expr` 是 [MetricsQL](https://docs.victoriametrics.com/metricsql/)，必须使用 pack 的至少一个度量（按其完整名称）。它引用的任何其他 `dumbmonit_*` 名称也必须由 pack 产生（`dumbmonit_up` 除外）：一个 pack 不能对另一个类型的系列报警。
- `{{option.x}}` 的 `threshold` 安装该选项的默认值，每个设备自己的该选项值变成其针对此规则的阈值（每次保存设备时重写的每设备覆盖）。

## SNMP profiles {#snmp-profiles}

一个 pack 可以带 SNMP profiles，写法完全像 [`profiles/`](https://github.com/laupernoe/dumbmonit/tree/main/profiles) 中的内置 ones。它们加入 SNMP 目录，并像其他的一样由 `sysObjectID` 应用；它们不创建自己的设备类型。

```yaml
snmp_profiles:
  - id: acme-ups            # 该 pack id，或以"<pack id>-"开头
    name: ACME UPS
    match:
      sysobjectid: [1.3.6.1.4.1.99999]
    include: [system]
    metrics:
      - name: acme_ups_battery_percent   # 必须以 pack 前缀开头
        oid: 1.3.6.1.4.1.99999.1.1.0
        kind: gauge
```

SNMP 目录在启动时读取：安装或更新带 SNMP profiles 的 pack 后重启 DumbMonit。

## Limits {#limits}

| Limit | Value |
|---|---|
| Pack size | 256 KiB |
| Sources (requests per poll, redirects included) | 16 |
| Response size | 4 MiB |
| Series per poll | 5,000 (the rest is dropped and logged) |
| Metrics declared | 256 |
| Rules | 64 |
| Rows per `discover` | 1,000 |
| Label value length | 200 bytes |

## Fixtures and expected output {#fixtures-and-expected-output}

`dumbmonit pack test <dir>` 读 `fixtures/<source id>.json`、`.txt` 或 `.prom`，运行与服务器相同的提取，并将结果与 `expected.prom` 比较：Prometheus 文本格式，排序，没有时间戳，没有服务器添加的设备标签（`target`、`host`、`tag_*`）。`--update` 从 fixtures 写 `expected.prom`；在提交前审查 diff。
