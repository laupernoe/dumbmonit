# 更新 DumbMonit {#update-dumbmonit}

DumbMonit 会在有新版本时告诉你,并引导你完成更新。它永远不会
自行安装任何内容。

## 发现新版本 {#spotting-a-new-version}

服务器最多每天一次向 GitHub 的公开发布 API
(`api.github.com/repos/laupernoe/dumbmonit/releases`)询问最新发布版本,
并与你当前运行的版本进行比较。诸如 `0.1.0-alpha.7` 这样的预发布版本
会被正确比较:`alpha.10` 比 `alpha.9` 新,`0.1.0` 比任何
`0.1.0-alpha.N` 都新。草稿会被忽略,运行稳定版本的实例永远不会
被推荐使用预发布版本。

当存在更新的版本时,每一页的右下角以及**设置 → 关于**中都会出现
**有可用更新:vX**,其中**如何更新**会展开下面的步骤,
**查看更新内容**会重新打开*更新内容*窗口。**立即检查**会马上
再查一次(最多每分钟一次)。

!!! note "发送的内容"
    一次匿名的 `GET` 请求,带有 `DumbMonit/<version>` 的用户代理。
    不会发送关于你的实例、监控项或网络的任何信息,地址在代码中是固定的。

### 关闭检查 {#turning-the-check-off}

对于物理隔离的网络,可在**设置 → 关于**中关闭**检查新版本**
(管理员操作),或设置环境变量,它的优先级高于该开关:

```yaml
environment:
  DUMBMONIT_UPDATE_CHECK: "off"
```

同样的信息也可通过 `GET /api/update` 提供给脚本和 API 令牌使用。

## 更新步骤 {#updating}

1. **备份。** 从**设置 → 备份**下载一份备份,并保留一份
   `/data/secret.key` 的副本(参见[备份与恢复](backup.md))。没有
   这个密钥,已保存的密码和令牌将无法解密。
2. **拉取并重启**,在保存你的 `docker-compose.yml` 的目录中执行:

    ```bash
    docker compose pull && docker compose up -d
    ```

    你的数据保存在 `/data` 卷中,不会丢失;数据库迁移会在启动时运行。
3. **刷新页面。** **设置 → 关于**会显示新版本以及
   *已是最新版本*。

若要固定某个版本而不是跟随 `:latest`,请修改 Compose 文件中的镜像
标签(例如 `ghcr.io/laupernoe/dumbmonit:0.1.0-alpha.7`),然后执行
同样的命令。

若要回退,请在上一个镜像上恢复第 1 步中所做的备份。
