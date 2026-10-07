// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { say } from "../../../core/lang";
import type { Lang } from "../../../core/lang";
import type { WireApi } from "../../../core/commands";
import type { Draft, Line } from "./draft";
import { figureIn, idOf } from "./draft";

// The two readings of a draft that let somebody check it before they
// send it: the `config.toml` table this form would have written, and
// the call the endpoint would make with every override already
// applied.
//
// Both are drawn from the draft alone. Neither is a second
// authority: a figure the draft leaves absent is left out of the file
// rather than filled in with the city's default, because the city is
// where that default lives and a preview that printed it would be
// the place the two could disagree.

// The path a dialect's call goes to, which is also what the preview
// shows: the base URL is the root a provider states, never the full
// path, and this is the one place that knows what each wire adds to
// it.
function callPath(api: WireApi): string {
  switch (api) {
    case "chat":
      return "/chat/completions";
    case "responses":
      return "/responses";
    case "messages":
      return "/messages";
  }
}

function isRecord(held: unknown): held is Record<string, unknown> {
  return typeof held === "object" && held !== null && !Array.isArray(held);
}

// What a person typed into an override's value box, read as the JSON
// value it spells, for the preview alone. **The city is the
// authority**: the frame carries the text, and
// `gateway::EndpointTuning` reads it by the same rule - numbers, the
// two booleans and null as themselves, everything else as the string
// it looks like, which is what `/reasoning/effort` wants.
function jsonValue(text: string): unknown {
  const trimmed = text.trim();
  if (trimmed === "true") return true;
  if (trimmed === "false") return false;
  if (trimmed === "null") return null;
  if (/^-?[0-9]+(\.[0-9]+)?$/.test(trimmed)) return Number.parseFloat(trimmed);
  return trimmed;
}

// One JSON pointer written into a body, returning the body it
// produced. A segment whose parent is not an object replaces that
// parent: the preview's job is to show where an override lands, and
// a pointer into a number lands by making the number an object.
function withPointer(
  node: Record<string, unknown>,
  path: readonly string[],
  value: unknown,
): Record<string, unknown> {
  const [head, ...rest] = path;
  if (head === undefined) return node;
  if (rest.length === 0) return { ...node, [head]: value };
  const held = node[head];
  return { ...node, [head]: withPointer(isRecord(held) ? held : {}, rest, value) };
}

function pointerPath(pointer: string): readonly string[] {
  return pointer
    .split("/")
    .map((segment) => segment.trim())
    .filter((segment) => segment !== "");
}

function pairs(rows: readonly Line[]): readonly Line[] {
  return rows.filter((row) => row.name.trim() !== "");
}

// The `config.toml` table this form would have written, so somebody
// who keeps one can compare the two by eye.
//
// `name` appears only when somebody typed one. A display name nobody
// stated is the id, and that fallback belongs to the city
// (`AttachedEndpoint::label`); printing it here would put the same
// rule in a second place and promise a file this form does not send.
function configToml(draft: Draft, reference: string): string {
  const id = idOf(draft);
  const lines = [`[model_providers.${id === "" ? "<id>" : id}]`];
  if (draft.label.kind === "typed") {
    lines.push(`name = ${JSON.stringify(draft.label.value.trim())}`);
  }
  lines.push(
    `base_url = ${JSON.stringify(draft.baseUrl.trim())}`,
    `wire_api = ${JSON.stringify(draft.wireApi)}`,
    `env_key = ${JSON.stringify(reference)}`,
  );
  // A box left empty is left out, as the attach command leaves it
  // out, so the file and the command both leave the figure to the city.
  const figures = [
    ["request_max_retries", draft.requestRetries],
    ["stream_idle_timeout_ms", draft.streamIdleMs],
    ["timeout_ms", draft.timeoutMs],
  ] as const;
  for (const [name, text] of figures) {
    const figure = figureIn(text);
    if (figure !== null) lines.push(`${name} = ${String(figure)}`);
  }
  const headers = pairs(draft.headers);
  if (headers.length > 0) {
    const written = headers.map(
      (row) => `${JSON.stringify(row.name.trim())} = ${JSON.stringify(row.value)}`,
    );
    lines.push(`http_headers = { ${written.join(", ")} }`);
  }
  return lines.join("\n");
}

// The call this endpoint would make, headers and body, with every
// override already applied. One minimal conversation, because the
// question it answers is where a pointer lands rather than what a
// model would say.
function requestPreview(draft: Draft, reference: string, model: string): string {
  const url = `${draft.baseUrl.trim()}${callPath(draft.wireApi)}`;
  const header =
    draft.wireApi === "messages"
      ? `x-api-key: ${reference}`
      : `authorization: Bearer ${reference}`;
  const written = [
    `POST ${url === callPath(draft.wireApi) ? "<base_url>" : url}`,
    header,
    "content-type: application/json",
    ...pairs(draft.headers).map((row) => `${row.name.trim().toLowerCase()}: ${row.value}`),
  ];
  let body: Record<string, unknown> = {
    model,
    messages: [{ role: "user", content: "hello" }],
    stream: true,
  };
  for (const row of pairs(draft.overrides)) {
    body = withPointer(body, pointerPath(row.name), jsonValue(row.value));
  }
  return `${written.join("\n")}\n\n${JSON.stringify(body, null, 2)}`;
}

// One reading as a look draws it: the word on its disclosure and the
// text under it.
export interface ReadingLook {
  readonly key: "toml" | "request";
  readonly title: string;
  readonly text: string;
  // The file is prose-shaped and wraps; the call keeps its lines, so a
  // header or a JSON line is never broken in the middle.
  readonly wraps: boolean;
}

export function readingsOf(draft: Draft, reference: string, model: string, lang: Lang): readonly ReadingLook[] {
  return [
    { key: "toml", title: say(lang, "setup_preview_toml"), text: configToml(draft, reference), wraps: true },
    { key: "request", title: say(lang, "setup_preview_request"), text: requestPreview(draft, reference, model), wraps: false },
  ];
}
