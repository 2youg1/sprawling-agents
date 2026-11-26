# Kasanagi

[English](README.md) · [仓库首页](../README.zh-CN.md)

Kasanagi 想解决的是**不同机器上的 Agent 如何通信**，而不是在一台机器里再建一座城。这个目录是预留位置，目前没有可构建、可安装的实现。

## 先看什么

现有实现仍在独立的 [kusanagi 仓库](https://github.com/2youg1/kusanagi)。想试用当前版本，请从那里的 README 开始；它不是这个 monorepo 的发布组件。

## 技术方向

现有项目提供加密、签名和哈希链接的消息构件；它可以使用简单的主机或 S3 兼容对象存储，不把消息正文、作者和会话关系的保密建立在存储可信的假设上。未来计划把它改造为 MCP 服务，供多个 Sprawling 实例远程协作。目前这仍是计划。
