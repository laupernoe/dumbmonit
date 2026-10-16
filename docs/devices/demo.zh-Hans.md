# 演示监控项 {#demo-device}

一个虚构的监控项,产生测量数据,用于在没有硬件的情况下体验这个工具。
不需要任何硬件。

## 设置 {#setup}

1. 随意起一个名字并确认。
2. 虚假的测量数据会立刻出现,足以浏览整个界面。

地址是空闲的(占位符写的是 `demo`),没有凭据,也没有任何选项。
该演示监控项会产生一条 `cpu_usage_percent` 序列,因此“CPU 过高”
和“CPU 异常”规则对它同样适用,它也可以作为
[依赖链](../alerting/index.md#dependency-suppression)中的父监控项或子监控项,
用来观察抑制机制的实际效果。

如果想要在没有专用硬件的情况下获得真实的测量数据,可以把一台 SNMP
监控项指向任意一台运行 `snmpd` 的机器:参见
[无硬件测试](../install/docker.md#testing-without-hardware)。
