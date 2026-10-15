# 备份和恢复 {#backup-and-restore}

两种不同的东西，用于两种不同的事故。

| | 加密包 | 定时本地备份 |
|---|---|---|
| 它包含什么 | 配置：监控项及其凭证、规则、通道、状态页面、账户、令牌。 | 整个数据库，作为一个文件，加上 `secret.key`。 |
| 它去哪里 | 任何你放它的地方——它用你选择的密码加密。 | `/data/backups/`，在同一个卷内。 |
| 它能保护你免受什么 | 丢失的服务器、移到新硬件、全新安装。 | 你十分钟后后悔的更改、出错的升级。 |
| 它不包含什么 | 指标历史、警报状态和历史、审计日志、会话。 | 指标历史 (`vm/`)。 |

都不能替代整个卷的存档如果你想找回图表；那在这个页面的底部。

!!! danger "`/data/secret.key` 是解密你监控项凭证的钥匙"
    SNMP社区、Proxmox令牌、SMTP密码和心跳URL用来自实例密钥的密钥加密存储。在首次启动时，DumbMonit将其写入 `/data/secret.key` （或从 `DUMBMONIT_SECRET` 读取（如果你设置了）。

    **没有该文件的数据库副本恢复一个无法与任何内容通话的实例。** 服务器注意到并拒绝以显式消息启动，而不是在每个探针失败，但凭证消失，每个监控项必须再次输入。

    加密包是唯一的例外：其密钥用你的密码重新加密，所以它恢复到拥有自己的、全新密钥的全新实例上。

## 加密包 {#the-encrypted-bundle}

**设置→备份→下载包。** 选择至少16个字符的密码——三个单词就足够了——并确认。下来的文件是带有加密主体的纯JSON：

```json
{
  "format": "dumbmonit-backup",
  "version": 1,
  "created_at": "2026-09-22 19:04:11",
  "source_version": "0.1.0-alpha.3",
  "summary": { "targets": 12, "channels": 2, "rules": 41 },
  "kdf": { "algorithm": "argon2id", "salt": "…", "m_cost": 32768, "t_cost": 3, "p_cost": 1 },
  "cipher": "aes-256-gcm",
  "payload": "…"
}
```

标头保持可读，所以你可以在输入任何内容之前判断文件是什么。一切其他的——地址包括在内——在 `payload` 内，用从你的密码通过Argon2id派生的密钥用AES-256-GCM加密。**没有恢复**：密码丢失的包丢失。

!!! warning "该包持有该实例的每个凭证"
    任何拥有文件和密码的人都可以读取你监控的每个监控项的密码。将其保存在你保存密码的地方。

### 它包含什么 {#what-is-in-it}

| 部分 | 内容 |
|---|---|
| `targets` | 监控项：名称、类型、地址、间隔、选项、父级、中继代理和凭证。 |
| `rules` | 警报规则，包括你已调整或禁用的内置规则。 |
| `rule_overrides` | 每个监控项的阈值和排除。 |
| `channels` | 通知通道、其设置、其每个通道策略和其密钥。 |
| `notify_policy` | 全局通知策略（分组、每小时上限、抖动、公共URL）。 |
| `silences` | 维护窗口。 |
| `status_pages` | 公共状态页面和每个页面列出的服务。 |
| `incidents` | 事件和维护公告，带有其更新。 |
| `users` | 账户：用户名、显示名称、角色、SSO链接、启用或否。 |
| `agent_tokens` | 代理注册令牌，作为哈希。安装的代理在恢复后继续推送。 |
| `api_tokens` | API令牌，作为哈希。现有令牌继续工作。 |
| `push_monitors` | 心跳令牌，所以你的cron作业已经调用的URL保持相同。 |

监控项与**类型+地址**匹配，这也是表格的唯一密钥：导入相同包两次永远不会创建任何东西的第二个副本。

### 账户密码 {#account-passwords}

