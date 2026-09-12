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

describe("what the wire carries", () => {
  test("a command carries its arguments and its environment", () => {
    expect(
      encode(
        {
          ...EMPTY,
          label: "docs",
          command: "npx -y mcp-docs",
          env: [{ name: "TOKEN", value: "secret:mcp/docs" }, { name: "", value: "" }],
        },
        [],
      ),
    ).toEqual({
      kind: "ready",
      server: {
        label: ServerLabel.make("docs"),
        transport: {
          stdio: {
            command: "npx",
            args: ["-y", "mcp-docs"],
            env: [["TOKEN", "secret:mcp/docs"]],
          },
        },
      },
    });
  });

  test("a url carries every header, and says which way it answers", () => {
    const url = {
      ...EMPTY,
      label: "apps",
      url: "https://example.test/mcp",
      headers: [{ name: "X-Key", value: "k" }, { name: "X-Account", value: "acme" }],
    } as const;
    const headers = [["X-Key", "k"], ["X-Account", "acme"]] as const;
    expect(encode({ ...url, transport: "http" }, [])).toEqual({
      kind: "ready",
      server: {
        label: ServerLabel.make("apps"),
        transport: { http: { url: "https://example.test/mcp", headers } },
      },
    });
    expect(encode({ ...url, transport: "sse" }, [])).toEqual({
      kind: "ready",
      server: {
        label: ServerLabel.make("apps"),
        transport: { sse: { url: "https://example.test/mcp", headers } },
      },
    });
  });

  test("a value typed beside no name is refused rather than dropped", () => {
    expect(
      encode({ ...EMPTY, label: "docs", command: "x", env: [{ name: " ", value: "t" }] }, []),
    ).toEqual({ kind: "blocked", blocker: "pair_nameless" });
  });

  test("a label is present, spellable and free before anything else is judged", () => {
    expect(encode(EMPTY, [])).toEqual({ kind: "blocked", blocker: "label_missing" });
    expect(encode({ ...EMPTY, label: "My_Server" }, [])).toEqual({ kind: "blocked", blocker: "label_grammar" });
    expect(encode({ ...EMPTY, label: "docs" }, ["docs"])).toEqual({ kind: "blocked", blocker: "label_taken" });
    expect(encode({ ...EMPTY, label: "docs" }, [])).toEqual({ kind: "blocked", blocker: "command_missing" });
  });
});
