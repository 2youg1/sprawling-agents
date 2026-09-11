// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";

import { EMPTY, encode } from "./draft";
import { ServerLabel } from "../../wire";
import { readFragment } from "./fragment";

describe("a pasted mcpServers block", () => {
  test("is read whether the wrapper is there or not", () => {
    const inner = '{"docs":{"command":"npx","args":["-y","mcp-docs"]}}';
    expect(readFragment(`{"mcpServers":${inner}}`)).toEqual(readFragment(inner));
  });

  test("carries the command and its arguments as one line", () => {
    expect(readFragment('{"mcpServers":{"docs":{"command":"npx","args":["-y","mcp-docs"]}}}')).toEqual({
      kind: "servers",
      drafts: [
        {
          label: "docs",
          transport: "stdio",
          command: "npx -y mcp-docs",
          env: [],
          url: "",
          headers: [],
        },
      ],
    });
  });

  test("reads a url without a stated type as http, and keeps its headers", () => {
    expect(
      readFragment('{"mcpServers":{"apps":{"url":"https://example.test/mcp","headers":{"X-Key":"k"}}}}'),
    ).toEqual({
      kind: "servers",
      drafts: [
        {
          label: "apps",
          transport: "http",
          command: "",
          env: [],
          url: "https://example.test/mcp",
          headers: [{ name: "X-Key", value: "k" }],
        },
      ],
    });
  });

  test("answers unreadable rather than throwing", () => {
    expect(readFragment("{")).toEqual({ kind: "unreadable" });
    expect(readFragment('{"mcpServers":{"docs":{"args":"not a list"}}}')).toEqual({ kind: "unreadable" });
  });
});

describe("what today's wire can carry", () => {
  test("a command becomes a stdio server", () => {
    expect(encode({ ...EMPTY, label: "docs", command: "npx -y mcp-docs" }, [])).toEqual({
      kind: "ready",
      server: {
        label: ServerLabel.make("docs"),
        transport: { stdio: { command: "npx", args: ["-y", "mcp-docs"] } },
      },
    });
  });

  test("a url with one header becomes an http server", () => {
    expect(
      encode(
        {
          ...EMPTY,
          label: "apps",
          transport: "http",
          url: "https://example.test/mcp",
          headers: [{ name: "X-Key", value: "k" }, { name: "", value: "" }],
        },
        [],
      ),
    ).toEqual({
      kind: "ready",
      server: {
        label: ServerLabel.make("apps"),
        transport: { http: { url: "https://example.test/mcp", header: "X-Key: k" } },
      },
    });
  });

  test("the holes in the wire are refused rather than dropped", () => {
    const url = { ...EMPTY, label: "apps", transport: "http", url: "https://example.test/mcp" } as const;
    expect(encode({ ...EMPTY, label: "docs", command: "x", env: [{ name: "TOKEN", value: "t" }] }, [])).toEqual({
      kind: "blocked",
      blocker: "env_unsendable",
    });
    expect(
      encode({ ...url, headers: [{ name: "A", value: "1" }, { name: "B", value: "2" }] }, []),
    ).toEqual({ kind: "blocked", blocker: "headers_many" });
    expect(encode({ ...url, transport: "sse" }, [])).toEqual({ kind: "blocked", blocker: "transport_sse" });
  });

  test("a label is present, spellable and free before anything else is judged", () => {
    expect(encode(EMPTY, [])).toEqual({ kind: "blocked", blocker: "label_missing" });
    expect(encode({ ...EMPTY, label: "My_Server" }, [])).toEqual({ kind: "blocked", blocker: "label_grammar" });
    expect(encode({ ...EMPTY, label: "docs" }, ["docs"])).toEqual({ kind: "blocked", blocker: "label_taken" });
    expect(encode({ ...EMPTY, label: "docs" }, [])).toEqual({ kind: "blocked", blocker: "command_missing" });
  });
});