默认情况下账户导出**不**包括其密码哈希、TOTP密钥或恢复代码。恢复后的账户存在着正确的名称和角色，但没有人可以用它们登录直到管理员设置密码——或直到他们通过SSO登录，这不需要包中的任何内容。

当你移动实例而不是复制其配置时，打开**包括账户密码和2FA密钥**。Argon2哈希在一个可以旅行的文件中会被离线攻击，只要攻击者喜欢；默认是安全的。

### 它不包含什么 {#what-is-not-in-it}

指标历史（那生活在VictoriaMetrics中，不在数据库中）、警报状态和历史、异常基线、审计日志、打开的会话、代理的注册和最后见、Proxmox和Synology面板的探针缓存。它们是观察，不是设置：它们自己回来。

而且**不是实例密钥**。那是关键点：包将其密钥重新加密在你的密码下正是为了它可以恢复到从未见过 `secret.key` 的地方。

### 恢复到全新实例 {#restoring-onto-a-fresh-instance}

1. 启动新容器并在 `/setup` 创建第一个管理员账户，使用其日志中打印的设置代码。该账户是你的；恢复不会接触它。
2. **设置→备份→恢复包。** 选择文件，输入密码，并按**检查该备份**。
3. 阅读报告。它逐部分说明什么会**创建**，什么会**更新**因为它与这里已有的不同，什么已经相同——加上笔记用于它无法放置的任何内容，如指向包不包含的监控项的规则。
4. 按**真正恢复**。

恢复永远不会打破的三个规则：

- **没有东西永远被删除。** 恢复让包描述的存在；实例额外有的任何东西留下。
- **没有东西永远被复制。** 每项与其自然密钥匹配——监控项的类型+地址，规则的 `uid`，通道的名称，状态页面的slug。恢复相同包两次报告一切已相同并写入无。
- **现有账户不被触及。** 用户名已经在这里的账户被跳过，永不覆盖：文件不得能改变运行恢复的管理员的角色或密码。

干跑不是恢复的模拟，它*是*恢复，在最后被回滚的事务内运行。报告不能在应用会做什么上出错。

用更新的DumbMonit编写的包被拒绝，带一句说这个的句子——先升级，然后恢复。被修改的包或其密码错的包也被拒绝：AES-GCM不能区分两者，消息也不能。

### 从命令行 {#from-the-command-line}

两个路由都需要管理员**会话**，不是API令牌：令牌生活在脚本中，脚本无权导出实例的每个凭证。

```bash
# 登录
curl -s -c jar -X POST http://localhost:8080/api/auth/login \
  -H 'content-type: application/json' -H 'X-Requested-With: DumbMonit' \
  -d '{"username":"admin","password":"…"}'

# 导出
curl -s -b jar -X POST http://localhost:8080/api/backup \
  -H 'content-type: application/json' -H 'X-Requested-With: DumbMonit' \
  -d '{"passphrase":"three words are enough"}' -o backup.json

# 干跑，然后应用
jq -n --slurpfile b backup.json \
  '{bundle: $b[0], passphrase: "three words are enough", apply: false}' > restore.json
curl -s -b jar -X POST http://localhost:8080/api/backup/restore \
  -H 'content-type: application/json' -H 'X-Requested-With: DumbMonit' \
  -d @restore.json | jq
```

## 定时本地备份 {#scheduled-local-backups}

DumbMonit按计划写自己数据库的副本，默认**一天一次，保留最后七个**，进入 `/data/backups/`：

```
/data/backups/dumbmonit-20260922-030000.db
/data/backups/dumbmonit-20260922-030000.key
```

`.db` 文件用SQLite的 `VACUUM INTO` 生成：一个在线的、一致的副本在服务器继续运行时写入，不是一个可能捕捉数据库中途写的文件副本。`sqlite3` 如常打开它，没有要重放的日志。
`.key` 在其旁是 `secret.key` 的副本——一起，该对足以重建实例。

它们生活在与数据库相同的卷中，所以它们撤销一个错误，不是丢失的磁盘。如果你想要比那更多，将它们复制离开机器。

### 设置 {#settings}

