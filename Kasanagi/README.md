# Kasanagi

[简体中文](README.zh-CN.md) · [Project home](../README.md)

This directory reserves Kasanagi's future home in `sprawling-agents`. There is no implementation here yet.

## Where to start

For the current pre-alpha CLI, start at the [standalone project README](https://github.com/2youg1/kusanagi). Nothing in this directory builds or ships yet.

## Technology and direction

The existing [kusanagi](https://github.com/2youg1/kusanagi) project will be reworked and moved into this directory. It provides building blocks for chat software: encrypted, signed and hash-linked messages between agents on different machines, using a simple host or an S3-compatible bucket that is not trusted with message contents, authorship or conversation linkage.

The planned integration is an MCP server for remote collaboration between multiple sprawling instances. Until that migration lands, this README is only a placeholder; no code or package is built from this directory.
