# 编写你的第一个集成包 {#write-your-first-pack}

本页将在大约十分钟内从零开始构建 `shelly-plug` 参考集成包:通过
本地 API 读取一个 Shelly 智能插座(Plus、Pro 或 Gen3),获取
功率、能耗和温度,并在它过热时发出告警。

你需要 Docker,以及网络上的一个 Shelly 插座,或使用下面提供的
录制应答。

## 1. 查看监控项返回的内容 {#1-look-at-what-the-device-answers}

Shelly 第二代监控项用一次调用就能描述一个继电器:

```bash
curl http://192.168.1.50/rpc/Switch.GetStatus?id=0
```

```json
{
  "id": 0,
  "source": "HTTP_in",
  "output": true,
  "apower": 8.9,
  "voltage": 237.5,
  "current": 0.069,
  "aenergy": { "total": 6532.2, "by_minute": [45.199, 47.141, 88.397] },
  "temperature": { "tC": 41.3, "tF": 106.3 }
}
```

把这个应答保存为 `shelly-plug/fixtures/status.json`:这就是该
集成包将被测试的样例文件,因此你可以在没有实际监控项的情况下
进行开发。

## 2. 头部信息 {#2-the-header}

创建 `shelly-plug/pack.yaml`:

```yaml
schema: 1
id: shelly-plug
version: 1.0.0
label: Shelly plug (Gen2+)
summary: Power, energy, voltage and temperature of a Shelly Plus/Pro/Gen3 plug or relay.
examples: [Shelly Plus Plug S, Shelly Pro 1PM]
address_hint: 192.168.1.50
default_port: 80
credentials: [none]
```

`id` 决定了两个名称:监控项类型 `pack.shelly-plug`,以及指标前缀
`dumbmonit_shelly_plug_`。

## 3. 一个数据源 {#3-one-source}

一个数据源就是一次请求,始终发往监控项自身的地址:

```yaml
options:
  - key: switch_id
    label: Switch channel
    help: 0 for a plug; 0, 1, 2… for each channel of a multi-channel relay.
    input: number
    default: 0

sources:
  - id: status
    type: http
    path: /rpc/Switch.GetStatus?id={{option.switch_id}}
```

`{{option.switch_id}}` 会被替换为监控项自身的设置,在监控项表单中
显示为**Switch channel**。测试样例文件的名称 `status.json` 与
数据源的 `id` 相匹配。

## 4. 指标 {#4-the-metrics}

每个指标通过一个 [JSONPath](https://www.rfc-editor.org/rfc/rfc9535)
表达式提取一个值。名称书写时不带前缀:

```yaml
metrics:
  - name: output_on
    help: 1 when the relay is closed (power flows), 0 when it is open.
    source: status
    json: $.output
  - name: power_watts
    source: status
    json: $.apower
  - name: voltage_volts
    source: status
    json: $.voltage
  - name: current_amperes
    source: status
    json: $.current
  - name: energy_watthours_total
    source: status
    kind: counter
    json: $.aenergy.total
  - name: temperature_celsius
    source: status
    json: $.temperature.tC
```

`output` 是一个布尔值:它会变成 1 或 0。能耗总量只会增长,因此
它是一个 `counter`:图表和规则会用 `rate()` 来读取它,插座重启
也不会表现为一次下跌。

## 5. 检查它 {#5-check-it}

```bash
docker run --rm -v "$PWD:/work" --user "$(id -u)" ghcr.io/laupernoe/dumbmonit:latest \
  pack test /work/shelly-plug --update
```

```text
ok: shelly-plug 1.0.0 (pack.shelly-plug, 6 metrics, 0 rules)
  wrote /work/shelly-plug/expected.prom
```

`expected.prom` 会准确显示服务器将从该样例文件中存储的内容:

```text
# TYPE dumbmonit_shelly_plug_power_watts gauge
dumbmonit_shelly_plug_power_watts 8.9
# TYPE dumbmonit_shelly_plug_temperature_celsius gauge
dumbmonit_shelly_plug_temperature_celsius 41.3
…
```

读一遍即可。从现在起,如果对该集成包的某次修改改变了提取出的
内容,不带 `--update` 的 `pack test` 就会失败,并打印出不同的
那些行。

## 6. 一条告警规则 {#6-an-alert-rule}

规则随集成包一起提供,并通过完整名称使用其指标。阈值可以来自
一个选项,这样每个插座都能有自己的阈值:

```yaml
options:
  # … switch_id, then:
  - key: max_temperature
    label: Temperature alert (°C)
    input: number
    default: 70

rules:
  - name: overheating
    title: Shelly plug overheating
    description: The internal temperature stayed above the limit for five minutes.
    expr: dumbmonit_shelly_plug_temperature_celsius
    op: ">"
    threshold: "{{option.max_temperature}}"
    for: 5m
    severity: warning
    unit: °C
```

运行 `pack lint` 可以检查整个文件:一条规则引用了该集成包不会
产生的指标、一个不存在的选项,或一个指向别处的路径,都会被报告
出来并指明位置。

```bash
docker run --rm -v "$PWD:/work:ro" ghcr.io/laupernoe/dumbmonit:latest pack lint /work/shelly-plug
```

## 7. 安装它 {#7-install-it}

使用一个带有 `write` 范围的 API 令牌:

```bash
curl -H 'Authorization: Bearer dmt_…' -H 'Content-Type: application/yaml' \
  --data-binary @shelly-plug/pack.yaml http://localhost:8080/api/packs
```

打开**监控项 → 添加监控项**:*Shelly plug (Gen2+)* 已经在列表中。
用你插座的地址添加它;首次轮询会在一分钟内进行,*Shelly plug
overheating* 规则会出现在**告警 → 规则**中。

若要发布一个新版本,请提高 `version`,运行 `pack test`,然后
重新安装它:提取逻辑会被替换,而你可能调整过的规则会被保留。

完整的集成包及其设置提示位于
[`packs/shelly-plug`](https://github.com/laupernoe/dumbmonit/tree/main/packs/shelly-plug)。
该格式接受的所有内容都记录在[参考文档](format.md)中。