| 变量 | 默认 | 角色 |
|---|---|---|
| `DUMBMONIT_BACKUP_ENABLED` | `1` | `0`、`false`、`no`或`off`停止写定时备份。 |
| `DUMBMONIT_BACKUP_DIR` | `<data dir>/backups` | 它们被写入的地方。指向第二个卷以在第一个卷丢失时生存。 |
| `DUMBMONIT_BACKUP_INTERVAL_HOURS` | `24` | 两个备份之间的小时数。第一个发生在启动后的一个间隔，不在启动时。 |
| `DUMBMONIT_BACKUP_KEEP` | `7` | 保留多少个。最旧的被删除，用它们的 `.key`。 |

目录中的没有其他东西被触及：只有命名为 `dumbmonit-<timestamp>.db` 及其 `.key` 的文件被轮转。

当实例密钥来自 `DUMBMONIT_SECRET` 时没有文件要复制，没有被写——改为在你的密码管理器中保留该值。备份部分说哪两种情况你在其中。

### 知道它运行 {#knowing-that-it-ran}

每个运行发布三个序列：

| 指标 | 含义 |
|---|---|
| `dumbmonit_instance_backup_last_success_seconds` | 最后**成功的**备份的Unix时间戳。失败的尝试永不刷新它。 |
| `dumbmonit_instance_backup_last_status` | 如果最后尝试成功为`1`，如果失败为`0`。 |
| `dumbmonit_instance_backup_size_bytes` | 最后写的文件的大小。 |

内置规则**DumbMonit备份未运行** (`instance_backup_missing`) 当最后成功超过两天前时触发——一个失误的夜晚可能是重启，两个是问题。序列仅在第一个备份被写后存在，所以一个关闭的调度实例永不被它烦扰。

**设置→备份**显示相同的东西不没有MetricsQL：最后运行及是否成功、保留的文件及其大小，以及**立即备份**立即写一个。

### 恢复一个 {#restoring-one}

停止堆栈，将文件放回数据库的名称下，并在其旁恢复密钥：

```bash
docker compose stop
docker run --rm -v dumbmonit-data:/data alpine sh -c '
  cp /data/backups/dumbmonit-20260922-030000.db /data/dumbmonit.db &&
  cp /data/backups/dumbmonit-20260922-030000.key /data/secret.key &&
  rm -f /data/dumbmonit.db-wal /data/dumbmonit.db-shm'
docker compose start
```

删除 `-wal` 和 `-shm` 文件很重要：它们属于你刚替换的数据库，SQLite会尝试将它们应用于新的。

要读备份不恢复它，将它复制出来并用 `sqlite3` 打开它：

```bash
docker run --rm -v dumbmonit-data:/data -v "$PWD:/out" alpine \
  cp /data/backups/dumbmonit-20260922-030000.db /out/
sqlite3 dumbmonit-20260922-030000.db 'SELECT name, kind, address FROM targets;'
```

该文件中的凭证不没有匹配的 `secret.key` 保持不可读。

## 存档整个卷 {#archiving-the-whole-volume}

保留图表的唯一方式。一个命名卷，`dumbmonit-data`，持有一切：数据库、密钥、本地备份和VictoriaMetrics数据。

```bash
docker compose stop
docker run --rm -v dumbmonit-data:/data -v "$PWD:/backup" alpine \
  tar czf /backup/dumbmonit-data.tgz -C /data .
docker compose start
```

要恢复，创建卷，以相同方式将存档提取到其中，然后 `docker compose up -d`。

如果空间重要，`--exclude=./vm` 保持存档小：数据库和密钥是设置，`vm/` 仅是历史。

## 升级前做什么 {#what-to-do-before-an-upgrade}

1. **设置→备份→立即备份**——三十秒，它包括密钥。
2. 下载包如果升级是个大的，或如果你在移到另一台机器。
3. `docker compose pull && docker compose up -d`。迁移在启动时运行。

如果出错，[恢复本地备份](#restoring-one)并启动之前的镜像标签。
