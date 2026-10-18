# 路线图 {#roadmap}

已发布、进行中和计划中的内容都记录在公开的项目看板上:
[DumbMonit Roadmap](https://github.com/users/laupernoe/projects/2)。
优先级和时间可能调整;该看板是一份动态计划,而不是承诺。

- **已发布的变更**见
  [变更日志](https://github.com/laupernoe/dumbmonit/blob/main/CHANGELOG.md)
  以及每个 [GitHub 发行版](https://github.com/laupernoe/dumbmonit/releases)的说明。
- **想法和缺陷**请提交到
  [问题跟踪器](https://github.com/laupernoe/dumbmonit/issues)。在为较大的改动编写代码之前,
  请先开一个 issue。
- **安全修复**会在发行说明和 GitHub 安全公告中公布,并附有 CVE 或 GHSA 编号。

DumbMonit 目前尚未到达 1.0:在宣布稳定版本线之前,API 和数据模型会持续演进。
如需参与,请参阅
[贡献指南](https://github.com/laupernoe/dumbmonit/blob/main/CONTRIBUTING.md)
和
[治理说明](https://github.com/laupernoe/dumbmonit/blob/main/GOVERNANCE.md)。

## 未来十二个月的方向 {#direction-for-the-next-twelve-months}

这些是意向而不是承诺,大致按顺序排列。

- **迈向 1.0。** 确定 HTTP API 和数据库架构,以便宣布稳定版本线;在此
  之前,次要版本仍可能更改它们,升级说明会写在变更日志中。
- **更多有权限的人。** 仓库、镜像仓库、文档托管和域名各增加第二位管理员
  (参见[治理说明](https://github.com/laupernoe/dumbmonit/blob/main/GOVERNANCE.md))。
- **供应链。** 可复现的发布构建([当前状态](reproducible-builds.md))和
  已签名的版本标签。
- **arm64 镜像。** 服务器镜像目前仅有 amd64;arm64 将随原生构建运行器
  一起回归。
- **集成与配置包。** 更多设备和服务集成以及 SNMP 配置文件,由贡献和
  需求驱动。
- **文档。** 让界面和文档的九种语言与英文源保持同步。

## DumbMonit 不打算做的事 {#what-dumbmonit-does-not-intend-to-do}

- 成为日志、链路追踪或 APM 平台:它监控的是设备和服务。
- 提供付费版本或托管服务:它保持 100% 开源。
- 向任何地方发送关于你的实例的遥测数据。
- 面向成千上万台设备的规模:它为家庭实验室和小型团队而构建,运行在
  一个容器上。
