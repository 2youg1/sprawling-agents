// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { Schema } from "effect";

import type { Key } from "../../core/lang";
import type { McpServer } from "../../wire";
import { ServerLabel } from "../../wire";

const labelled = Schema.is(ServerLabel);

export type Transport = "stdio" | "http" | "sse";

export const SPELLED: Record<Transport, Key> = {
  stdio: "mcp_kind_stdio",
  http: "mcp_kind_http",
  sse: "mcp_kind_sse",
};

export interface Pair {
  readonly name: string;
  readonly value: string;
}

export interface Draft {
  readonly label: string;
  readonly transport: Transport;
  readonly command: string;
  readonly env: readonly Pair[];
  readonly url: string;
  readonly headers: readonly Pair[];
}

export const EMPTY: Draft = {
  label: "",
  transport: "stdio",
  command: "",
  env: [],
  url: "",
  headers: [],
};

export type Blocker =
  | "label_missing"
  | "label_grammar"
  | "label_taken"
  | "command_missing"
  | "url_missing"
  | "pair_nameless";

export const WHY: Record<Blocker, Key> = {
  label_missing: "mcp_why_label",
  label_grammar: "mcp_why_grammar",
  label_taken: "mcp_why_taken",
  command_missing: "mcp_why_command",
  url_missing: "mcp_why_url",
  pair_nameless: "mcp_why_pair_name",
};

export type Encoded =
  | { readonly kind: "ready"; readonly server: McpServer }
  | { readonly kind: "blocked"; readonly blocker: Blocker };

export function written(pairs: readonly Pair[]): readonly Pair[] {
  return pairs.filter((pair) => pair.name.trim() !== "");
}

export function encode(draft: Draft, taken: readonly string[]): Encoded {
  const label = draft.label.trim();
  if (label === "") return { kind: "blocked", blocker: "label_missing" };
  if (!labelled(label)) return { kind: "blocked", blocker: "label_grammar" };
  if (taken.includes(label)) return { kind: "blocked", blocker: "label_taken" };
  switch (draft.transport) {
    case "stdio":
      return started(label, draft);
    case "http":
    case "sse":
      return reached(label, draft, draft.transport);
  }
}

function carried(pairs: readonly Pair[]): readonly (readonly [string, string])[] {
  return written(pairs).map((pair) => [pair.name.trim(), pair.value.trim()] as const);
}

function nameless(pairs: readonly Pair[]): boolean {
  return pairs.some((pair) => pair.name.trim() === "" && pair.value.trim() !== "");
}
function started(label: string, draft: Draft): Encoded {
  const words = draft.command
    .trim()
    .split(/\s+/)
    .filter((word) => word !== "");
  const program = words[0];
  if (program === undefined) return { kind: "blocked", blocker: "command_missing" };
  if (nameless(draft.env)) return { kind: "blocked", blocker: "pair_nameless" };
  return {
    kind: "ready",
    server: {
      label: ServerLabel.make(label),
      transport: {
        stdio: { command: program, args: words.slice(1), env: carried(draft.env) },
      },
    },
  };
}

function reached(label: string, draft: Draft, over: "http" | "sse"): Encoded {
  const url = draft.url.trim();
  if (url === "") return { kind: "blocked", blocker: "url_missing" };
  if (nameless(draft.headers)) return { kind: "blocked", blocker: "pair_nameless" };
  const headers = carried(draft.headers);
  return {
    kind: "ready",
    server: {
      label: ServerLabel.make(label),
      transport: over === "http" ? { http: { url, headers } } : { sse: { url, headers } },
    },
  };
}

export interface Intake {
  readonly taken: () => readonly string[];
  readonly offer: (server: McpServer) => boolean;
  readonly offerMany: (servers: readonly McpServer[]) => boolean;

  readonly why: () => string | null;
}
