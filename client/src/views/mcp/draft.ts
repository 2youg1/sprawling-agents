// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What a person spelled about a server, and the one place that decides
// whether today's wire can carry it.
//
// A draft holds everything Claude Code lets somebody write - a command
// with environment variables, a url with several headers, a transport
// that may be `sse` - and `encode` answers with either the frame the
// city accepts or the single reason it cannot be sent. Every door on the
// page asks this function, so the reason a control is inert is written
// once and cannot differ between the command door, the url door and the
// pasted JSON.
//
// The wire now carries all three transports, an environment table and a
// header table, so every reason left here is a person's own mistake
// rather than a hole in the wire.

import type { Key } from "../../core/lang";
import type { McpServer } from "../../wire";
import { ServerLabel } from "../../wire";

// The label is the first segment of every tool name the server offers,
// so its grammar is the wire's, not this page's taste.
const LABEL = /^[a-z][a-z0-9-]*$/;

// The three a person picks between, and the three the wire spells.
export type Transport = "stdio" | "http" | "sse";

// How a transport is written on screen. The wire's own spellings are
// the industry's words, and this keeps them coming from the table like
// everything else a reader is handed.
export const SPELLED: Record<Transport, Key> = {
  stdio: "mcp_kind_stdio",
  http: "mcp_kind_http",
  sse: "mcp_kind_sse",
};

// One row of an environment table or a header table.
export interface Pair {
  readonly name: string;
  readonly value: string;
}

export interface Draft {
  readonly label: string;
  readonly transport: Transport;
  // The program and its arguments as one shell line, the way
  // `claude mcp add <name> -- <cmd> [args]` takes them.
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

/**
 * Why a draft cannot become a frame. Every one of these is something a
 * person can fix in the form in front of them; the page reads them from
 * one list, so the command door, the url door and a pasted block
 * explain one mistake identically.
 */
export type Blocker =
  | "label_missing"
  | "label_grammar"
  | "label_taken"
  | "command_missing"
  | "url_missing"
  | "pair_nameless";

// The sentence each blocker is shown as.
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

// The rows somebody actually wrote something in. An empty row is a row
// waiting to be typed in, not a value.
export function written(pairs: readonly Pair[]): readonly Pair[] {
  return pairs.filter((pair) => pair.name.trim() !== "");
}

// The one reading of a draft. `taken` is the labels this scope already
// reaches, because two servers under one label would collapse into one
// set of tool names.
export function encode(draft: Draft, taken: readonly string[]): Encoded {
  const label = draft.label.trim();
  if (label === "") return { kind: "blocked", blocker: "label_missing" };
  if (!LABEL.test(label)) return { kind: "blocked", blocker: "label_grammar" };
  if (taken.includes(label)) return { kind: "blocked", blocker: "label_taken" };
  switch (draft.transport) {
    case "stdio":
      return started(label, draft);
    case "http":
    case "sse":
      return reached(label, draft, draft.transport);
  }
}

// The wire carries a pair as a two-element list, name before value, in
// the order the table was filled in.
function carried(pairs: readonly Pair[]): readonly (readonly [string, string])[] {
  return written(pairs).map((pair) => [pair.name.trim(), pair.value.trim()] as const);
}

// A row with a value and no name is a row somebody started and did not
// finish. It is refused rather than dropped, because dropping it would
// leave a server quietly missing the credential they typed.
function nameless(pairs: readonly Pair[]): boolean {
  return pairs.some((pair) => pair.name.trim() === "" && pair.value.trim() !== "");
}

function started(label: string, draft: Draft): Encoded {
  const words = draft.command.trim().split(/\s+/).filter((word) => word !== "");
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

/**
 * Where a new server is handed in: what this scope already reaches, the
 * send itself, and - when the scope a person chose has no field on the
 * wire - the sentence that says why nothing can be sent at all.
 *
 * The doors take this rather than an address so that a door knows
 * nothing about scopes, and the page stays the single authority on which
 * scope a frame is written into.
 */
export interface Intake {
  readonly taken: () => readonly string[];
  readonly offer: (server: McpServer) => boolean;
  // Already in the person's language, or null when the scope is sendable.
  readonly why: () => string | null;
}
