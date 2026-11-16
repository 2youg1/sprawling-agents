// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { Result, Schema } from "effect";

import type { Draft, Pair, Transport } from "./draft";

const Entry = Schema.Struct({
  type: Schema.optional(Schema.String),
  transport: Schema.optional(Schema.String),
  command: Schema.optional(Schema.String),
  args: Schema.optional(Schema.Array(Schema.String)),
  env: Schema.optional(Schema.Record(Schema.String, Schema.String)),
  url: Schema.optional(Schema.String),
  headers: Schema.optional(Schema.Record(Schema.String, Schema.String)),
});

type Entry = typeof Entry.Type;

const Servers = Schema.Record(Schema.String, Entry);

const readWrapped = Schema.decodeUnknownResult(
  Schema.fromJsonString(Schema.Struct({ mcpServers: Servers })),
);
const readBare = Schema.decodeUnknownResult(Schema.fromJsonString(Servers));
const readKeys = Schema.decodeUnknownResult(
  Schema.fromJsonString(Schema.Record(Schema.String, Schema.Unknown)),
);

export type Fragment =
  { readonly kind: "servers"; readonly drafts: readonly Draft[] } | { readonly kind: "unreadable" };

export function readFragment(text: string): Fragment {
  const keys = readKeys(text);
  if (Result.isSuccess(keys) && Object.hasOwn(keys.success, "mcpServers")) {
    const block = readWrapped(text);
    return Result.isSuccess(block)
      ? { kind: "servers", drafts: drafted(block.success.mcpServers) }
      : { kind: "unreadable" };
  }
  const bare = readBare(text);
  return Result.isSuccess(bare)
    ? { kind: "servers", drafts: drafted(bare.success) }
    : { kind: "unreadable" };
}

function drafted(servers: Readonly<Record<string, Entry>>): readonly Draft[] {
  return Object.entries(servers).map(([label, entry]) => ({
    label,
    transport: transportOf(entry),
    command: [entry.command ?? "", ...(entry.args ?? [])].join(" ").trim(),
    env: paired(entry.env),
    url: entry.url ?? "",
    headers: paired(entry.headers),
  }));
}

function transportOf(entry: Entry): Transport {
  const stated = entry.type ?? entry.transport;
  if (stated === undefined) {
    return entry.url === undefined ? "stdio" : "http";
  }
  switch (stated) {
    case "sse":
      return "sse";
    case "http":
    case "streamable-http":
    case "http-stream":
      return "http";
    case "stdio":
      return "stdio";
    default:
      return entry.url === undefined ? "stdio" : "http";
  }
}

function paired(table: Readonly<Record<string, string>> | undefined): readonly Pair[] {
  if (table === undefined) return [];
  return Object.entries(table).map(([name, value]) => ({ name, value }));
}
