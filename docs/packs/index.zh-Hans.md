# 集成包 {#integration-packs}

集成包可以在不发布新版本的情况下为 DumbMonit 添加一种设备类型。
它是单个 YAML 文件,说明从哪里读取(设备上的某个 HTTP API 或
Prometheus `/metrics` 页面)、提取什么内容,以及随之附带哪些告警
规则。安装后,该包对应的类型会出现在**添加设备**中,与内置类型
并列,其设备会像任何其他设备一样被轮询、绘制图表和发出告警。

集成包是声明式的:没有代码,没有脚本。DumbMonit 会自己执行每个
请求,遵循包本身无法更改的规则。

## 集成包能做和不能做的事 {#what-a-pack-can-and-cannot-do}

| 集成包能做的 | 集成包不能做的 |
|---|---|
| 调用设备自身地址上的 `GET` 或 `POST` 路径 | 调用任何其他主机、跟随指向另一个主机的重定向,或使用绝对 URL |
| 读取 JSON(JSONPath)、纯文本(正则表达式)或 Prometheus 文本格式 | 运行代码、shell 命令,或超出 `{{option.x}}` 和 `{{credential.x}}` 的模板 |
| 以请求头或 basic/bearer 认证方式发送设备的凭据 | 把凭据放进 URL 或请求体中 |
| 声明在设备表单中显示的选项(一个开关通道、一个阈值) | 写入超出自身 `dumbmonit_<pack>_` 前缀的指标 |
| 为自己的指标附带告警规则 | 更改内置设备类型或内置规则 |
| 以内置格式携带 SNMP 配置档案 | 每次响应读取超过 4 MiB、每次轮询超过 16 次请求,或每次轮询产生超过 5000 条序列 |

回环地址和链路本地地址(`127.0.0.1`、`169.254.x.x`)会被拒绝,
除非设备的**允许回环和链路本地目标**选项处于开启状态,这与 HTTP
检查完全一致。私有局域网地址始终被允许。

## 参考集成包 {#reference-packs}

仓库在
[`packs/`](https://github.com/laupernoe/dumbmonit/tree/main/packs)
下附带了三个集成包,各自带有测试样例和预期输出:

| 集成包 | 读取内容 | 规则 |
|---|---|---|
| `node-exporter` | Prometheus node_exporter 的一个精选子集(CPU、内存、文件系统、磁盘、网络) | 文件系统即将用尽、内存即将耗尽 |
| `shelly-plug` | Shelly Gen2+ 插座和继电器,`/rpc/Switch.GetStatus` | 过热 |
| `speedtest-tracker` | Speedtest Tracker v1 API 的最新结果 | 网速低于预期、测速失败 |

## 安装一个集成包 {#install-a-pack}

在界面中,以管理员身份:**设置 → 集成包**。粘贴该包的 YAML,或
上传其 `pack.yaml`,然后点击**安装该集成包**。未通过检查的包会
被拒绝,并给出需要修复的清单;通过检查的包会连同其说明(如果有)
一起被保存。同一个页面可以启用、禁用和卸载集成包。安装后的类型
会出现在添加设备时的**社区集成包**下,标注为*集成包*。

通过 HTTP API,你需要一个管理员会话,或一个带有 `write` 范围的
API 令牌(设置 → **API 与助手**)。

```bash
# 安装,或将同一个包更新到更新的版本
curl -H 'Authorization: Bearer dmt_…' -H 'Content-Type: application/yaml' \
  --data-binary @packs/shelly-plug/pack.yaml http://localhost:8080/api/packs

# 列出已安装的内容
curl -H 'Authorization: Bearer dmt_…' http://localhost:8080/api/packs
```

应答会说明发生了什么(`created`、`updated` 或 `unchanged`)、该包
产生的指标,以及新增了多少条告警规则。未通过验证的包会被拒绝,
并给出完整的问题清单,每行一条。

| 调用 | 作用 |
|---|---|
| `GET /api/packs` | 已安装的包:版本、类型、指标、规则、使用它的设备 |
| `GET /api/packs/{id}` | 单个包 |
| `POST /api/packs` | 安装或更新;请求体:YAML(`application/yaml`),或 JSON 中的 `{"yaml": "…"}` |
| `PUT /api/packs/{id}/disable` | 停止轮询该包的设备,保留其他一切 |
| `PUT /api/packs/{id}/enable` | 重新开始轮询它们 |
| `DELETE /api/packs/{id}` | 卸载,连同其规则;当仍有设备使用它时会被拒绝(`409`) |

使用会话 cookie 而不是令牌时,请在每个 `POST`、`PUT` 和 `DELETE`
中添加 `X-Requested-With: DumbMonit`,与[API](../reference/api.md)
其余部分的要求一致。

安装会立即生效:新类型无需重启即可在设备表单中使用。唯一的例外
是携带 SNMP 配置档案的集成包;此时应答会显示
`"restart_required": true`,因为 SNMP 目录是在启动时读取的。

### 更新与你的改动 {#updates-and-your-changes}

更新一个集成包(同样的 `id`,新的内容)会替换其提取逻辑和描述。
它的告警规则永远只会被*添加*:你修改、禁用或删除过的规则绝不会
被更新重写。

禁用一个集成包会撤回其设备类型,保留其他一切:设备、历史记录、
规则。它的设备不会再被轮询(每个都会显示配置错误),并且像任何
停止上报的设备一样,会在三分钟后触发*设备不可达*:如果打算暂时
禁用该包,也请同时暂停这些设备。

### 备份 {#backups}

已安装的集成包会随其 YAML 包含在[备份](../install/backup.md)中,
并会在使用它们的设备之前被先恢复。

## 在安装前检查一个集成包 {#check-a-pack-before-installing-it}

服务器二进制文件可以离线验证和测试集成包:

```bash
# 模式和一致性检查,无需服务器或数据库
docker run --rm -v "$PWD/packs:/packs:ro" ghcr.io/laupernoe/dumbmonit:latest \
  pack lint /packs/shelly-plug

# 通过真实的提取逻辑重放 fixtures/ 并与 expected.prom 对比
docker run --rm -v "$PWD/packs:/packs:ro" ghcr.io/laupernoe/dumbmonit:latest \
  pack test /packs/shelly-plug
```

`pack test --update` 会根据测试样例重写 `expected.prom`(请以
读写方式挂载该目录,并以 `--user "$(id -u)"` 运行,这样文件所有者
仍是你自己)。

接下来:在大约十分钟内[编写你的第一个集成包](first-pack.md),
或阅读[格式参考](format.md)。
