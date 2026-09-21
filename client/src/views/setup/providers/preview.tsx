// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The two readings of a draft that let somebody check it before they
// send it: the `config.toml` table this form would have written, and
// the call the endpoint would make with every override already applied.
//
// Both are drawn from the draft alone. Neither is a second authority:
// a figure the draft leaves absent is left out of the file rather than
// filled in with the city's default, because the city is where that
// default lives and a preview that printed it would be the place the
// two could disagree.

import { referenceFor, referenceText } from "../../../core/enrol";
import type { Pair, WireApi } from "../../../core/commands";
import { useSay } from "../../../ui";
import { idOf } from "./draft";
import type { Draft } from "./draft";

// The path a dialect's call goes to, which is also what the preview
// shows: the base URL is the root a provider states, never the full
// path, and this is the one place that knows what each wire adds to it.
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
// value it spells, for the preview alone. **The city is the authority**:
// the frame carries the text, and `gateway::EndpointTuning` reads it by
// the same rule - numbers, the two booleans and null as themselves,
// everything else as the string it looks like, which is what
// `/reasoning/effort` wants.
function jsonValue(text: string): unknown {
  const trimmed = text.trim();
  if (trimmed === "true") return true;
  if (trimmed === "false") return false;
  if (trimmed === "null") return null;
  if (/^-?[0-9]+(\.[0-9]+)?$/.test(trimmed)) return Number.parseFloat(trimmed);
  return trimmed;
}

// One JSON pointer written into a body, returning the body it produced.
// A segment whose parent is not an object replaces that parent: the
// preview's job is to show where an override lands, and a pointer into
// a number lands by making the number an object.
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

function pairs(rows: readonly Pair[]): readonly Pair[] {
  return rows.filter((row) => row.name.trim() !== "");
}

// The reference an id derives, shown before anything is enrolled so a
// person can see what the form is about to file their key under.
export function referenceOf(id: string): string {
  return referenceText(referenceFor(id === "" ? "<id>" : id));
}

// The `config.toml` table this form would have written, so somebody who
// keeps one can compare the two by eye.
//
// `name` appears only when somebody typed one. A display name nobody
// stated is the id, and that fallback belongs to the city
// (`AttachedEndpoint::label`); printing it here would put the same rule
// in a second place and promise a file this form does not send.
export function configToml(draft: Draft, reference: string): string {
  const id = idOf(draft);
  const lines = [`[model_providers.${id === "" ? "<id>" : id}]`];
  if (draft.label.kind === "typed") {
    lines.push(`name = ${JSON.stringify(draft.label.value.trim())}`);
  }
  lines.push(
    `base_url = ${JSON.stringify(draft.baseUrl.trim())}`,
    `wire_api = ${JSON.stringify(draft.wireApi)}`,
    `env_key = ${JSON.stringify(reference)}`,
    `request_max_retries = ${draft.requestRetries.trim()}`,
    `stream_idle_timeout_ms = ${draft.streamIdleMs.trim()}`,
    `timeout_ms = ${draft.timeoutMs.trim()}`,
  );
  const headers = pairs(draft.headers);
  if (headers.length > 0) {
    const written = headers.map((row) => `${JSON.stringify(row.name.trim())} = ${JSON.stringify(row.value)}`);
    lines.push(`http_headers = { ${written.join(", ")} }`);
  }
  return lines.join("\n");
}

// The call this endpoint would make, headers and body, with every
// override already applied. One minimal conversation, because the
// question it answers is where a pointer lands rather than what a model
// would say.
export function requestPreview(draft: Draft, reference: string, model: string): string {
  const url = `${draft.baseUrl.trim()}${callPath(draft.wireApi)}`;
  const header =
    draft.wireApi === "messages" ? `x-api-key: ${reference}` : `authorization: Bearer ${reference}`;
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

// Both readings, folded away. They sit at the foot of the form because
// they answer a question somebody asks about a draft they have already
// finished writing.
export function Previews(props: {
  readonly draft: Draft;
  // What the key is filed under, which the form holds because only the
  // form knows whether one has been enrolled yet.
  readonly reference: string;
  readonly model: string;
}) {
  const say = useSay();
  return (
    <>
      <details class="rounded-card bg-chrome px-base py-snug text-note">
        <summary class="cursor-pointer text-text-quiet">{say("setup_preview_toml")}</summary>
        <pre class="mt-snug overflow-auto whitespace-pre-wrap font-mono text-text-faint">
          {configToml(props.draft, props.reference)}
        </pre>
      </details>
      <details class="rounded-card bg-chrome px-base py-snug text-note">
        <summary class="cursor-pointer text-text-quiet">{say("setup_preview_request")}</summary>
        <pre class="mt-snug overflow-auto whitespace-pre-wrap font-mono text-text-faint">
          {requestPreview(props.draft, props.reference, props.model)}
        </pre>
      </details>
    </>
  );
}
