// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Reading the `mcpServers` block a person copied out of Claude Code,
// Cursor or a project's `.mcp.json`.
//
// The block is other software's file format, so it is read with the
// same door every wire frame is read with: a Schema that answers
// `Either`, never a parse that throws. What comes back is drafts -
// `encode` alone decides which of them this city can be told about, so
// a fragment carrying an environment variable is refused with the same
// sentence the command door shows.

import { Either, Schema } from "effect";

import type { Draft, Pair, Transport } from "./draft";

// Every field is optional because every writer of this format leaves
// some of them out, and a block that states nothing useful is better
// read as a server with nothing in it than as unreadable text.
const Entry = Schema.Struct({
  type: Schema.optional(Schema.String),
  transport: Schema.optional(Schema.String),
  command: Schema.optional(Schema.String),
  args: Schema.optional(Schema.Array(Schema.String)),
  env: Schema.optional(Schema.Record({ key: Schema.String, value: Schema.String })),
  url: Schema.optional(Schema.String),
  headers: Schema.optional(Schema.Record({ key: Schema.String, value: Schema.String })),
});

type Entry = typeof Entry.Type;

const Servers = Schema.Record({ key: Schema.String, value: Entry });

const readWrapped = Schema.decodeUnknownEither(
  Schema.parseJson(Schema.Struct({ mcpServers: Servers })),
);
const readBare = Schema.decodeUnknownEither(Schema.parseJson(Servers));
// Whether the wrapper is there at all, asked before the block inside it
// is judged. Without this question a settings file holding one bad
// entry reads as a single server called `mcpServers`, because a block
// of blocks is itself a block of servers that state nothing.
const readKeys = Schema.decodeUnknownEither(
  Schema.parseJson(Schema.Record({ key: Schema.String, value: Schema.Unknown })),
);

export type Fragment =
  | { readonly kind: "servers"; readonly drafts: readonly Draft[] }
  | { readonly kind: "unreadable" };

// Both shapes people paste: the whole settings file with its
// `mcpServers` key, and the block alone.
export function readFragment(text: string): Fragment {
  const keys = readKeys(text);
  if (Either.isRight(keys) && Object.hasOwn(keys.right, "mcpServers")) {
    const block = readWrapped(text);
    return Either.isRight(block)
      ? { kind: "servers", drafts: drafted(block.right.mcpServers) }
      : { kind: "unreadable" };
  }
  const bare = readBare(text);
  return Either.isRight(bare)
    ? { kind: "servers", drafts: drafted(bare.right) }
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

// What the block says, and what its shape says when it says nothing.
// `streamable-http` and `http-stream` are the same transport under two
// spellings that both appear in the wild.
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
